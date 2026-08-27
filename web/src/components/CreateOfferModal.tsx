'use client';

import React, { useState } from 'react';
import { useAuth } from '@/src/context/AuthContext';
import { createOffer } from '@/src/lib/api';
import { Offer } from '@/src/types';
import { X, Sparkles, ShieldCheck, ArrowRight, Loader2 } from 'lucide-react';

interface CreateOfferModalProps {
  isOpen: boolean;
  onClose: () => void;
  onOfferCreated: (offer: Offer) => void;
}

export const CreateOfferModal: React.FC<CreateOfferModalProps> = ({
  isOpen,
  onClose,
  onOfferCreated,
}) => {
  const { user, isAuthenticated, openAuthModal, refreshUser } = useAuth();

  const [assetSymbol, setAssetSymbol] = useState<string>('RGB++');
  const [sellAmount, setSellAmount] = useState<string>('1000');
  const [buyAsset, setBuyAsset] = useState<string>('BTC');
  const [buyAmount, setBuyAmount] = useState<string>('0.05');
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const rate =
    parseFloat(sellAmount) > 0 && parseFloat(buyAmount) > 0
      ? (parseFloat(buyAmount) / parseFloat(sellAmount)).toFixed(8)
      : '0.00000000';

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!isAuthenticated || !user) {
      openAuthModal();
      return;
    }

    const numSell = parseFloat(sellAmount);
    const numBuy = parseFloat(buyAmount);

    if (isNaN(numSell) || numSell <= 0 || isNaN(numBuy) || numBuy <= 0) {
      setError('Please enter valid numeric amounts.');
      return;
    }

    try {
      setIsSubmitting(true);
      setError(null);

      const offer = await createOffer({
        sellerId: user.id,
        sellerAddress: user.walletAddress,
        assetSymbol,
        sellAmount: numSell,
        buyAsset,
        buyAmount: numBuy,
      });

      onOfferCreated(offer);
      await refreshUser();
      onClose();
    } catch (err: any) {
      setError(err.message || 'Failed to broadcast offer to CKB Testnet');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/80 backdrop-blur-md flex items-center justify-center z-50 p-4">
      <div className="bg-[#161A1E] border border-white/10 rounded-xl max-w-lg w-full p-6 shadow-2xl space-y-5">
        
        {/* Modal Header */}
        <div className="flex justify-between items-center pb-4 border-b border-white/5">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded bg-[#43e9b7]/10 border border-[#43e9b7]/30 flex items-center justify-center text-[#43e9b7]">
              <Sparkles className="w-4 h-4" />
            </div>
            <div>
              <h3 className="font-bold text-lg text-[#d9e3f3]">Create New Offer</h3>
              <p className="text-xs text-[#848E9C] font-mono">
                Deploy atomic covenant sell order on Nervos CKB
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-[#848E9C] hover:text-[#d9e3f3] p-1.5 rounded hover:bg-white/5"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {error && (
          <div className="bg-red-500/10 border border-red-500/30 text-red-400 p-3 rounded text-xs font-mono">
            {error}
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4 font-mono text-xs">
          
          {/* Sell Asset & Amount */}
          <div className="bg-[#0a141f] border border-white/5 rounded-lg p-3.5 space-y-2">
            <div className="flex justify-between text-[#848E9C]">
              <span>Asset to Sell</span>
              <span>Your Address: {user?.walletAddress ? user.walletAddress.substring(0, 12) + '...' : 'Connect Wallet'}</span>
            </div>
            <div className="flex gap-3">
              <input
                type="number"
                placeholder="Amount (e.g. 1000)"
                value={sellAmount}
                onChange={(e) => setSellAmount(e.target.value)}
                className="bg-transparent text-lg font-bold text-[#d9e3f3] w-full focus:outline-none"
                required
              />
              <select
                value={assetSymbol}
                onChange={(e) => setAssetSymbol(e.target.value)}
                className="bg-[#2c3641] text-[#d9e3f3] px-3 py-1.5 rounded border border-white/10 focus:outline-none"
              >
                <option value="RGB++">RGB++</option>
                <option value="SEAL">SEAL</option>
                <option value="NOVX">NOVX</option>
                <option value="CKB">CKB</option>
              </select>
            </div>
          </div>

          {/* Buy Asset & Amount */}
          <div className="bg-[#0a141f] border border-white/5 rounded-lg p-3.5 space-y-2">
            <div className="flex justify-between text-[#848E9C]">
              <span>Receive Currency</span>
              <span>Locked Leg</span>
            </div>
            <div className="flex gap-3">
              <input
                type="number"
                step="0.00000001"
                placeholder="Amount in BTC (e.g. 0.05)"
                value={buyAmount}
                onChange={(e) => setBuyAmount(e.target.value)}
                className="bg-transparent text-lg font-bold text-[#d9e3f3] w-full focus:outline-none"
                required
              />
              <select
                value={buyAsset}
                onChange={(e) => setBuyAsset(e.target.value)}
                className="bg-[#2c3641] text-[#d9e3f3] px-3 py-1.5 rounded border border-white/10 focus:outline-none"
              >
                <option value="BTC">BTC</option>
                <option value="CKB">CKB</option>
              </select>
            </div>
          </div>

          {/* Rate Summary */}
          <div className="bg-[#17202b] rounded-lg p-3 space-y-1.5 text-[11px] text-[#848E9C]">
            <div className="flex justify-between">
              <span>Implied Unit Rate</span>
              <span className="text-[#43e9b7] font-bold">1 {assetSymbol} = {rate} {buyAsset}</span>
            </div>
            <div className="flex justify-between">
              <span>Covenant Lock Type</span>
              <span className="text-[#d9e3f3]">Type ID Script (Isomorphic UTXO)</span>
            </div>
            <div className="flex justify-between">
              <span>Estimated Expiry</span>
              <span className="text-[#d9e3f3]">~72 Hours (Blk 12.8M)</span>
            </div>
            <div className="flex justify-between">
              <span>Maker Fee</span>
              <span className="text-[#00CC9C] font-bold">0% (Free for Makers)</span>
            </div>
          </div>

          {/* Submit Action */}
          <div className="pt-2">
            <button
              type="submit"
              disabled={isSubmitting}
              className="w-full bg-[#43e9b7] hover:bg-[#35dfae] text-[#003829] font-bold text-sm py-3.5 rounded flex items-center justify-center gap-2 transition-all disabled:opacity-50"
            >
              {isSubmitting ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  <span>Signing CKB Covenant...</span>
                </>
              ) : (
                <>
                  <span>Broadcast Offer to Order Book</span>
                  <ArrowRight className="w-4 h-4" />
                </>
              )}
            </button>
          </div>

        </form>

      </div>
    </div>
  );
};
