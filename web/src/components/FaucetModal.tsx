'use client';

import React, { useState } from 'react';
import { useAuth } from '@/src/context/AuthContext';
import { X, Coins, Sparkles, CheckCircle2, Loader2, ArrowRight } from 'lucide-react';

interface FaucetModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const FaucetModal: React.FC<FaucetModalProps> = ({ isOpen, onClose }) => {
  const { user, isAuthenticated, requestFaucet, refreshUser, openAuthModal } = useAuth();
  const [selectedAsset, setSelectedAsset] = useState<'CKB' | 'BTC' | 'RGB++' | 'SEAL'>('RGB++');
  const [isClaiming, setIsClaiming] = useState(false);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleClaim = async () => {
    if (!isAuthenticated) {
      openAuthModal();
      return;
    }

    try {
      setIsClaiming(true);
      setError(null);
      setSuccessMessage(null);
      const msg = await requestFaucet(selectedAsset);
      setSuccessMessage(msg);
      await refreshUser();
    } catch (err: any) {
      setError(err.message || 'Faucet claim failed');
    } finally {
      setIsClaiming(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/80 backdrop-blur-md flex items-center justify-center z-50 p-4">
      <div className="bg-[#161A1E] border border-white/10 rounded-xl max-w-md w-full p-6 shadow-2xl space-y-5">
        
        <div className="flex justify-between items-center pb-3 border-b border-white/5">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded bg-[#43e9b7]/10 border border-[#43e9b7]/30 flex items-center justify-center text-[#43e9b7]">
              <Coins className="w-4 h-4" />
            </div>
            <div>
              <h3 className="font-bold text-base text-[#d9e3f3]">CKB Testnet Faucet</h3>
              <p className="text-xs text-[#848E9C] font-mono">
                Get free testnet tokens for atomic swaps
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-[#848E9C] hover:text-[#d9e3f3] p-1.5 rounded hover:bg-white/5"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {error && (
          <div className="bg-red-500/10 border border-red-500/30 text-red-400 p-2.5 rounded text-xs font-mono">
            {error}
          </div>
        )}

        {successMessage && (
          <div className="bg-[#00CC9C]/10 border border-[#00CC9C]/30 text-[#00CC9C] p-3 rounded text-xs font-mono flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 shrink-0" />
            <span>{successMessage}</span>
          </div>
        )}

        <div className="space-y-4 font-mono text-xs">
          <div>
            <label className="text-[#848E9C] block mb-2">Select Test Asset</label>
            <div className="grid grid-cols-2 gap-2.5">
              {[
                { id: 'RGB++', name: '1,000 RGB++', desc: 'Bound UTXO Token' },
                { id: 'CKB', name: '50,000 CKB', desc: 'Gas & State Storage' },
                { id: 'BTC', name: '0.10 BTC', desc: 'Fiber Lightning Channel' },
                { id: 'SEAL', name: '5,000 SEAL', desc: 'Micro-swap Liquidity' },
              ].map((item) => (
                <button
                  type="button"
                  key={item.id}
                  onClick={() => setSelectedAsset(item.id as any)}
                  className={`p-3 rounded-lg border text-left transition-all ${
                    selectedAsset === item.id
                      ? 'bg-[#43e9b7]/15 border-[#43e9b7] text-[#43e9b7]'
                      : 'border-white/10 text-[#848E9C] hover:text-[#d9e3f3] hover:bg-white/5'
                  }`}
                >
                  <div className="font-bold text-sm text-[#d9e3f3]">{item.name}</div>
                  <div className="text-[10px] text-[#848E9C] mt-0.5">{item.desc}</div>
                </button>
              ))}
            </div>
          </div>

          <div className="bg-[#0a141f] rounded p-3 text-[11px] text-[#848E9C] space-y-1">
            <div className="flex justify-between">
              <span>Target Wallet</span>
              <span className="text-[#d9e3f3] truncate max-w-[200px]">
                {user?.walletAddress || 'Connect Wallet First'}
              </span>
            </div>
            <div className="flex justify-between">
              <span>Cooldown Period</span>
              <span className="text-[#00CC9C]">0s (Unlimited on Testnet)</span>
            </div>
          </div>

          <button
            onClick={handleClaim}
            disabled={isClaiming}
            className="w-full bg-[#43e9b7] hover:bg-[#35dfae] text-[#003829] font-bold text-sm py-3.5 rounded-lg flex items-center justify-center gap-2 transition-all disabled:opacity-50"
          >
            {isClaiming ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>Minting Testnet Cells...</span>
              </>
            ) : (
              <>
                <span>Claim {selectedAsset} Tokens</span>
                <ArrowRight className="w-4 h-4" />
              </>
            )}
          </button>
        </div>

      </div>
    </div>
  );
};
