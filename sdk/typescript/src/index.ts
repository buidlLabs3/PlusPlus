/// ++ SDK — TypeScript client for the RGB++ DEX.
///
/// Features:
/// - Custom error classes for different failure modes
/// - Retry with exponential backoff
/// - Configurable request timeouts
/// - WebSocket event stream subscription
/// - Robust HTTP error handling (non-JSON responses, network failures)

import * as crypto from 'crypto';
import {
  PlusPlusConfig,
  CellType,
  Offer,
  OfferEnvelope,
  SwapAcceptance,
  Route,
  SwapResult,
  WsEvent,
  WsEventMessage,
  ApiResponse,
} from './types';

export * from './types';

// ---------------------------------------------------------------------------
// Error classes
// ---------------------------------------------------------------------------

/** Base error for all SDK errors */
export class PlusPlusError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'PlusPlusError';
  }
}

/** Network/HTTP error */
export class NetworkError extends PlusPlusError {
  public readonly statusCode?: number;
  public readonly responseText?: string;

  constructor(message: string, statusCode?: number, responseText?: string) {
    super(message);
    this.name = 'NetworkError';
    this.statusCode = statusCode;
    this.responseText = responseText;
  }
}

/** Request timed out */
export class TimeoutError extends PlusPlusError {
  constructor(message: string) {
    super(message);
    this.name = 'TimeoutError';
  }
}

/** All retries exhausted */
export class RetriesExhaustedError extends PlusPlusError {
  public readonly attempts: number;
  public readonly lastError: Error;

  constructor(attempts: number, lastError: Error) {
    super(`Retries exhausted after ${attempts} attempts: ${lastError.message}`);
    this.name = 'RetriesExhaustedError';
    this.attempts = attempts;
    this.lastError = lastError;
  }
}

/** WebSocket connection error */
export class WebSocketError extends PlusPlusError {
  constructor(message: string) {
    super(message);
    this.name = 'WebSocketError';
  }
}

// ---------------------------------------------------------------------------
// Hash helpers
// ---------------------------------------------------------------------------

function blake2b256(data: Buffer): Buffer {
  return crypto.createHash('blake2b-256').update(data).digest();
}

function hexToBuffer(hex: string): Buffer {
  return Buffer.from(hex, 'hex');
}

function bufferToHex(buf: Buffer): string {
  return buf.toString('hex');
}

// ---------------------------------------------------------------------------
// Retry helper
// ---------------------------------------------------------------------------

/** Sleep for the given number of milliseconds */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Execute an async function with retry and exponential backoff */
async function withRetry<T>(
  fn: () => Promise<T>,
  maxRetries: number,
  baseDelay: number,
  label: string
): Promise<T> {
  let lastError: Error | null = null;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await fn();
    } catch (err) {
      lastError = err instanceof Error ? err : new Error(String(err));

      // Don't retry client errors (4xx) except 429
      if (err instanceof NetworkError && err.statusCode) {
        if (err.statusCode >= 400 && err.statusCode < 500 && err.statusCode !== 429) {
          throw err;
        }
      }

      if (attempt < maxRetries) {
        const delay = baseDelay * Math.pow(2, attempt - 1) + Math.random() * 100;
        console.warn(
          `[++] ${label}: attempt ${attempt} failed, retrying in ${Math.round(delay)}ms...`
        );
        await sleep(delay);
      }
    }
  }

  throw new RetriesExhaustedError(maxRetries, lastError!);
}

// ---------------------------------------------------------------------------
// HTTP fetch wrapper with timeout and error handling
// ---------------------------------------------------------------------------

/** Fetch with timeout, retry, and proper error handling */
async function fetchWithTimeout(
  url: string,
  options: RequestInit & { timeout?: number; maxRetries?: number; retryBaseDelay?: number } = {}
): Promise<unknown> {
  const {
    timeout = 30000,
    maxRetries = 3,
    retryBaseDelay = 500,
    ...fetchOptions
  } = options;

  return withRetry(
    async () => {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), timeout);

      try {
        const response = await fetch(url, {
          ...fetchOptions,
          signal: controller.signal,
        });

        clearTimeout(timeoutId);

        // Handle non-OK responses
        if (!response.ok) {
          let body = '';
          try {
            body = await response.text();
          } catch {
            body = '<unreadable>';
          }

          // Try to parse error from API response format
          let errorMsg = `HTTP ${response.status}: ${response.statusText}`;
          try {
            const parsed = JSON.parse(body) as ApiResponse<unknown>;
            if (parsed.error) {
              errorMsg = parsed.error;
            }
          } catch {
            // Not JSON, use raw body
            if (body.length > 0 && body.length < 200) {
              errorMsg = `HTTP ${response.status}: ${body}`;
            }
          }

          throw new NetworkError(errorMsg, response.status, body);
        }

        // Handle empty responses
        const text = await response.text();
        if (!text || text.trim() === '') {
          return null;
        }

        // Parse JSON
        try {
          return JSON.parse(text);
        } catch {
          throw new NetworkError(
            `Failed to parse response as JSON: ${text.substring(0, 100)}`,
            response.status,
            text
          );
        }
      } catch (err) {
        clearTimeout(timeoutId);

        // Re-throw SDK errors directly
        if (err instanceof NetworkError) {
          throw err;
        }

        // Handle fetch-specific errors
        if (err instanceof DOMException && err.name === 'AbortError') {
          throw new TimeoutError(`Request to ${url} timed out after ${timeout}ms`);
        }

        if (err instanceof TypeError && err.message.includes('fetch')) {
          throw new NetworkError(`Network error: ${err.message}`);
        }

        throw new NetworkError(
          err instanceof Error ? err.message : String(err)
        );
      }
    },
    maxRetries,
    retryBaseDelay,
    `fetch ${url}`
  );
}

// ---------------------------------------------------------------------------
// Offer operations
// ---------------------------------------------------------------------------

interface OfferPayload {
  sellType: CellType;
  sellAmount: number;
  buyType: CellType;
  buyAmount: number;
  sellerLockHash: string;
  expiry: number;
}

/** Compute offer ID from payload (hash of unsigned payload) */
function computeOfferId(payload: OfferPayload): string {
  const json = JSON.stringify(payload);
  const hash = blake2b256(Buffer.from(json));
  return bufferToHex(hash);
}

/** Create a new offer payload (ready for signing) */
export function createOffer(params: {
  sellType: CellType;
  sellAmount: number;
  buyType: CellType;
  buyAmount: number;
  sellerLockHash: string;
  expiry: number;
}): { payload: OfferPayload; offerId: string } {
  const payload: OfferPayload = {
    sellType: params.sellType,
    sellAmount: params.sellAmount,
    buyType: params.buyType,
    buyAmount: params.buyAmount,
    sellerLockHash: params.sellerLockHash,
    expiry: params.expiry,
  };
  return { payload, offerId: computeOfferId(payload) };
}

/** Sign an offer payload and build the full Offer */
export function signOffer(payload: OfferPayload, signature: string): Offer {
  return {
    ...payload,
    signature,
  };
}

/** Wrap an offer in an envelope */
export function envelope(offer: Offer): OfferEnvelope {
  const payload: OfferPayload = {
    sellType: offer.sellType,
    sellAmount: offer.sellAmount,
    buyType: offer.buyType,
    buyAmount: offer.buyAmount,
    sellerLockHash: offer.sellerLockHash,
    expiry: offer.expiry,
  };
  return { offer, offerId: computeOfferId(payload) };
}

/** Verify an offer (basic checks — signature verification requires CKB lock script) */
export function verifyOffer(offer: Offer): boolean {
  if (!offer.signature || offer.signature.length === 0) return false;
  if (offer.sellAmount <= 0 || offer.buyAmount <= 0) return false;
  if (!offer.sellerLockHash || offer.sellerLockHash.length !== 64) return false;
  return true;
}

/** Check if an offer has expired */
export function isExpired(offer: Offer, currentBlock: number): boolean {
  return currentBlock > offer.expiry;
}

// ---------------------------------------------------------------------------
// Swap operations
// ---------------------------------------------------------------------------

/** Accept an offer as a buyer */
export function acceptOffer(
  offerId: string,
  buyerLockHash: string,
  amount: number,
  signature: string
): SwapAcceptance {
  return { offerId, buyerLockHash, amount, signature };
}

/** Validate an acceptance against its offer */
export function validateAcceptance(
  offer: Offer,
  acceptance: SwapAcceptance
): { valid: boolean; error?: string } {
  const env = envelope(offer);
  if (acceptance.offerId !== env.offerId) {
    return { valid: false, error: 'offer ID mismatch' };
  }
  if (acceptance.amount > offer.buyAmount) {
    return { valid: false, error: 'amount exceeds offer' };
  }
  if (!acceptance.signature || acceptance.signature.length === 0) {
    return { valid: false, error: 'missing signature' };
  }
  return { valid: true };
}

// ---------------------------------------------------------------------------
// Route operations
// ---------------------------------------------------------------------------

/** Format a route for display */
export function formatRoute(route: Route): string {
  const path = route.path.map((n) => n.slice(0, 8) + '...').join(' → ');
  return `Route: ${path} | Fee: ${route.totalFee} | Channels: ${route.channels.length}`;
}

// ---------------------------------------------------------------------------
// WebSocket event stream
// ---------------------------------------------------------------------------

export type WsEventCallback = (event: WsEventMessage) => void;
export type WsErrorCallback = (error: Error) => void;
export type WsCloseCallback = () => void;

/**
 * Subscribe to real-time events from the ++ DEX server.
 *
 * @example
 * ```ts
 * const unsub = subscribeEvents('ws://localhost:3000/ws', {
 *   onEvent: (e) => console.log('Event:', e),
 *   onError: (err) => console.error('Error:', err),
 *   onClose: () => console.log('Disconnected'),
 * });
 * // Later: unsub() to disconnect
 * ```
 */
export function subscribeEvents(
  wsUrl: string,
  callbacks: {
    onEvent: WsEventCallback;
    onError?: WsErrorCallback;
    onClose?: WsCloseCallback;
  },
  options?: { reconnect?: boolean; reconnectDelay?: number }
): () => void {
  const { reconnect = true, reconnectDelay = 3000 } = options ?? {};
  let ws: WebSocket | null = null;
  let closed = false;

  function connect() {
    if (closed) return;

    try {
      ws = new WebSocket(wsUrl);
    } catch (err) {
      callbacks.onError?.(
        err instanceof Error ? err : new Error(String(err))
      );
      if (reconnect && !closed) {
        setTimeout(connect, reconnectDelay);
      }
      return;
    }

    ws.onopen = () => {
      console.log('[++] WebSocket connected');
    };

    ws.onmessage = (msg) => {
      try {
        const data = JSON.parse(String(msg.data)) as WsEventMessage;
        callbacks.onEvent(data);
      } catch (err) {
        callbacks.onError?.(
          err instanceof Error ? err : new Error('Failed to parse WS message')
        );
      }
    };

    ws.onerror = (err) => {
      callbacks.onError?.(
        err instanceof Error ? err : new WebSocketError('WebSocket error')
      );
    };

    ws.onclose = () => {
      callbacks.onClose?.();
      if (reconnect && !closed) {
        console.log('[++] WebSocket reconnecting...');
        setTimeout(connect, reconnectDelay);
      }
    };
  }

  connect();

  // Return unsubscribe function
  return () => {
    closed = true;
    ws?.close();
  };
}

// ---------------------------------------------------------------------------
// SDK Client
// ---------------------------------------------------------------------------

/** Main SDK client for interacting with the ++ DEX */
export class PlusPlusClient {
  private config: Required<PlusPlusConfig>;

  constructor(config: PlusPlusConfig) {
    this.config = {
      requestTimeout: 30000,
      maxRetries: 3,
      retryBaseDelay: 500,
      ...config,
    };
  }

  /** Fetch available offers from the indexer */
  async getOffers(params?: {
    asset?: string;
    limit?: number;
  }): Promise<OfferEnvelope[]> {
    const url = new URL('/offers', this.config.indexerUrl);
    if (params?.asset) url.searchParams.set('asset', params.asset);
    if (params?.limit) url.searchParams.set('limit', String(params.limit));

    const data = (await fetchWithTimeout(url.toString(), {
      timeout: this.config.requestTimeout,
      maxRetries: this.config.maxRetries,
      retryBaseDelay: this.config.retryBaseDelay,
    })) as ApiResponse<OfferEnvelope[]>;

    if (!data.success) {
      throw new NetworkError(data.error ?? 'Failed to fetch offers');
    }
    return data.data ?? [];
  }

  /** Fetch a single offer by ID */
  async getOffer(offerId: string): Promise<OfferEnvelope | null> {
    try {
      const data = (await fetchWithTimeout(
        `${this.config.indexerUrl}/offers/${offerId}`,
        {
          timeout: this.config.requestTimeout,
          maxRetries: this.config.maxRetries,
          retryBaseDelay: this.config.retryBaseDelay,
        }
      )) as ApiResponse<OfferEnvelope>;

      if (!data.success) return null;
      return data.data ?? null;
    } catch (err) {
      if (err instanceof NetworkError && err.statusCode === 404) {
        return null;
      }
      throw err;
    }
  }

  /** Submit a new offer to the network */
  async submitOffer(offer: Offer): Promise<OfferEnvelope> {
    const data = (await fetchWithTimeout(`${this.config.indexerUrl}/offers`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(offer),
      timeout: this.config.requestTimeout,
      maxRetries: this.config.maxRetries,
      retryBaseDelay: this.config.retryBaseDelay,
    })) as ApiResponse<OfferEnvelope>;

    if (!data.success) {
      throw new NetworkError(data.error ?? 'Failed to submit offer');
    }
    return data.data!;
  }

  /** Cancel an active offer */
  async cancelOffer(offerId: string): Promise<void> {
    const data = (await fetchWithTimeout(
      `${this.config.indexerUrl}/offers/${offerId}`,
      {
        method: 'DELETE',
        timeout: this.config.requestTimeout,
        maxRetries: this.config.maxRetries,
        retryBaseDelay: this.config.retryBaseDelay,
      }
    )) as ApiResponse<unknown>;

    if (!data.success) {
      throw new NetworkError(data.error ?? 'Failed to cancel offer');
    }
  }

  /** Execute a swap (accept an offer) */
  async executeSwap(acceptance: SwapAcceptance): Promise<SwapResult> {
    const data = (await fetchWithTimeout(`${this.config.indexerUrl}/swaps`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(acceptance),
      timeout: this.config.requestTimeout,
      maxRetries: this.config.maxRetries,
      retryBaseDelay: this.config.retryBaseDelay,
    })) as ApiResponse<SwapResult>;

    if (!data.success) {
      throw new NetworkError(data.error ?? 'Failed to execute swap');
    }
    return data.data!;
  }

  /** Check swap status */
  async getSwapStatus(txHash: string): Promise<SwapResult> {
    const data = (await fetchWithTimeout(
      `${this.config.indexerUrl}/swaps/${txHash}`,
      {
        timeout: this.config.requestTimeout,
        maxRetries: this.config.maxRetries,
        retryBaseDelay: this.config.retryBaseDelay,
      }
    )) as ApiResponse<SwapResult>;

    if (!data.success) {
      throw new NetworkError(data.error ?? 'Failed to get swap status');
    }
    return data.data!;
  }

  /** Find a route through Fiber network */
  async findRoute(
    from: string,
    to: string,
    amount: number
  ): Promise<Route | null> {
    try {
      const data = (await fetchWithTimeout(`${this.config.fiberNode}/route`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ from, to, amount }),
        timeout: this.config.requestTimeout,
        maxRetries: this.config.maxRetries,
        retryBaseDelay: this.config.retryBaseDelay,
      })) as ApiResponse<Route>;

      if (!data.success) return null;
      return data.data ?? null;
    } catch (err) {
      if (err instanceof NetworkError && err.statusCode === 404) {
        return null;
      }
      throw err;
    }
  }

  /** Subscribe to real-time events via WebSocket */
  subscribeEvents(callbacks: {
    onEvent: WsEventCallback;
    onError?: WsErrorCallback;
    onClose?: WsCloseCallback;
  }): () => void {
    const wsUrl = this.config.indexerUrl
      .replace(/^http/, 'ws')
      .replace(/\/$/, '');
    return subscribeEvents(`${wsUrl}/ws`, callbacks, {
      reconnect: true,
      reconnectDelay: 3000,
    });
  }
}
