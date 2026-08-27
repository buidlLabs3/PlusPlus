'use client';

import React, { useState } from 'react';
import { useAuth } from '@/src/context/AuthContext';
import { Asset, Swap } from '@/src/types';
import { 
  ReceiptText, 
  Coins, 
  RefreshCw, 
  ExternalLink, 
  Layers, 
  Send, 
  Plus, 
  ArrowUpRight, 
  ArrowDownLeft,
  CheckCircle2,
  Lock
} from 'lucide-react';

interface PortfolioViewProps {
  swaps: Swap[];
  assets: Asset[];
  onRefresh: () => void;
  onOpenFaucet: () => void;
}

export const PortfolioView: React.FC<PortfolioViewProps> = ({
  swaps,
  assets,
  onRefresh,
  onOpenFaucet,
}) => {
  const { user, isAuthenticated, requestFaucet, refreshUser } = useAuth();
  const [isClaiming, setIsClaiming] = useState<string | null>(null);

  const handleClaim = async (asset: 'CKB' | 'BTC' | 'RGB++' | 'SEAL') => {
    try {
      setIsClaiming(asset);
      await requestFaucet(asset);
      await refreshUser();
    } catch (err: any) {
      alert(`Faucet error: ${err.message}`);
    } finally {
      setIsClaiming(null);
    }
  };

  const userSwaps = user
    ? swaps.filter(
        (s) =>
          s.buyerAddress.toLowerCase().includes(user.walletAddress.toLowerCase()) ||
          s.sellerAddress.toLowerCase().includes(user.walletAddress.toLowerCase())
      )
    : swaps;

  return (
    <div className="w-full max-w-[1440px] mx-auto px-4 md:px-12 py-8 flex flex-col gap-8">
      
      {/* Portfolio Header matching Image 5 */}
      <div className="flex flex-col md:flex-row justify-between md:items-end border-b border-white/5 pb-4 gap-4">
        <div>
          <h1 className="text-3xl md:text-4xl font-bold text-[#d9e3f3]">Portfolio & Swaps</h1>
          <p className="font-mono text-sm text-[#848E9C] mt-1.5">
            Institutional Dashboard / CKB Testnet
          </p>
        </div>

        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 bg-[#161A1E] border border-white/10 px-3.5 py-1.5 rounded text-xs font-mono">
            <span className="w-2 h-2 rounded-full bg-[#00CC9C]"></span>
            <span className="text-[#848E9C]">
              {isAuthenticated ? 'Connected to CKB Testnet' : 'Guest Mode'}
            </span>
          </div>

          <button
            onClick={onRefresh}
            className="bg-[#161A1E] hover:bg-[#2c3641] text-[#848E9C] hover:text-[#d9e3f3] border border-white/10 px-3 py-1.5 rounded font-mono text-xs flex items-center gap-1.5 transition-colors"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            <span>Refresh</span>
          </button>
        </div>
      </div>

      {/* Account Overview Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-[#161A1E] border border-white/10 rounded-xl p-4 font-mono">
          <div className="text-[11px] text-[#848E9C] uppercase tracking-wider mb-1">CKB Balance</div>
          <div className="text-2xl font-bold text-[#43e9b7]">
            {user ? user.ckbBalance.toLocaleString() : '0.00'} <span className="text-xs text-[#848E9C]">CKB</span>
          </div>
          <div className="text-[11px] text-[#848E9C] mt-2 flex justify-between">
            <span>PoW Layer 1</span>
            <button onClick={() => handleClaim('CKB')} className="text-[#43e9b7] hover:underline">
              + Faucet
            </button>
          </div>
        </div>

        <div className="bg-[#161A1E] border border-white/10 rounded-xl p-4 font-mono">
          <div className="text-[11px] text-[#848E9C] uppercase tracking-wider mb-1">BTC Balance (Fiber)</div>
          <div className="text-2xl font-bold text-[#F7931A]">
            {user ? user.btcBalance.toFixed(4) : '0.0000'} <span className="text-xs text-[#848E9C]">BTC</span>
          </div>
          <div className="text-[11px] text-[#848E9C] mt-2 flex justify-between">
            <span>Payment Channels</span>
            <button onClick={() => handleClaim('BTC')} className="text-[#F7931A] hover:underline">
              + Faucet
            </button>
          </div>
        </div>

        <div className="bg-[#161A1E] border border-white/10 rounded-xl p-4 font-mono">
          <div className="text-[11px] text-[#848E9C] uppercase tracking-wider mb-1">RGB++ Tokens</div>
          <div className="text-2xl font-bold text-[#00CC9C]">
            {user ? user.rgbBalance.toLocaleString() : '0'} <span className="text-xs text-[#848E9C]">RGB++</span>
          </div>
          <div className="text-[11px] text-[#848E9C] mt-2 flex justify-between">
            <span>Isomorphic Bound</span>
            <button onClick={() => handleClaim('RGB++')} className="text-[#00CC9C] hover:underline">
              + Faucet
            </button>
          </div>
        </div>

        <div className="bg-[#161A1E] border border-white/10 rounded-xl p-4 font-mono">
          <div className="text-[11px] text-[#848E9C] uppercase tracking-wider mb-1">SEAL Tokens</div>
          <div className="text-2xl font-bold text-[#ffb874]">
            {user?.customTokens?.SEAL ? user.customTokens.SEAL.toLocaleString() : '0'}{' '}
            <span className="text-xs text-[#848E9C]">SEAL</span>
          </div>
          <div className="text-[11px] text-[#848E9C] mt-2 flex justify-between">
            <span>Fiber Micro-swaps</span>
            <button onClick={() => handleClaim('SEAL')} className="text-[#ffb874] hover:underline">
              + Faucet
            </button>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        
        {/* My Swaps Section (Matches Image 5) */}
        <section className="bg-[#161A1E] border border-white/10 rounded-xl flex flex-col overflow-hidden shadow-2xl">
          <div className="p-4 border-b border-white/5 flex justify-between items-center">
            <h2 className="text-lg font-bold text-[#d9e3f3]">My Swaps</h2>
            <button
              onClick={onRefresh}
              className="text-[#43e9b7] hover:text-[#35dfae] text-xs font-mono flex items-center gap-1"
            >
              <RefreshCw className="w-3 h-3" />
              <span>Refresh</span>
            </button>
          </div>

          <div className="overflow-x-auto min-h-[240px]">
            <table className="w-full text-left border-collapse whitespace-nowrap">
              <thead className="bg-[#0a141f]/70 border-b border-white/5 text-[11px] font-mono text-[#848E9C] uppercase">
                <tr>
                  <th className="py-3 px-4">Tx Hash</th>
                  <th className="py-3 px-4">Offer ID</th>
                  <th className="py-3 px-4">Amount</th>
                  <th className="py-3 px-4">Status</th>
                  <th className="py-3 px-4 text-right">Time</th>
                </tr>
              </thead>
              <tbody className="font-mono text-xs divide-y divide-white/5">
                {userSwaps.length === 0 ? (
                  <tr>
                    <td colSpan={5} className="py-12 px-4 text-center text-[#848E9C]">
                      <div className="flex flex-col items-center justify-center gap-2">
                        <ReceiptText className="w-8 h-8 opacity-30 text-[#848E9C]" />
                        <h3 className="text-xs font-semibold text-[#d9e3f3] font-sans">
                          No swaps yet
                        </h3>
                        <p className="text-xs text-[#848E9C]">
                          Your swap history will appear here
                        </p>
                      </div>
                    </td>
                  </tr>
                ) : (
                  userSwaps.map((swap) => (
                    <tr key={swap.id} className="hover:bg-white/[0.02] transition-colors">
                      <td className="py-3 px-4 text-[#43e9b7] font-semibold flex items-center gap-1.5">
                        <span>{swap.txHash.substring(0, 10)}...</span>
                        <ExternalLink className="w-3 h-3 text-[#848E9C]" />
                      </td>
                      <td className="py-3 px-4 text-[#848E9C]">
                        {(swap.offerId || swap.offer_id) ? `#${(swap.offerId || swap.offer_id || '').replace('offer-', '').substring(0, 6)}` : 'Direct'}
                      </td>
                      <td className="py-3 px-4 text-[#d9e3f3]">
                        {swap.sellAmount.toLocaleString()} {swap.assetSymbol}
                      </td>
                      <td className="py-3 px-4">
                        <span className="px-2 py-0.5 rounded-full border border-[#00CC9C]/30 bg-[#00CC9C]/10 text-[#00CC9C] text-[10px] uppercase tracking-wider font-bold">
                          {swap.status}
                        </span>
                      </td>
                      <td className="py-3 px-4 text-right text-[#848E9C]">
                        {new Date(swap.createdAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>

        {/* My Assets Section (Matches Image 5) */}
        <section className="bg-[#161A1E] border border-white/10 rounded-xl flex flex-col overflow-hidden shadow-2xl">
          <div className="p-4 border-b border-white/5 flex justify-between items-center">
            <h2 className="text-lg font-bold text-[#d9e3f3]">My Assets</h2>
            <span className="text-xs font-mono text-[#848E9C]">
              {assets.length} Tokens Bound
            </span>
          </div>

          <div className="overflow-x-auto min-h-[240px]">
            <table className="w-full text-left border-collapse whitespace-nowrap">
              <thead className="bg-[#0a141f]/70 border-b border-white/5 text-[11px] font-mono text-[#848E9C] uppercase">
                <tr>
                  <th className="py-3 px-4">Name</th>
                  <th className="py-3 px-4">Symbol</th>
                  <th className="py-3 px-4">Supply</th>
                  <th className="py-3 px-4 text-right">Issuer</th>
                </tr>
              </thead>
              <tbody className="font-mono text-xs divide-y divide-white/5">
                {assets.length === 0 ? (
                  <tr>
                    <td colSpan={4} className="py-12 px-4 text-center text-[#848E9C]">
                      <div className="flex flex-col items-center justify-center gap-2">
                        <Coins className="w-8 h-8 opacity-30 text-[#848E9C]" />
                        <h3 className="text-xs font-semibold text-[#d9e3f3] font-sans">
                          No assets registered
                        </h3>
                        <p className="text-xs text-[#848E9C]">
                          Register an RGB++ asset to start trading
                        </p>
                      </div>
                    </td>
                  </tr>
                ) : (
                  assets.map((asset) => (
                    <tr key={asset.id} className="hover:bg-white/[0.02] transition-colors">
                      <td className="py-3 px-4 font-semibold text-[#d9e3f3]">
                        {asset.name}
                      </td>
                      <td className="py-3 px-4">
                        <span className="px-2 py-0.5 rounded bg-[#2c3641] text-[#43e9b7] text-[11px] font-bold">
                          {asset.symbol}
                        </span>
                      </td>
                      <td className="py-3 px-4 text-[#848E9C]">
                        {asset.totalSupply}
                      </td>
                      <td className="py-3 px-4 text-right text-[#848E9C] text-[11px]">
                        {asset.issuer.substring(0, 12)}...
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>

      </div>

    </div>
  );
};
