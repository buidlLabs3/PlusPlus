import { 
  ApiAsset, 
  ApiOffer, 
  ApiResponse, 
  ApiSwap, 
  Asset, 
  FiberNode, 
  NetworkStats, 
  Offer, 
  RouteResponse, 
  ServerInfoResponse, 
  Swap, 
  User 
} from '@/src/types';

const BASE_URL = process.env.NEXT_PUBLIC_PLUSPLUS_API_URL || '';

// Helper to normalize ApiAsset to UI Asset
export function normalizeAsset(raw: ApiAsset | any): Asset {
  const symbol = raw.symbol || 'RGB++';
  return {
    ...raw,
    asset_id: raw.asset_id || raw.id || 'asset-default',
    id: raw.id || raw.asset_id || `asset-${symbol.toLowerCase()}`,
    name: raw.name || `${symbol} Token`,
    symbol: symbol,
    type: raw.type || 'RGB++',
    decimals: raw.decimals ?? 8,
    totalSupply: raw.totalSupply || (raw.total_supply ? raw.total_supply.toLocaleString() : '21,000,000'),
    total_supply: raw.total_supply ?? 21000000,
    issuer: raw.issuer || raw.issuer_lock || '0xdfec80...covenant',
    issuer_lock: raw.issuer_lock || raw.issuer || '0xdfec80...',
    code_hash: raw.code_hash || '0x6798ae...',
    registered_block: raw.registered_block ?? 1284000,
    utxoCellId: raw.utxoCellId || `${(raw.code_hash || '0x6798').substring(0, 8)}...:0 (BTC Bound)`,
    description: raw.description || `${raw.name || symbol} isomorphic-bound RGB++ asset`,
    priceBtc: raw.priceBtc ?? (symbol === 'SEAL' ? 0.00012 : symbol === 'NOVX' ? 0.00085 : symbol === 'BTC' ? 1.0 : symbol === 'CKB' ? 0.00000018 : 0.00005),
    createdAt: raw.createdAt || new Date().toISOString(),
  };
}

// Helper to normalize ApiOffer to UI Offer
export function normalizeOffer(raw: ApiOffer | any): Offer {
  const sell_amount = raw.sell_amount ?? raw.sellAmount ?? 1000;
  const buy_amount = raw.buy_amount ?? raw.buyAmount ?? 0.05;
  const rate = raw.rate ?? (sell_amount > 0 ? Number((buy_amount / sell_amount).toFixed(8)) : 0.00005);
  const offer_id = raw.offer_id || raw.id || `offer-${Math.floor(1000 + Math.random() * 9000)}`;

  let assetSymbol = 'RGB++';
  if (raw.assetSymbol) {
    assetSymbol = raw.assetSymbol;
  } else if (raw.sell_asset) {
    if (raw.sell_asset.toLowerCase().includes('88c2')) assetSymbol = 'SEAL';
    else if (raw.sell_asset.toLowerCase().includes('71fa')) assetSymbol = 'NOVX';
    else if (raw.sell_asset.toLowerCase().includes('6798')) assetSymbol = 'RGB++';
  }

  return {
    ...raw,
    offer_id,
    id: offer_id,
    offerNumber: raw.offerNumber || (parseInt(offer_id.replace(/\D/g, '').substring(0, 4)) || 1042),
    sellerId: raw.sellerId || 'usr_seller',
    sellerAddress: raw.sellerAddress || raw.seller_lock || '0xdfec80...covenant',
    seller_lock: raw.seller_lock || raw.sellerAddress || '0xdfec80...',
    assetSymbol,
    sell_asset: raw.sell_asset || '0x6798aeb78f2389dcba1902834710928347109283741029384710293847102938',
    sell_amount,
    sellAmount: sell_amount,
    buy_asset: raw.buy_asset || '0x0000000000000000000000000000000000000000000000000000000000000000',
    buyAsset: raw.buyAsset || 'BTC',
    buy_amount,
    buyAmount: buy_amount,
    rate,
    expiry: raw.expiry ?? 100000,
    expiryBlock: raw.expiryBlock || `Blk ${(raw.expiry ? raw.expiry / 10000 : 12.8).toFixed(1)}M`,
    status: raw.status || 'Active',
    created_block: raw.created_block ?? 1284100,
    updated_block: raw.updated_block ?? 1284100,
    txHash: raw.txHash || `0x${offer_id.substring(0, 16)}...`,
    createdAt: raw.createdAt || new Date().toISOString(),
  };
}

// Helper to normalize ApiSwap to UI Swap
export function normalizeSwap(raw: ApiSwap | any): Swap {
  const tx_hash = raw.tx_hash || raw.txHash || `0x${Math.random().toString(16).substring(2, 18)}...`;
  const amount = raw.amount ?? raw.sellAmount ?? 1000;
  return {
    ...raw,
    tx_hash,
    txHash: tx_hash,
    id: raw.id || tx_hash,
    offer_id: raw.offer_id || raw.offerId || 'offer-1042',
    offerId: raw.offer_id || raw.offerId || 'offer-1042',
    buyer_lock: raw.buyer_lock || raw.buyerAddress || '0x789abc...',
    buyerAddress: raw.buyerAddress || raw.buyer_lock || 'ckb1qyq2...91ab',
    sellerAddress: raw.sellerAddress || 'ckb1qzda...0xwsq',
    assetSymbol: raw.assetSymbol || 'RGB++',
    amount,
    sellAmount: raw.sellAmount ?? amount,
    buyAmount: raw.buyAmount ?? (amount * 0.00005),
    rate: raw.rate ?? 0.00005,
    status: raw.status || 'Confirmed',
    block: raw.block ?? 1284150,
    routePath: raw.routePath || 'Fiber Channel Routing (Dijkstra optimal multi-hop)',
    networkFee: raw.networkFee || '0.0001 CKB',
    createdAt: raw.createdAt || new Date().toISOString(),
  };
}

// GET /info (Server info and stats)
export async function fetchServerInfo(): Promise<ServerInfoResponse['data']> {
  const res = await fetch(`${BASE_URL}/info`);
  const json: ServerInfoResponse = await res.json();
  if (!json.success && (json as any).error) {
    throw new Error((json as any).error || 'Failed to fetch server info');
  }
  return json.data;
}

// GET /stats (Combined UI stats)
export async function fetchStats(): Promise<NetworkStats> {
  const res = await fetch(`${BASE_URL}/api/stats`);
  if (!res.ok) {
    // Fallback to /info
    const info = await fetchServerInfo();
    return {
      activeOffers: info.offers_count,
      totalSwaps: info.swaps_count,
      fiberNodes: 2,
      makerFee: '0%',
      takerFee: '0.2%',
      activeChannels: 10,
      connectedPeers: 23,
      settlementSpeedMs: 420,
    };
  }
  return res.json();
}

// GET /offers (List offers with filters)
export async function fetchOffers(status?: string, asset?: string): Promise<Offer[]> {
  const params = new URLSearchParams();
  if (status) params.append('status', status);
  if (asset) params.append('asset', asset);
  const res = await fetch(`${BASE_URL}/offers?${params.toString()}`);
  const json = await res.json();
  const list = Array.isArray(json) ? json : json.data || [];
  return list.map(normalizeOffer);
}

// POST /offers (Create a new offer)
export async function createOffer(data: {
  sellerId?: string;
  sellerAddress?: string;
  seller_lock_hash?: string;
  assetSymbol?: string;
  sell_type_code_hash?: string;
  sellAmount?: number;
  sell_amount?: number;
  buyAsset?: string;
  buy_type_code_hash?: string;
  buyAmount?: number;
  buy_amount?: number;
  expiry?: number;
  signature?: string;
}): Promise<Offer> {
  const payload = {
    sell_type_code_hash: data.sell_type_code_hash || (data.assetSymbol === 'SEAL' ? '0x88c291ab47109284102947192847102938471092834710293847102938471029' : data.assetSymbol === 'NOVX' ? '0x71fa9900c410be22904bc3771940ef9012a43b18c7720938472910abcdef1234' : '0x6798aeb78f2389dcba1902834710928347109283741029384710293847102938'),
    sell_type_args: '0x',
    sell_amount: Number(data.sell_amount || data.sellAmount),
    buy_type_code_hash: data.buy_type_code_hash || '0x0000000000000000000000000000000000000000000000000000000000000000',
    buy_type_args: '0x',
    buy_amount: Number(data.buy_amount || data.buyAmount),
    seller_lock_hash: data.seller_lock_hash || data.sellerAddress || '0xdfec80123456789abcdef0123456789abcdef0123456789abcdef0123456789a',
    expiry: Number(data.expiry || 100000),
    signature: data.signature,
  };

  const res = await fetch(`${BASE_URL}/offers`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  const json = await res.json();
  if (!res.ok || (json.success === false)) {
    throw new Error(json.error || 'Failed to create offer');
  }
  return normalizeOffer(json.data || json);
}

// POST /offers/{offer_id}/cancel (Cancel an offer)
export async function cancelOffer(offer_id: string): Promise<boolean> {
  const res = await fetch(`${BASE_URL}/offers/${offer_id}/cancel`, {
    method: 'POST',
  });
  const json = await res.json();
  if (!res.ok || json.success === false) {
    throw new Error(json.error || 'Failed to cancel offer');
  }
  return true;
}

// GET /swaps (List swap history)
export async function fetchSwaps(userAddress?: string): Promise<Swap[]> {
  const params = new URLSearchParams();
  if (userAddress) params.append('buyer_lock', userAddress);
  const res = await fetch(`${BASE_URL}/swaps?${params.toString()}`);
  const json = await res.json();
  const list = Array.isArray(json) ? json : json.data || [];
  return list.map(normalizeSwap);
}

// POST /swaps (Execute a swap)
export async function executeSwap(data: {
  offer_id?: string;
  offerId?: string;
  buyer_lock_hash?: string;
  buyerAddress?: string;
  sellerAddress?: string;
  assetSymbol?: string;
  amount?: number;
  sellAmount?: number;
  buyAmount?: number;
  signature?: string;
}): Promise<{ swap: Swap; offer?: Offer }> {
  const offer_id = data.offer_id || data.offerId;
  const buyer_lock_hash = data.buyer_lock_hash || data.buyerAddress || '0x789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456';
  const amount = Number(data.amount || data.buyAmount || data.sellAmount || 500);

  const res = await fetch(`${BASE_URL}/swaps`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      offer_id,
      buyer_lock_hash,
      amount,
      signature: data.signature,
    }),
  });
  const json = await res.json();
  if (!res.ok || json.success === false) {
    throw new Error(json.error || 'Failed to execute swap');
  }

  const rawSwap = json.data || json;
  return {
    swap: normalizeSwap(rawSwap),
  };
}

// GET /assets (List registered assets)
export async function fetchAssets(search?: string): Promise<Asset[]> {
  const params = new URLSearchParams();
  if (search) params.append('search', search);
  const res = await fetch(`${BASE_URL}/assets?${params.toString()}`);
  const json = await res.json();
  const list = Array.isArray(json) ? json : json.data || [];
  return list.map(normalizeAsset);
}

// POST /assets (Register a new asset)
export async function registerAsset(data: {
  name: string;
  symbol: string;
  issuer_lock?: string;
  total_supply?: number;
  totalSupply?: string;
  code_hash?: string;
}): Promise<Asset> {
  const payload = {
    name: data.name,
    symbol: data.symbol,
    issuer_lock: data.issuer_lock || '0xdfec80123456789abcdef0123456789abcdef0123456789abcdef0123456789a',
    total_supply: data.total_supply || (data.totalSupply ? parseFloat(data.totalSupply.replace(/,/g, '')) : 1000000),
    code_hash: data.code_hash || '0x6798aeb78f2389dcba1902834710928347109283741029384710293847102938',
  };

  const res = await fetch(`${BASE_URL}/assets`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  const json = await res.json();
  if (!res.ok || json.success === false) {
    throw new Error(json.error || 'Failed to register asset');
  }
  return normalizeAsset(json.data || json);
}

// POST /route (Find route through Fiber)
export async function findRoute(params: {
  from: string;
  to: string;
  amount: number;
}): Promise<RouteResponse['data']> {
  const res = await fetch(`${BASE_URL}/route`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  });
  const json = await res.json();
  if (!res.ok || json.success === false) {
    throw new Error(json.error || 'Failed to find route');
  }
  return json.data;
}

// Nodes
export async function fetchNodes(): Promise<FiberNode[]> {
  const res = await fetch(`${BASE_URL}/api/nodes`);
  if (!res.ok) throw new Error('Failed to fetch Fiber nodes');
  return res.json();
}

// User Profile
export async function fetchUserProfile(userId?: string): Promise<User> {
  const params = new URLSearchParams();
  if (userId) params.append('id', userId);
  const res = await fetch(`${BASE_URL}/api/user/profile?${params.toString()}`);
  if (!res.ok) throw new Error('Failed to fetch user profile');
  return res.json();
}

// User Sync
export async function syncUser(userData: Partial<User>): Promise<User> {
  const res = await fetch(`${BASE_URL}/api/user/sync`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(userData),
  });
  if (!res.ok) throw new Error('Failed to sync user');
  return res.json();
}

// Faucet
export async function claimFaucet(userId: string, assetType: 'CKB' | 'BTC' | 'RGB++' | 'SEAL'): Promise<{ success: boolean; message: string; user: User }> {
  const res = await fetch(`${BASE_URL}/api/faucet`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ userId, assetType }),
  });
  if (!res.ok) throw new Error('Failed to claim faucet tokens');
  return res.json();
}
