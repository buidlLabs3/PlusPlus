// TypeScript Data Types for ++ DEX Protocol (crates/plusplus-server & RGB++)

export interface ServerInfoResponse {
  success: boolean;
  data: {
    name: string;
    version: string;
    network: string;
    offers_count: number;
    swaps_count: number;
    assets_count: number;
  };
  error?: string | null;
  total?: number | null;
}

export interface ApiOffer {
  offer_id: string;
  sell_asset: string;
  sell_amount: number;
  buy_asset: string;
  buy_amount: number;
  seller_lock: string;
  expiry: number;
  status: 'Active' | 'Filled' | 'Expired' | 'Cancelled' | string;
  created_block?: number;
  updated_block?: number;
}

export interface ApiSwap {
  tx_hash: string;
  offer_id: string;
  buyer_lock: string;
  amount: number;
  status: 'Pending' | 'Confirmed' | 'Failed' | string;
  block?: number;
}

export interface ApiAsset {
  asset_id: string;
  name: string;
  symbol: string;
  issuer_lock: string;
  total_supply: number;
  code_hash: string;
  registered_block: number;
}

export interface ApiResponse<T> {
  success: boolean;
  data: T;
  error?: string | null;
  total?: number | null;
}

export interface RouteResponse {
  success: boolean;
  data: {
    path: string[];
    channels: any[];
    total_fee: number;
    max_amount: number;
  };
}

export interface WsEvent {
  event: {
    type: 'offer.created' | 'offer.cancelled' | 'swap.executed' | 'swap.confirmed' | 'swap.failed' | 'asset.registered';
    offer_id?: string;
    seller_lock?: string;
    tx_hash?: string;
    amount?: number;
    error?: string;
    asset_id?: string;
    name?: string;
    symbol?: string;
    [key: string]: any;
  };
  timestamp: string;
}

// UI Normalization Interfaces
export interface Offer extends ApiOffer {
  id: string;
  offerNumber: number;
  assetSymbol: string;
  sellAmount: number;
  buyAsset: string;
  buyAmount: number;
  rate: number;
  expiryBlock: string;
  txHash: string;
  sellerAddress: string;
  sellerId?: string;
  createdAt: string;
}

export interface Swap extends ApiSwap {
  id: string;
  txHash: string;
  buyerAddress: string;
  sellerAddress: string;
  assetSymbol: string;
  sellAmount: number;
  buyAmount: number;
  rate: number;
  routePath: string;
  networkFee: string;
  offerId?: string;
  createdAt: string;
}

export interface Asset extends ApiAsset {
  id: string;
  type: string;
  decimals: number;
  totalSupply: string;
  issuer: string;
  utxoCellId: string;
  description: string;
  priceBtc: number;
  icon?: string;
  createdAt: string;
}

export interface User {
  id: string;
  clerkId?: string;
  email?: string;
  name: string;
  avatarUrl?: string;
  walletAddress: string;
  ckbBalance: number;
  btcBalance: number;
  rgbBalance: number;
  customTokens: Record<string, number>;
  createdAt: string;
  updatedAt: string;
}

export interface FiberNode {
  id: string;
  name: string;
  pubkey: string;
  status: 'ONLINE' | 'SYNCING' | 'OFFLINE';
  connectedPeers: number;
  channelsCount: number;
  capacityBtc: number;
  capacityCkb: number;
  latencyMs: number;
}

export interface NetworkStats {
  activeOffers: number;
  totalSwaps: number;
  fiberNodes: number;
  makerFee: string;
  takerFee: string;
  activeChannels: number;
  connectedPeers: number;
  settlementSpeedMs: number;
}

export type ActiveTab = 'overview' | 'swap' | 'marketplace' | 'portfolio' | 'network' | 'docs';
