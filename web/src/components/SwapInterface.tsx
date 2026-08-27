'use client';

import React, { useEffect, useState } from 'react';
import { useAuth } from '@/src/context/AuthContext';
import { executeSwap } from '@/src/lib/api';
import { Asset, Swap } from '@/src/types';
import { 
  ArrowDownUp, 
  SlidersHorizontal, 
  CheckCircle2, 
  Loader2, 
  ExternalLink, 
  AlertCircle,
  ChevronDown,
  Sparkles,
  Info
} from 'lucide-react';

interface SwapInterfaceProps {
  assets: Asset[];
  onSwapSuccess: (swap: Swap) => void;
  onOpenCreateOffer: () => void;
}

export const SwapInterface: React.FC<SwapInterfaceProps> = ({
  assets,
  onSwapSuccess,
  onOpenCreateOffer,
}) => {
  const { user, isAuthenticated, openAuthModal, refreshUser } = useAuth();

  const [sellAsset, setSellAsset] = useState<string>('RGB++');
  const [buyAsset, setBuyAsset] = useState<string>('BTC');
  const [sellAmount, setSellAmount] = useState<string>('');
  const [buyAmount, setBuyAmount] = useState<string>('');
  const [slippage, setSlippage] = useState<number>(0.5);
  const [isSlippageOpen, setIsSlippageOpen] = useState<boolean>(false);
  const [isAssetModalOpen, setIsAssetModalOpen] = useState<'sell' | 'buy' | null>(null);

  // Swap Execution Status Modal
  const [isExecuting, setIsExecuting] = useState<boolean>(false);
  const [executionStep, setExecutionStep] = useState<number>(0);
  const [executionTx, setExecutionTx] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Exchange rates relative to BTC
  const getRate = (from: string, to: string): number => {
    const rateMap: Record<string, number> = {
      'RGB++': 0.00005,
      'SEAL': 0.00012,
      'NOVX': 0.00085,
      'CKB': 0.00000018,
      'BTC': 1.0,
    };

    const fromRate = rateMap[from] || 0.00005;
    const toRate = rateMap[to] || 1.0;

    return fromRate / toRate;
  };

  const handleSellAmountChange = (val: string) => {
    setSellAmount(val);
    const num = parseFloat(val);
    if (!isNaN(num) && num > 0) {
      const rate = getRate(sellAsset, buyAsset);
      setBuyAmount((num * rate).toFixed(6));
    } else {
      setBuyAmount('');
    }
  };

  const handleBuyAmountChange = (val: string) => {
    setBuyAmount(val);
    const num = parseFloat(val);
    if (!isNaN(num) && num > 0) {
      const rate = getRate(buyAsset, sellAsset);
      setSellAmount((num * rate).toFixed(4));
    } else {
      setSellAmount('');
    }
  };

  const handleFlip = () => {
    const prevSell = sellAsset;
    const prevBuy = buyAsset;
    setSellAsset(prevBuy);
    setBuyAsset(prevSell);
    
    if (sellAmount) {
      const num = parseFloat(sellAmount);
      const rate = getRate(prevBuy, prevSell);
      setBuyAmount((num * rate).toFixed(6));
    }
  };

  const getUserBalance = (symbol: string): number => {
    if (!user) return 0;
    if (symbol === 'CKB') return user.ckbBalance;
    if (symbol === 'BTC') return user.btcBalance;
    if (symbol === 'RGB++') return user.rgbBalance;
    return user.customTokens?.[symbol] ?? 0;
  };

  const handleSetMax = (fraction: number = 1) => {
    const balance = getUserBalance(sellAsset);
    const maxVal = balance * fraction;
    handleSellAmountChange(maxVal.toString());
  };

  const handleExecuteSwap = async () => {
    if (!isAuthenticated) {
      openAuthModal();
      return;
    }

    const numSell = parseFloat(sellAmount);
    const numBuy = parseFloat(buyAmount);

    if (isNaN(numSell) || numSell <= 0 || isNaN(numBuy) || numBuy <= 0) {
      setErrorMessage('Please enter a valid amount to swap.');
      return;
    }

    const balance = getUserBalance(sellAsset);
    if (numSell > balance) {
      setErrorMessage(`Insufficient ${sellAsset} balance (${balance.toLocaleString()}). Use the Faucet in top bar.`);
      return;
    }

    try {
      setIsExecuting(true);
      setErrorMessage(null);
      setExecutionStep(1); // Signing CKB Covenant

      await new Promise((r) => setTimeout(r, 700));
      setExecutionStep(2); // Locking UTXO Bound Cell

      await new Promise((r) => setTimeout(r, 800));
      setExecutionStep(3); // Fiber Channel Pathfinding & Routing

      await new Promise((r) => setTimeout(r, 750));
      setExecutionStep(4); // On-Chain Atomic Settlement

      const result = await executeSwap({
        buyerAddress: user?.walletAddress || 'ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq',
        assetSymbol: sellAsset,
        sellAmount: numSell,
        buyAmount: numBuy,
      });

      setExecutionTx(result.swap.txHash);
      onSwapSuccess(result.swap);
      await refreshUser();
      setSellAmount('');
      setBuyAmount('');
    } catch (err: any) {
      setErrorMessage(err.message || 'Swap execution failed');
      setIsExecuting(false);
    }
  };

  const currentRate = getRate(sellAsset, buyAsset);

  return (
    <div className="w-full flex flex-col gap-6">
      
      {/* Main Swap Card */}
      <div className="bg-[#161A1E] border border-white/10 rounded-xl p-6 flex flex-col gap-5 shadow-2xl">
        
        {/* Card Header */}
        <div className="flex justify-between items-center border-b border-white/5 pb-4">
          <div className="flex items-center gap-2">
            <h2 className="text-xl font-bold text-[#d9e3f3]">Swap</h2>
            <span className="text-[10px] font-mono bg-[#00CC9C]/10 text-[#43e9b7] px-2 py-0.5 rounded border border-[#00CC9C]/20">
              Atomic Covenant
            </span>
          </div>

          <button
            onClick={() => setIsSlippageOpen(!isSlippageOpen)}
            className={`p-1.5 rounded transition-colors ${
              isSlippageOpen ? 'text-[#43e9b7] bg-white/5' : 'text-[#848E9C] hover:text-[#d9e3f3]'
            }`}
            title="Slippage & Routing Settings"
          >
            <SlidersHorizontal className="w-4 h-4" />
          </button>
        </div>

        {/* Slippage Settings Drawer */}
        {isSlippageOpen && (
          <div className="bg-[#0a141f] border border-white/5 rounded-lg p-3 text-xs font-mono space-y-2">
            <div className="flex justify-between text-[#848E9C]">
              <span>Slippage Tolerance</span>
              <span className="text-[#43e9b7]">{slippage}%</span>
            </div>
            <div className="flex gap-2">
              {[0.1, 0.5, 1.0, 2.5].map((s) => (
                <button
                  key={s}
                  onClick={() => setSlippage(s)}
                  className={`flex-1 py-1 rounded border text-center transition-all ${
                    slippage === s
                      ? 'bg-[#43e9b7]/15 border-[#43e9b7] text-[#43e9b7]'
                      : 'border-white/10 text-[#848E9C] hover:text-[#d9e3f3]'
                  }`}
                >
                  {s}%
                </button>
              ))}
            </div>
          </div>
        )}

        {/* You Sell Input Container */}
        <div className="bg-[#0a141f] border border-white/5 rounded-lg p-4 focus-within:border-[#43e9b7] transition-all">
          <div className="flex justify-between items-center mb-2 text-xs">
            <span className="text-[#848E9C]">You Sell</span>
            <div className="flex items-center gap-1 font-mono text-[#848E9C]">
              <span>Balance:</span>
              <span className="text-[#d9e3f3]">
                {isAuthenticated ? getUserBalance(sellAsset).toLocaleString() : '--'}
              </span>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <input
              type="number"
              placeholder="0.0"
              value={sellAmount}
              onChange={(e) => handleSellAmountChange(e.target.value)}
              className="bg-transparent text-2xl font-mono text-[#d9e3f3] w-full focus:outline-none border-none p-0 placeholder:text-[#848E9C]/30"
            />

            {/* Asset Selector Button */}
            <button
              onClick={() => setIsAssetModalOpen('sell')}
              className="bg-[#2c3641] hover:bg-[#303a46] text-[#d9e3f3] flex items-center gap-2 px-3 py-2 rounded-lg border border-white/10 transition-colors shrink-0 font-mono text-sm"
            >
              <span className="w-2.5 h-2.5 rounded-full bg-[#00CC9C]"></span>
              <span className="font-bold">{sellAsset}</span>
              <ChevronDown className="w-3.5 h-3.5 text-[#848E9C]" />
            </button>
          </div>

          {/* Quick percentage buttons */}
          {isAuthenticated && (
            <div className="flex gap-1.5 mt-3 pt-2 border-t border-white/5">
              {[0.25, 0.5, 0.75, 1].map((frac) => (
                <button
                  key={frac}
                  onClick={() => handleSetMax(frac)}
                  className="px-2 py-0.5 text-[10px] font-mono bg-[#161A1E] text-[#848E9C] hover:text-[#43e9b7] hover:bg-white/5 rounded border border-white/5 transition-colors"
                >
                  {frac === 1 ? 'MAX' : `${frac * 100}%`}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Swap Flip Icon */}
        <div className="flex justify-center -my-2 relative z-10">
          <button
            onClick={handleFlip}
            className="bg-[#161A1E] border border-white/10 hover:border-[#43e9b7]/50 text-[#848E9C] hover:text-[#43e9b7] p-2.5 rounded-full transition-all shadow-md active:rotate-180"
            title="Invert Swap Direction"
          >
            <ArrowDownUp className="w-4 h-4" />
          </button>
        </div>

        {/* You Buy Input Container */}
        <div className="bg-[#0a141f] border border-white/5 rounded-lg p-4 focus-within:border-[#F7931A] transition-all">
          <div className="flex justify-between items-center mb-2 text-xs">
            <span className="text-[#848E9C]">You Buy</span>
            <div className="flex items-center gap-1 font-mono text-[#848E9C]">
              <span>Balance:</span>
              <span className="text-[#d9e3f3]">
                {isAuthenticated ? getUserBalance(buyAsset).toLocaleString() : '--'}
              </span>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <input
              type="number"
              placeholder="0.0"
              value={buyAmount}
              onChange={(e) => handleBuyAmountChange(e.target.value)}
              className="bg-transparent text-2xl font-mono text-[#d9e3f3] w-full focus:outline-none border-none p-0 placeholder:text-[#848E9C]/30"
            />

            {/* Asset Selector Button */}
            <button
              onClick={() => setIsAssetModalOpen('buy')}
              className="bg-[#2c3641] hover:bg-[#303a46] text-[#d9e3f3] flex items-center gap-2 px-3 py-2 rounded-lg border border-white/10 transition-colors shrink-0 font-mono text-sm"
            >
              <span className="w-2.5 h-2.5 rounded-full bg-[#F7931A]"></span>
              <span className="font-bold">{buyAsset}</span>
              <ChevronDown className="w-3.5 h-3.5 text-[#848E9C]" />
            </button>
          </div>
        </div>

        {/* Isomorphic Route & Rate Info (Matches Image 3) */}
        <div className="bg-[#0a141f]/70 border border-white/5 rounded-lg p-4 flex flex-col gap-3 font-mono text-xs">
          <div className="flex justify-between items-center">
            <span className="text-[#848E9C]">Route</span>
            <div className="flex items-center gap-2">
              <span className="w-6 h-6 rounded-full bg-[#00CC9C]/20 border border-[#00CC9C]/40 flex items-center justify-center text-[10px] text-[#00CC9C] font-bold">
                C
              </span>
              <div className="isomorphic-line"></div>
              <span className="w-6 h-6 rounded-full bg-[#F7931A]/20 border border-[#F7931A]/40 flex items-center justify-center text-[10px] text-[#F7931A] font-bold">
                B
              </span>
            </div>
          </div>

          <div className="flex justify-between items-center text-[#848E9C]">
            <span>Best available rate</span>
            <span className="text-[#d9e3f3]">
              1 {sellAsset} ≈ {currentRate.toFixed(8)} {buyAsset}
            </span>
          </div>

          <div className="flex justify-between items-center text-[#848E9C]">
            <span>Network fee</span>
            <span className="text-[#d9e3f3]">~0.0001 CKB</span>
          </div>
        </div>

        {/* Error message */}
        {errorMessage && (
          <div className="bg-red-500/10 border border-red-500/30 text-red-400 p-3 rounded text-xs flex items-center gap-2">
            <AlertCircle className="w-4 h-4 shrink-0" />
            <span>{errorMessage}</span>
          </div>
        )}

        {/* Action Button */}
        {isAuthenticated ? (
          <button
            onClick={handleExecuteSwap}
            disabled={!sellAmount || parseFloat(sellAmount) <= 0}
            className="w-full bg-[#43e9b7] hover:bg-[#35dfae] text-[#003829] font-bold text-sm py-3.5 rounded transition-all active:scale-98 disabled:opacity-40 disabled:cursor-not-allowed shadow-lg shadow-[#43e9b7]/10"
          >
            Swap {sellAsset} for {buyAsset}
          </button>
        ) : (
          <button
            onClick={openAuthModal}
            className="w-full bg-[#43e9b7]/20 border border-[#43e9b7]/30 text-[#43e9b7] hover:bg-[#43e9b7] hover:text-[#003829] font-semibold text-sm py-3.5 rounded transition-all active:scale-98"
          >
            Connect Wallet to Swap
          </button>
        )}
      </div>

      {/* Asset Selection Modal */}
      {isAssetModalOpen && (
        <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-50 p-4">
          <div className="bg-[#161A1E] border border-white/10 rounded-xl max-w-sm w-full p-5 shadow-2xl">
            <div className="flex justify-between items-center pb-3 mb-3 border-b border-white/5">
              <h3 className="font-bold text-base text-[#d9e3f3]">
                Select {isAssetModalOpen === 'sell' ? 'Sell' : 'Buy'} Asset
              </h3>
              <button
                onClick={() => setIsAssetModalOpen(null)}
                className="text-[#848E9C] hover:text-[#d9e3f3] text-sm"
              >
                ✕
              </button>
            </div>

            <div className="space-y-1.5 max-h-72 overflow-y-auto">
              {['RGB++', 'BTC', 'CKB', 'SEAL', 'NOVX'].map((sym) => (
                <button
                  key={sym}
                  onClick={() => {
                    if (isAssetModalOpen === 'sell') {
                      setSellAsset(sym);
                      if (buyAsset === sym) setBuyAsset(sym === 'BTC' ? 'RGB++' : 'BTC');
                    } else {
                      setBuyAsset(sym);
                      if (sellAsset === sym) setSellAsset(sym === 'RGB++' ? 'BTC' : 'RGB++');
                    }
                    setIsAssetModalOpen(null);
                  }}
                  className="w-full flex items-center justify-between p-3 rounded hover:bg-white/5 transition-colors font-mono text-sm"
                >
                  <div className="flex items-center gap-3">
                    <div className="w-7 h-7 rounded-full bg-[#2c3641] flex items-center justify-center text-xs font-bold text-[#43e9b7]">
                      {sym.substring(0, 3)}
                    </div>
                    <div className="text-left">
                      <div className="font-bold text-[#d9e3f3]">{sym}</div>
                      <div className="text-[11px] text-[#848E9C]">
                        {sym === 'BTC' ? 'Bitcoin' : sym === 'CKB' ? 'Nervos Layer 1' : 'RGB++ Isomorphic Asset'}
                      </div>
                    </div>
                  </div>
                  <div className="text-right text-xs text-[#848E9C]">
                    <div>{getUserBalance(sym).toLocaleString()}</div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Execution Progress & Settlement Modal */}
      {isExecuting && (
        <div className="fixed inset-0 bg-black/80 backdrop-blur-md flex items-center justify-center z-50 p-4">
          <div className="bg-[#161A1E] border border-white/10 rounded-xl max-w-md w-full p-6 shadow-2xl space-y-6">
            
            <div className="text-center">
              <div className="w-12 h-12 rounded-full bg-[#43e9b7]/10 border border-[#43e9b7]/30 flex items-center justify-center mx-auto mb-3 text-[#43e9b7]">
                {executionTx ? (
                  <CheckCircle2 className="w-6 h-6 text-[#00CC9C]" />
                ) : (
                  <Loader2 className="w-6 h-6 animate-spin" />
                )}
              </div>
              <h3 className="text-lg font-bold text-[#d9e3f3]">
                {executionTx ? 'Atomic Swap Settled' : 'Executing CKB Covenant Swap'}
              </h3>
              <p className="text-xs text-[#848E9C] font-mono mt-1">
                {executionTx ? 'Assets transferred trustlessly on-chain' : 'Coordinating atomic transaction legs'}
              </p>
            </div>

            {/* Step Timeline */}
            <div className="space-y-3 font-mono text-xs">
              <div className={`flex items-center gap-3 p-2.5 rounded ${executionStep >= 1 ? 'bg-white/5 text-[#d9e3f3]' : 'text-[#848E9C]'}`}>
                <span className={`w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold ${executionStep > 1 ? 'bg-[#00CC9C] text-black' : executionStep === 1 ? 'bg-[#43e9b7] text-black animate-pulse' : 'bg-[#2c3641]'}`}>
                  1
                </span>
                <span>1. Sign CKB Covenant Smart Contract</span>
              </div>

              <div className={`flex items-center gap-3 p-2.5 rounded ${executionStep >= 2 ? 'bg-white/5 text-[#d9e3f3]' : 'text-[#848E9C]'}`}>
                <span className={`w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold ${executionStep > 2 ? 'bg-[#00CC9C] text-black' : executionStep === 2 ? 'bg-[#43e9b7] text-black animate-pulse' : 'bg-[#2c3641]'}`}>
                  2
                </span>
                <span>2. Lock RGB++ UTXO Isomorphic Cell</span>
              </div>

              <div className={`flex items-center gap-3 p-2.5 rounded ${executionStep >= 3 ? 'bg-white/5 text-[#d9e3f3]' : 'text-[#848E9C]'}`}>
                <span className={`w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold ${executionStep > 3 ? 'bg-[#00CC9C] text-black' : executionStep === 3 ? 'bg-[#43e9b7] text-black animate-pulse' : 'bg-[#2c3641]'}`}>
                  3
                </span>
                <span>3. Dijkstra Pathfinding on Fiber Network</span>
              </div>

              <div className={`flex items-center gap-3 p-2.5 rounded ${executionStep >= 4 ? 'bg-white/5 text-[#d9e3f3]' : 'text-[#848E9C]'}`}>
                <span className={`w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold ${executionTx ? 'bg-[#00CC9C] text-black' : executionStep === 4 ? 'bg-[#43e9b7] text-black animate-pulse' : 'bg-[#2c3641]'}`}>
                  4
                </span>
                <span>4. Simultaneous Dual-Leg Settlement</span>
              </div>
            </div>

            {/* Tx Hash display */}
            {executionTx && (
              <div className="bg-[#0a141f] border border-white/10 rounded p-3 font-mono text-xs space-y-1.5">
                <div className="text-[#848E9C] text-[10px] uppercase">CKB Transaction Hash</div>
                <div className="text-[#43e9b7] truncate">{executionTx}</div>
                <div className="flex justify-between items-center pt-1 text-[11px] text-[#848E9C]">
                  <span>Status: Confirmed in Testnet Block</span>
                  <span className="text-[#00CC9C]">0.0001 CKB Fee</span>
                </div>
              </div>
            )}

            {/* Dismiss Button */}
            {executionTx && (
              <button
                onClick={() => {
                  setIsExecuting(false);
                  setExecutionTx(null);
                }}
                className="w-full bg-[#43e9b7] text-[#003829] font-bold text-sm py-3 rounded"
              >
                Close & Return to Terminal
              </button>
            )}

          </div>
        </div>
      )}

    </div>
  );
};
