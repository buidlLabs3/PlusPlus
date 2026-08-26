/// Type definitions for the ++ RGB++ DEX SDK.

/** Cell type identifying a CKB script */
export interface CellType {
  codeHash: string; // hex
  hashType: number; // 0=data, 1=type, 2=data1
  args: string; // hex
}

/** A swap offer signed by the seller */
export interface Offer {
  sellType: CellType;
  sellAmount: number;
  buyType: CellType;
  buyAmount: number;
  sellerLockHash: string; // hex
  expiry: number; // block number
  signature: string; // hex
}

/** Offer with metadata */
export interface OfferEnvelope {
  offer: Offer;
  offerId: string; // hex
}

/** Buyer's acceptance of an offer */
export interface SwapAcceptance {
  offerId: string; // hex
  buyerLockHash: string; // hex
  amount: number;
  signature: string; // hex
}

/** Fiber channel info */
export interface Channel {
  channelId: string; // hex
  partyALock: string; // hex
  partyBLock: string; // hex
  tokenType: CellType;
  capacity: number;
  balanceA: number;
  balanceB: number;
  status: 'Opening' | 'Active' | 'Closing' | 'Closed' | 'Disputed';
  openedAt: number;
  closedAt: number;
  sequence: number;
}

/** Route through Fiber network */
export interface Route {
  path: string[]; // hex node IDs
  channels: string[]; // hex channel IDs
  totalFee: number;
  maxAmount: number;
}

/** Swap result */
export interface SwapResult {
  status: 'Pending' | 'Submitted' | 'Confirmed' | 'Failed' | 'Expired';
  txHash?: string;
  error?: string;
}

/** SDK configuration */
export interface PlusPlusConfig {
  /** CKB RPC endpoint */
  ckbRpc: string;
  /** ++ indexer endpoint */
  indexerUrl: string;
  /** Fiber node endpoint */
  fiberNode: string;
  /** Network: 'testnet' | 'mainnet' */
  network: 'testnet' | 'mainnet';
  /** Request timeout in ms (default: 30000) */
  requestTimeout?: number;
  /** Max retries for failed requests (default: 3) */
  maxRetries?: number;
  /** Base delay for exponential backoff in ms (default: 500) */
  retryBaseDelay?: number;
}

/** WebSocket event types from the server */
export type WsEvent =
  | { type: 'offer.created'; offer_id: string; seller_lock: string }
  | { type: 'offer.cancelled'; offer_id: string }
  | { type: 'swap.executed'; tx_hash: string; offer_id: string; amount: number }
  | { type: 'asset.registered'; asset_id: string; name: string; symbol: string }
  | { type: 'swap.confirmed'; tx_hash: string; offer_id: string }
  | { type: 'swap.failed'; tx_hash: string; error: string };

/** WebSocket event with timestamp */
export interface WsEventMessage {
  event: WsEvent;
  timestamp: string;
}

/** API response wrapper */
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  total?: number;
}
