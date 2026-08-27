'use client';

import React, { useState } from 'react';
import { 
  Layers, 
  ShieldCheck, 
  Zap, 
  Database, 
  Terminal, 
  CheckCircle2, 
  Copy, 
  ExternalLink,
  Radio,
  ArrowRight
} from 'lucide-react';

interface ApiEndpointDoc {
  method: 'GET' | 'POST' | 'DELETE' | 'WS';
  path: string;
  handler: string;
  fileLine: string;
  description: string;
  requestBody?: string;
  queryParams?: string[];
  responseExample: string;
}

const apiEndpoints: ApiEndpointDoc[] = [
  {
    method: 'GET',
    path: '/info',
    handler: 'server_info',
    fileLine: 'main.rs:500',
    description: 'Server info, version, network state, and cumulative statistics.',
    responseExample: JSON.stringify(
      {
        success: true,
        data: {
          name: '++ DEX',
          version: '0.1.0',
          network: 'testnet',
          offers_count: 3,
          swaps_count: 2,
          assets_count: 5,
        },
      },
      null,
      2
    ),
  },
  {
    method: 'GET',
    path: '/offers',
    handler: 'list_offers',
    fileLine: 'main.rs:189',
    description: 'List active and historical offers with asset, status, limit, and offset filtering.',
    queryParams: ['asset=RGB++', 'status=Active', 'limit=50', 'offset=0'],
    responseExample: JSON.stringify(
      {
        success: true,
        data: [
          {
            offer_id: 'a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0',
            sell_asset: '0x6798aeb78f2389dcba1902834710928347109283741029384710293847102938',
            sell_amount: 10000,
            buy_asset: '0x0000000000000000000000000000000000000000000000000000000000000000',
            buy_amount: 500,
            seller_lock: '0xdfec80123456789abcdef0123456789abcdef0123456789abcdef0123456789a',
            expiry: 100000,
            status: 'Active',
            created_block: 1284100,
            updated_block: 1284100,
          },
        ],
        total: 1,
      },
      null,
      2
    ),
  },
  {
    method: 'POST',
    path: '/offers',
    handler: 'create_offer',
    fileLine: 'main.rs:208',
    description: 'Create a new atomic offer with lock hash verification (broadcasts offer.created).',
    requestBody: JSON.stringify(
      {
        sell_type_code_hash: '0x6798aeb78f2389dcba1902834710928347109283741029384710293847102938',
        sell_type_args: '0x',
        sell_amount: 10000,
        buy_type_code_hash: '0x0000000000000000000000000000000000000000000000000000000000000000',
        buy_type_args: '0x',
        buy_amount: 500,
        seller_lock_hash: '0xdfec80123456789abcdef0123456789abcdef0123456789abcdef0123456789a',
        expiry: 100000,
      },
      null,
      2
    ),
    responseExample: JSON.stringify(
      {
        success: true,
        data: {
          offer_id: 'a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0',
          sell_asset: '0x6798ae...',
          sell_amount: 10000,
          buy_asset: '0x000000...',
          buy_amount: 500,
          seller_lock: '0xdfec80...',
          expiry: 100000,
          status: 'Active',
          created_block: 1284600,
          updated_block: 1284600,
        },
      },
      null,
      2
    ),
  },
  {
    method: 'DELETE',
    path: '/offers/{offer_id}',
    handler: 'cancel_offer',
    fileLine: 'main.rs:272',
    description: 'Cancel an active offer (broadcasts offer.cancelled).',
    responseExample: JSON.stringify(
      {
        success: true,
        data: null,
      },
      null,
      2
    ),
  },
  {
    method: 'GET',
    path: '/swaps',
    handler: 'list_swaps',
    fileLine: 'main.rs:306',
    description: 'List settled or pending swap executions across Fiber routing hops.',
    queryParams: ['offer_id=...', 'status=Confirmed', 'limit=50', 'offset=0'],
    responseExample: JSON.stringify(
      {
        success: true,
        data: [
          {
            tx_hash: '0x8f2a64c09d8e7b1a23e945c7601f4a9b3d2e1c0876543210fedcba9876543210',
            offer_id: 'a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0',
            buyer_lock: '0x789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456',
            amount: 500,
            status: 'Confirmed',
            block: 1284150,
          },
        ],
        total: 1,
      },
      null,
      2
    ),
  },
  {
    method: 'POST',
    path: '/swaps',
    handler: 'execute_swap',
    fileLine: 'main.rs:324',
    description: 'Execute an atomic covenant swap against an active offer (broadcasts swap.executed).',
    requestBody: JSON.stringify(
      {
        offer_id: 'a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0',
        buyer_lock_hash: '0x789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456',
        amount: 500,
      },
      null,
      2
    ),
    responseExample: JSON.stringify(
      {
        success: true,
        data: {
          tx_hash: '0x8f2a64c0...',
          offer_id: 'a1b2c3d4...',
          buyer_lock: '0x789abc...',
          amount: 500,
          status: 'Confirmed',
          block: 1284620,
        },
      },
      null,
      2
    ),
  },
  {
    method: 'GET',
    path: '/assets',
    handler: 'list_assets',
    fileLine: 'main.rs:412',
    description: 'List registered RGB++ isomorphic assets and metadata.',
    queryParams: ['search=RGB++', 'limit=50', 'offset=0'],
    responseExample: JSON.stringify(
      {
        success: true,
        data: [
          {
            asset_id: 'a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0',
            name: 'RGB++ Token',
            symbol: 'RGB++',
            issuer_lock: '0xdfec80...',
            total_supply: 21000000,
            code_hash: '0x6798ae...',
            registered_block: 1284000,
          },
        ],
        total: 1,
      },
      null,
      2
    ),
  },
  {
    method: 'POST',
    path: '/assets',
    handler: 'register_asset',
    fileLine: 'main.rs:430',
    description: 'Register a new RGB++ isomorphic asset with code hash (broadcasts asset.registered).',
    requestBody: JSON.stringify(
      {
        name: 'Novax RGB++',
        symbol: 'NOVX',
        issuer_lock: '0x4f1277a9...',
        total_supply: 500000,
        code_hash: '0x71fa9900...',
      },
      null,
      2
    ),
    responseExample: JSON.stringify(
      {
        success: true,
        data: {
          asset_id: 'c3d4e5f6...',
          name: 'Novax RGB++',
          symbol: 'NOVX',
          issuer_lock: '0x4f1277a9...',
          total_supply: 500000,
          code_hash: '0x71fa9900...',
          registered_block: 1284630,
        },
      },
      null,
      2
    ),
  },
  {
    method: 'POST',
    path: '/route',
    handler: 'find_route_handler',
    fileLine: 'main.rs:488',
    description: 'Find Dijkstra path through Fiber channel network with fee calculation.',
    requestBody: JSON.stringify(
      {
        from: '0x023c9fa8940029b98ec34710928410294719284710293847109283471029384710',
        to: '0x038810293847109283741029384710293847109283741029384710928374102938',
        amount: 1000,
      },
      null,
      2
    ),
    responseExample: JSON.stringify(
      {
        success: true,
        data: {
          path: ['0x023c9fa8...', '0xckb_settlement_gateway', '0x03881029...'],
          channels: [{ channel_id: 'ch_fiber_01', capacity: 100000, fee: 50 }],
          total_fee: 100,
          max_amount: 5000,
        },
      },
      null,
      2
    ),
  },
  {
    method: 'WS',
    path: '/ws',
    handler: 'ws_handler',
    fileLine: 'main.rs:548',
    description: 'Real-time WebSocket & SSE event stream (offer.created, offer.cancelled, swap.executed, asset.registered).',
    responseExample: JSON.stringify(
      {
        type: 'connected',
        message: 'Connected to ++ DEX event stream',
        events: [
          {
            event: {
              type: 'offer.created',
              offer_id: 'a1b2c3...',
              seller_lock: '0xdfec80...',
            },
            timestamp: '2026-08-27T00:00:00Z',
          },
        ],
      },
      null,
      2
    ),
  },
];

export const DocsView: React.FC = () => {
  const [selectedEndpoint, setSelectedEndpoint] = useState<ApiEndpointDoc>(apiEndpoints[0]);
  const [copied, setCopied] = useState<boolean>(false);
  const [liveTestResponse, setLiveTestResponse] = useState<string | null>(null);
  const [testing, setTesting] = useState<boolean>(false);

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleLiveTest = async (ep: ApiEndpointDoc) => {
    setTesting(true);
    setLiveTestResponse(null);
    try {
      if (ep.method === 'GET') {
        const res = await fetch(ep.path);
        const json = await res.json();
        setLiveTestResponse(JSON.stringify(json, null, 2));
      } else if (ep.method === 'POST' && ep.requestBody) {
        const res = await fetch(ep.path, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: ep.requestBody,
        });
        const json = await res.json();
        setLiveTestResponse(JSON.stringify(json, null, 2));
      } else if (ep.method === 'WS') {
        const res = await fetch('/ws?mode=poll');
        const json = await res.json();
        setLiveTestResponse(JSON.stringify(json, null, 2));
      } else {
        setLiveTestResponse(ep.responseExample);
      }
    } catch (err: any) {
      setLiveTestResponse(JSON.stringify({ error: err.message }, null, 2));
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="w-full max-w-[1440px] mx-auto px-4 md:px-12 py-10 flex flex-col gap-12 font-sans">
      
      {/* Header */}
      <div className="border-b border-white/5 pb-6">
        <span className="text-xs font-mono text-[#43e9b7] uppercase tracking-widest block mb-2">
          Protocol Architecture & API Reference
        </span>
        <h1 className="text-3xl md:text-4xl font-bold text-[#d9e3f3]">
          ++ DEX Protocol Documentation
        </h1>
        <p className="text-sm md:text-base text-[#848E9C] mt-2 max-w-3xl">
          Full reference for Rust server skeleton (<span className="font-mono text-[#43e9b7]">crates/plusplus-server</span>), RGB++ Isomorphic Binding, CKB Type ID Covenants, and Fiber Lightning Channels.
        </p>
      </div>

      {/* Interactive API Reference Section */}
      <section className="bg-[#161A1E] border border-white/10 rounded-xl p-6 md:p-8 space-y-6">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-white/5 pb-6">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-[#43e9b7]/10 border border-[#43e9b7]/30 flex items-center justify-center text-[#43e9b7]">
              <Terminal className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-xl font-bold text-[#d9e3f3]">++ DEX — Server API Reference</h2>
              <p className="text-xs text-[#848E9C] font-mono mt-0.5">
                Target: crates/plusplus-server/src/main.rs | Port 3000 | DB: plusplus.db
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-mono bg-[#43e9b7]/10 text-[#43e9b7] border border-[#43e9b7]/20">
              <Radio className="w-3 h-3 animate-pulse" />
              Live Server Endpoints Active
            </span>
          </div>
        </div>

        {/* Endpoints Browser */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          
          {/* Endpoint List */}
          <div className="lg:col-span-4 space-y-2 max-h-[560px] overflow-y-auto pr-1">
            {apiEndpoints.map((ep) => {
              const isSelected = selectedEndpoint.path === ep.path && selectedEndpoint.method === ep.method;
              const methodColor = 
                ep.method === 'GET' ? 'bg-blue-500/10 text-blue-400 border-blue-500/20' :
                ep.method === 'POST' ? 'bg-[#00CC9C]/10 text-[#00CC9C] border-[#00CC9C]/20' :
                ep.method === 'DELETE' ? 'bg-red-500/10 text-red-400 border-red-500/20' :
                'bg-purple-500/10 text-purple-400 border-purple-500/20';

              return (
                <button
                  key={`${ep.method}-${ep.path}`}
                  onClick={() => {
                    setSelectedEndpoint(ep);
                    setLiveTestResponse(null);
                  }}
                  className={`w-full text-left p-3 rounded-lg border transition-all flex items-center justify-between ${
                    isSelected
                      ? 'bg-[#1b2533] border-[#43e9b7]/40 shadow-sm'
                      : 'bg-[#0e1318] border-white/5 hover:border-white/20'
                  }`}
                >
                  <div className="flex items-center gap-2.5">
                    <span className={`px-2 py-0.5 rounded text-[11px] font-mono font-bold border ${methodColor}`}>
                      {ep.method}
                    </span>
                    <span className="font-mono text-xs text-[#d9e3f3] font-medium">{ep.path}</span>
                  </div>
                  <span className="text-[10px] font-mono text-[#848E9C]">{ep.fileLine}</span>
                </button>
              );
            })}
          </div>

          {/* Endpoint Details & Live Test */}
          <div className="lg:col-span-8 bg-[#0a0e13] border border-white/10 rounded-xl p-5 flex flex-col gap-4">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-white/5 pb-3">
              <div className="flex items-center gap-3">
                <span className={`px-2.5 py-1 rounded text-xs font-mono font-bold border ${
                  selectedEndpoint.method === 'GET' ? 'bg-blue-500/10 text-blue-400 border-blue-500/20' :
                  selectedEndpoint.method === 'POST' ? 'bg-[#00CC9C]/10 text-[#00CC9C] border-[#00CC9C]/20' :
                  selectedEndpoint.method === 'DELETE' ? 'bg-red-500/10 text-red-400 border-red-500/20' :
                  'bg-purple-500/10 text-purple-400 border-purple-500/20'
                }`}>
                  {selectedEndpoint.method}
                </span>
                <span className="font-mono text-sm font-semibold text-[#43e9b7]">{selectedEndpoint.path}</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-xs font-mono text-[#848E9C]">{selectedEndpoint.handler} ({selectedEndpoint.fileLine})</span>
                <button
                  onClick={() => handleLiveTest(selectedEndpoint)}
                  disabled={testing}
                  className="bg-[#43e9b7] hover:bg-[#35dfae] disabled:opacity-50 text-[#003829] font-semibold text-xs px-3 py-1.5 rounded font-mono transition-all flex items-center gap-1.5"
                >
                  {testing ? 'Calling...' : 'Live Test'}
                  <ArrowRight className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>

            <p className="text-xs text-[#848E9C]">{selectedEndpoint.description}</p>

            {selectedEndpoint.queryParams && (
              <div>
                <div className="text-[11px] font-mono uppercase text-[#848E9C] mb-1.5">Query Parameters</div>
                <div className="flex flex-wrap gap-2">
                  {selectedEndpoint.queryParams.map((q, idx) => (
                    <span key={idx} className="px-2 py-1 bg-[#161A1E] border border-white/5 rounded font-mono text-[11px] text-[#43e9b7]">
                      {q}
                    </span>
                  ))}
                </div>
              </div>
            )}

            {selectedEndpoint.requestBody && (
              <div>
                <div className="text-[11px] font-mono uppercase text-[#848E9C] mb-1.5">Request Body Schema</div>
                <pre className="bg-[#121820] border border-white/5 rounded-lg p-3 text-xs font-mono text-[#848E9C] overflow-x-auto">
                  {selectedEndpoint.requestBody}
                </pre>
              </div>
            )}

            <div>
              <div className="flex items-center justify-between mb-1.5">
                <span className="text-[11px] font-mono uppercase text-[#848E9C]">
                  {liveTestResponse ? 'Live Server Response' : 'Response Schema'}
                </span>
                <button
                  onClick={() => handleCopy(liveTestResponse || selectedEndpoint.responseExample)}
                  className="text-xs text-[#848E9C] hover:text-[#43e9b7] flex items-center gap-1 font-mono"
                >
                  {copied ? <CheckCircle2 className="w-3.5 h-3.5 text-[#43e9b7]" /> : <Copy className="w-3.5 h-3.5" />}
                  <span>{copied ? 'Copied' : 'Copy JSON'}</span>
                </button>
              </div>
              <pre className="bg-[#121820] border border-white/5 rounded-lg p-3 text-xs font-mono text-[#43e9b7] overflow-x-auto max-h-64">
                {liveTestResponse || selectedEndpoint.responseExample}
              </pre>
            </div>
          </div>

        </div>
      </section>

      {/* Protocol Pillars */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
        
        {/* Section 1: RGB++ Protocol & Isomorphic Binding */}
        <div className="bg-[#161A1E] border border-white/10 rounded-xl p-6 space-y-4">
          <div className="flex items-center gap-3 text-[#43e9b7]">
            <Layers className="w-6 h-6" />
            <h2 className="text-xl font-bold text-[#d9e3f3]">RGB++ Isomorphic Binding</h2>
          </div>
          <p className="text-sm text-[#848E9C] leading-relaxed">
            RGB++ extends the original RGB protocol by replacing client-side validation with Nervos CKB cells. A Bitcoin UTXO is bound 1:1 to a CKB Cell (known as an Isomorphic Binding).
          </p>
          <div className="bg-[#0a141f] border border-white/5 rounded-lg p-3 font-mono text-xs text-[#848E9C] space-y-1">
            <div className="text-[#43e9b7] font-semibold">// Isomorphic Binding UTXO Definition</div>
            <div>btc_utxo = (txid, vout)</div>
            <div>ckb_cell.lock = btc_time_lock(btc_utxo)</div>
            <div>ckb_cell.type = xudt_type_script(asset_args)</div>
          </div>
        </div>

        {/* Section 2: CKB Covenant Atomic Swaps */}
        <div className="bg-[#161A1E] border border-white/10 rounded-xl p-6 space-y-4">
          <div className="flex items-center gap-3 text-[#F7931A]">
            <ShieldCheck className="w-6 h-6" />
            <h2 className="text-xl font-bold text-[#d9e3f3]">Atomic Covenant Contracts</h2>
          </div>
          <p className="text-sm text-[#848E9C] leading-relaxed">
            PlusPlus utilizes CKB Turing-complete Type ID scripts to enforce atomic execution without centralized custody or escrow relays. Either both legs settle on-chain, or both expire.
          </p>
          <div className="bg-[#0a141f] border border-white/5 rounded-lg p-3 font-mono text-xs text-[#848E9C] space-y-1">
            <div className="text-[#F7931A] font-semibold">// CKB Covenant Execution Flow</div>
            <div>1. Maker locks RGB++ cell with Covenant Lock</div>
            <div>2. Taker provides BTC payment proof or CKB input</div>
            <div>3. Single CKB tx unlocks both inputs atomically</div>
          </div>
        </div>

        {/* Section 3: Fiber Payment Channel Network */}
        <div className="bg-[#161A1E] border border-white/10 rounded-xl p-6 space-y-4">
          <div className="flex items-center gap-3 text-[#00CC9C]">
            <Zap className="w-6 h-6" />
            <h2 className="text-xl font-bold text-[#d9e3f3]">Fiber Lightning Channels</h2>
          </div>
          <p className="text-sm text-[#848E9C] leading-relaxed">
            The Fiber Network delivers sub-second off-chain payment routing for CKB and RGB++ assets with HTLC-based trustless multi-hop settlements and 0% maker fees.
          </p>
          <div className="bg-[#0a141f] border border-white/5 rounded-lg p-3 font-mono text-xs text-[#848E9C] space-y-1">
            <div className="text-[#00CC9C] font-semibold">// Dijkstra Path Metric</div>
            <div>Route Cost = base_fee + (amount * fee_rate) + latency_penalty</div>
            <div>Settlement: Multi-hop HTLC on Fiber daemon</div>
          </div>
        </div>

        {/* Section 4: Prisma & Database Architecture */}
        <div className="bg-[#161A1E] border border-white/10 rounded-xl p-6 space-y-4">
          <div className="flex items-center gap-3 text-[#5dfcc9]">
            <Database className="w-6 h-6" />
            <h2 className="text-xl font-bold text-[#d9e3f3]">Prisma Database & Clerk Auth</h2>
          </div>
          <p className="text-sm text-[#848E9C] leading-relaxed">
            The application stores indexed covenants, open offers, swap receipts, user accounts, and Fiber node telemetry in Prisma models with persistent database schema synchronization.
          </p>
          <div className="bg-[#0a141f] border border-white/5 rounded-lg p-3 font-mono text-xs text-[#848E9C] space-y-1">
            <div className="text-[#5dfcc9] font-semibold">// Prisma Models Active</div>
            <div>model User, Asset, Offer, Swap, FiberNode, Channel</div>
            <div>Authentication: Clerk React SDK + Web3 Passkeys</div>
          </div>
        </div>

      </div>
    </div>
  );
};
