'use client';

import React, { useState } from 'react';
import { useAuth } from '@/src/context/AuthContext';
import { cancelOffer, executeSwap } from '@/src/lib/api';
import { Offer, Swap } from '@/src/types';
import { 
  Plus, 
  ReceiptText, 
  ArrowRight, 
  CheckCircle2, 
  Filter, 
  Search, 
  X,
  ExternalLink,
  ShieldAlert
} from 'lucide-react';

interface MarketplaceProps {
  offers: Offer[];
  onOpenCreateOffer: () => void;
  onOfferFilled: (swap: Swap) => void;
  onOfferCancelled: (offerId: string) => void;
}

export const Marketplace: React.FC<MarketplaceProps> = ({
  offers,
  onOpenCreateOffer,
  onOfferFilled,
  onOfferCancelled,
}) => {
  const { user, isAuthenticated, openAuthModal, refreshUser } = useAuth();
  const [assetFilter, setAssetFilter] = useState<string>('ALL');
  const [statusFilter, setStatusFilter] = useState<string>('ALL');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [fillingOfferId, setFillingOfferId] = useState<string | null>(null);

  const filteredOffers = offers.filter((o) => {
    if (assetFilter !== 'ALL' && o.assetSymbol?.toUpperCase() !== assetFilter.toUpperCase()) return false;
    if (statusFilter !== 'ALL') {
      const isMatch = (statusFilter === 'Active' && o.status === 'Active') ||
        (statusFilter === 'Filled' && o.status === 'Filled') ||
        (statusFilter === 'Cancelled' && o.status === 'Cancelled');
      if (!isMatch) return false;
    }
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      return (
        (o.assetSymbol && o.assetSymbol.toLowerCase().includes(q)) ||
        (o.txHash && o.txHash.toLowerCase().includes(q)) ||
        (o.offer_id && o.offer_id.toLowerCase().includes(q)) ||
        (o.offerNumber && o.offerNumber.toString().includes(q))
      );
    }
    return true;
  });

  const handleFillOffer = async (offer: Offer) => {
    if (!isAuthenticated) {
      openAuthModal();
      return;
    }

    try {
      setFillingOfferId(offer.id);
      const result = await executeSwap({
        offerId: offer.id,
        buyerAddress: user?.walletAddress || 'ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq',
        sellerAddress: offer.sellerAddress,
        assetSymbol: offer.assetSymbol,
        sellAmount: offer.sellAmount,
        buyAmount: offer.buyAmount,
      });

      onOfferFilled(result.swap);
      await refreshUser();
    } catch (err: any) {
      alert(`Swap execution failed: ${err.message}`);
    } finally {
      setFillingOfferId(null);
    }
  };

  const handleCancelOffer = async (offerId: string) => {
    if (!confirm('Are you sure you want to cancel this CKB covenant offer?')) return;
    try {
      await cancelOffer(offerId);
      onOfferCancelled(offerId);
    } catch (err: any) {
      alert(`Cancel failed: ${err.message}`);
    }
  };

  return (
    <div className="bg-[#161A1E] border border-white/10 rounded-xl flex flex-col h-full overflow-hidden shadow-2xl">
      
      {/* Marketplace Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 p-6 border-b border-white/5">
        <div>
          <div className="flex items-center gap-3">
            <h2 className="text-xl font-bold text-[#d9e3f3]">Marketplace</h2>
            <span className="text-xs font-mono text-[#848E9C]">
              {filteredOffers.length} {filteredOffers.length === 1 ? 'offer' : 'offers'} listed
            </span>
          </div>
          <p className="text-xs text-[#848E9C] font-mono mt-0.5">
            Decentralized order book enforced by CKB UTXO covenants
          </p>
        </div>

        <div className="flex items-center gap-3 w-full sm:w-auto">
          {/* Filter Pills */}
          <div className="flex items-center bg-[#0a141f] rounded border border-white/5 p-0.5 text-xs font-mono">
            {['ALL', 'OPEN', 'FILLED'].map((st) => (
              <button
                key={st}
                onClick={() => setStatusFilter(st)}
                className={`px-2.5 py-1 rounded transition-colors ${
                  statusFilter === st
                    ? 'bg-[#2c3641] text-[#43e9b7] font-bold'
                    : 'text-[#848E9C] hover:text-[#d9e3f3]'
                }`}
              >
                {st}
              </button>
            ))}
          </div>

          {/* New Offer Button */}
          <button
            onClick={onOpenCreateOffer}
            className="flex items-center gap-1.5 bg-[#2c3641] hover:bg-[#303a46] text-[#d9e3f3] hover:text-[#43e9b7] px-4 py-2 rounded border border-white/10 transition-colors font-mono text-xs font-semibold"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>New Offer</span>
          </button>
        </div>
      </div>

      {/* Asset Filter & Search Bar */}
      <div className="px-6 py-3 bg-[#0a141f]/60 border-b border-white/5 flex flex-wrap items-center justify-between gap-3 text-xs font-mono">
        <div className="flex items-center gap-2">
          <span className="text-[#848E9C]">Filter Asset:</span>
          {['ALL', 'RGB++', 'SEAL', 'NOVX'].map((ast) => (
            <button
              key={ast}
              onClick={() => setAssetFilter(ast)}
              className={`px-2 py-0.5 rounded ${
                assetFilter === ast
                  ? 'bg-[#43e9b7]/15 text-[#43e9b7] font-semibold border border-[#43e9b7]/30'
                  : 'text-[#848E9C] hover:text-[#d9e3f3]'
              }`}
            >
              {ast}
            </button>
          ))}
        </div>

        <div className="flex items-center gap-2">
          <div className="relative">
            <Search className="w-3.5 h-3.5 text-[#848E9C] absolute left-2.5 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              placeholder="Search by ID or hash..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="bg-[#161A1E] border border-white/10 rounded pl-8 pr-3 py-1 text-xs text-[#d9e3f3] placeholder:text-[#848E9C]/50 focus:outline-none focus:border-[#43e9b7] w-48"
            />
          </div>
        </div>
      </div>

      {/* Offers Table */}
      <div className="flex-1 overflow-x-auto min-h-[300px]">
        <table className="w-full text-left border-collapse">
          <thead className="bg-[#0a141f]/70 border-b border-white/5 text-[11px] font-mono text-[#848E9C] uppercase tracking-wider">
            <tr>
              <th className="p-4">Asset</th>
              <th className="p-4">Sell</th>
              <th className="p-4">Buy</th>
              <th className="p-4">Rate</th>
              <th className="p-4">Expiry</th>
              <th className="p-4">Status</th>
              <th className="p-4 text-right">Action</th>
            </tr>
          </thead>

          <tbody className="font-mono text-xs divide-y divide-white/5">
            {filteredOffers.length === 0 ? (
              <tr>
                <td colSpan={7} className="p-16 text-center text-[#848E9C]">
                  <div className="flex flex-col items-center justify-center gap-3">
                    <ReceiptText className="w-10 h-10 opacity-30 text-[#848E9C]" />
                    <div>
                      <h3 className="text-sm font-semibold text-[#d9e3f3] mb-1 font-sans">
                        No offers yet
                      </h3>
                      <p className="text-xs text-[#848E9C]">
                        Create the first offer to start trading
                      </p>
                    </div>
                    <button
                      onClick={onOpenCreateOffer}
                      className="mt-2 bg-[#43e9b7] text-[#003829] font-bold text-xs px-4 py-2 rounded flex items-center gap-1.5 hover:bg-[#35dfae] transition-colors"
                    >
                      <Plus className="w-3.5 h-3.5" />
                      <span>Create Offer</span>
                    </button>
                  </div>
                </td>
              </tr>
            ) : (
              filteredOffers.map((offer) => {
                const isSeller = user && (offer.sellerAddress === user.walletAddress || offer.sellerId === user.id);
                return (
                  <tr
                    key={offer.id}
                    className="hover:bg-white/[0.02] transition-colors group"
                  >
                    {/* Asset Symbol */}
                    <td className="p-4 font-bold text-[#d9e3f3] flex items-center gap-2">
                      <span className="w-2 h-2 rounded-full bg-[#00CC9C]"></span>
                      <span>{offer.assetSymbol}</span>
                      <span className="text-[10px] text-[#848E9C] font-normal">#{offer.offerNumber}</span>
                    </td>

                    {/* Sell Amount */}
                    <td className="p-4 text-red-400 font-semibold">
                      {offer.sellAmount.toLocaleString()} {offer.assetSymbol}
                    </td>

                    {/* Buy Amount */}
                    <td className="p-4 text-[#00CC9C] font-semibold">
                      {offer.buyAmount} {offer.buyAsset}
                    </td>

                    {/* Rate */}
                    <td className="p-4 text-[#848E9C]">
                      {offer.rate.toFixed(8)}
                    </td>

                    {/* Expiry Block */}
                    <td className="p-4 text-[#848E9C]">
                      {offer.expiryBlock}
                    </td>

                    {/* Status Badge */}
                    <td className="p-4">
                      {offer.status === 'Active' && (
                        <span className="px-2 py-0.5 bg-[#43e9b7]/10 text-[#43e9b7] rounded text-[10px] font-bold border border-[#43e9b7]/20 uppercase">
                          ACTIVE
                        </span>
                      )}
                      {offer.status === 'Filled' && (
                        <span className="px-2 py-0.5 bg-white/5 text-[#848E9C] rounded text-[10px] font-bold uppercase">
                          FILLED
                        </span>
                      )}
                      {offer.status === 'Cancelled' && (
                        <span className="px-2 py-0.5 bg-red-500/10 text-red-400 rounded text-[10px] font-bold uppercase">
                          CANCELLED
                        </span>
                      )}
                    </td>

                    {/* Action Button */}
                    <td className="p-4 text-right">
                      {offer.status === 'Active' ? (
                        isSeller ? (
                          <button
                            onClick={() => handleCancelOffer(offer.id || offer.offer_id)}
                            className="text-xs text-red-400 hover:text-red-300 px-2.5 py-1 rounded bg-red-500/10 hover:bg-red-500/20 transition-colors"
                          >
                            Cancel
                          </button>
                        ) : (
                          <button
                            onClick={() => handleFillOffer(offer)}
                            disabled={fillingOfferId === (offer.id || offer.offer_id)}
                            className="bg-[#43e9b7] hover:bg-[#35dfae] text-[#003829] font-bold text-xs px-3.5 py-1.5 rounded transition-all active:scale-95 disabled:opacity-50"
                          >
                            {fillingOfferId === (offer.id || offer.offer_id) ? 'Settling...' : 'Atomic Swap'}
                          </button>
                        )
                      ) : (
                        <span className="text-[11px] text-[#848E9C]">—</span>
                      )}
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>

    </div>
  );
};
