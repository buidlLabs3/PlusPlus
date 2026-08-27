'use client';

import React from 'react';
import { ActiveTab, NetworkStats } from '@/src/types';
import { 
  Lock, 
  Zap, 
  Bitcoin, 
  Percent, 
  GitFork, 
  Radio, 
  ArrowRight, 
  ShieldCheck, 
  CheckCircle2, 
  Activity,
  Layers
} from 'lucide-react';

interface HeroOverviewProps {
  stats: NetworkStats | null;
  setActiveTab: (tab: ActiveTab) => void;
  onOpenCreateOffer: () => void;
}

export const HeroOverview: React.FC<HeroOverviewProps> = ({
  stats,
  setActiveTab,
  onOpenCreateOffer,
}) => {
  return (
    <div className="w-full flex flex-col items-center">
      
      {/* Hero Section */}
      <section className="w-full max-w-[1440px] px-4 md:px-12 py-20 md:py-28 flex flex-col items-center text-center relative">
        
        {/* Testnet Badge */}
        <div className="inline-flex items-center gap-2 bg-[#161A1E] border border-white/10 rounded-full px-3.5 py-1.5 mb-8 shadow-sm">
          <span className="w-2 h-2 rounded-full bg-[#43e9b7] shadow-[0_0_8px_rgba(67,233,183,0.8)] animate-pulse"></span>
          <span className="text-xs font-mono text-[#848E9C]">Live on CKB Testnet & Fiber Network</span>
        </div>

        {/* Headline */}
        <h1 className="text-4xl sm:text-5xl md:text-6xl font-extrabold text-[#d9e3f3] mb-6 max-w-4xl tracking-tight leading-tight">
          Trade RGB++ Assets <br className="hidden sm:inline" />
          <span className="text-transparent bg-clip-text bg-gradient-to-r from-[#43e9b7] to-[#F7931A]">
            Trustlessly
          </span>
        </h1>

        {/* Subtitle */}
        <p className="text-base md:text-lg text-[#848E9C] max-w-2xl mb-10 leading-relaxed font-sans">
          The premier decentralized exchange for RGB++ tokens. Atomic swaps enforced by CKB covenants. No intermediaries. No custody. Sub-second settlement across Bitcoin Fiber channels.
        </p>

        {/* Action Buttons */}
        <div className="flex flex-col sm:flex-row items-center gap-4 z-10">
          <button
            onClick={() => setActiveTab('swap')}
            className="w-full sm:w-auto bg-[#43e9b7] text-[#003829] font-semibold text-sm px-8 py-3.5 rounded flex items-center justify-center gap-2 hover:bg-[#35dfae] transition-all active:scale-95 shadow-md shadow-[#43e9b7]/10"
          >
            <span>Launch DEX</span>
            <ArrowRight className="w-4 h-4" />
          </button>
          
          <button
            onClick={() => setActiveTab('docs')}
            className="w-full sm:w-auto bg-transparent border border-[#848E9C]/40 text-[#848E9C] font-semibold text-sm px-8 py-3.5 rounded flex items-center justify-center gap-2 hover:text-[#d9e3f3] hover:border-white/40 transition-all"
          >
            Learn More
          </button>
        </div>

        {/* 4-Metric Stats Bar */}
        <div className="mt-20 grid grid-cols-2 md:grid-cols-4 gap-8 md:gap-16 border-t border-white/10 pt-12 w-full max-w-4xl">
          <div className="flex flex-col items-center">
            <span className="font-mono text-3xl md:text-4xl font-bold text-[#d9e3f3] mb-1">
              {stats?.activeOffers ?? 3}
            </span>
            <span className="text-[11px] font-mono text-[#848E9C] uppercase tracking-widest">
              Active Offers
            </span>
          </div>

          <div className="flex flex-col items-center">
            <span className="font-mono text-3xl md:text-4xl font-bold text-[#d9e3f3] mb-1">
              {stats?.totalSwaps ?? 2}
            </span>
            <span className="text-[11px] font-mono text-[#848E9C] uppercase tracking-widest">
              Total Swaps
            </span>
          </div>

          <div className="flex flex-col items-center">
            <span className="font-mono text-3xl md:text-4xl font-bold text-[#d9e3f3] mb-1">
              {stats?.fiberNodes ?? 2}
            </span>
            <span className="text-[11px] font-mono text-[#848E9C] uppercase tracking-widest">
              Fiber Nodes
            </span>
          </div>

          <div className="flex flex-col items-center">
            <span className="font-mono text-3xl md:text-4xl font-bold text-[#43e9b7] mb-1">
              {stats?.makerFee ?? '0%'}
            </span>
            <span className="text-[11px] font-mono text-[#848E9C] uppercase tracking-widest">
              Maker Fee
            </span>
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section className="w-full bg-[#050f19] py-24 border-y border-white/5">
        <div className="max-w-[1440px] mx-auto px-4 md:px-12">
          
          <div className="text-center mb-16">
            <span className="text-xs font-mono text-[#848E9C] uppercase tracking-widest mb-3 block">
              Features
            </span>
            <h2 className="text-2xl md:text-3xl font-bold text-[#d9e3f3] mb-3">
              Built for the Future of Bitcoin & CKB
            </h2>
            <p className="text-sm md:text-base text-[#848E9C]">
              Leveraging CKB's unique isomorphic capabilities for trustless decentralized trading
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            
            {/* Feature 1 */}
            <div className="bg-[#161A1E] border border-white/10 p-6 rounded hover:bg-[#2c3641]/50 transition-colors duration-200">
              <div className="w-10 h-10 rounded bg-[#43e9b7]/10 border border-[#43e9b7]/20 flex items-center justify-center mb-4 text-[#43e9b7]">
                <Lock className="w-5 h-5" />
              </div>
              <h3 className="font-mono font-bold text-[#d9e3f3] mb-2 text-base">
                Trustless Swaps
              </h3>
              <p className="text-sm text-[#848E9C] leading-relaxed">
                Every trade is an atomic swap enforced by a CKB covenant smart contract. Both legs complete or both revert. No counterparty risk.
              </p>
            </div>

            {/* Feature 2 */}
            <div className="bg-[#161A1E] border border-white/10 p-6 rounded hover:bg-[#2c3641]/50 transition-colors duration-200">
              <div className="w-10 h-10 rounded bg-[#F7931A]/10 border border-[#F7931A]/20 flex items-center justify-center mb-4 text-[#F7931A]">
                <Zap className="w-5 h-5" />
              </div>
              <h3 className="font-mono font-bold text-[#d9e3f3] mb-2 text-base">
                Fiber Channel Routing
              </h3>
              <p className="text-sm text-[#848E9C] leading-relaxed">
                Off-chain payment routing via Fiber Network. Instant settlements with multi-hop pathfinding and minimal fees.
              </p>
            </div>

            {/* Feature 3 */}
            <div className="bg-[#161A1E] border border-white/10 p-6 rounded hover:bg-[#2c3641]/50 transition-colors duration-200">
              <div className="w-10 h-10 rounded bg-[#ffb874]/10 border border-[#ffb874]/20 flex items-center justify-center mb-4 text-[#ffb874]">
                <Bitcoin className="w-5 h-5" />
              </div>
              <h3 className="font-mono font-bold text-[#d9e3f3] mb-2 text-base">
                RGB++ Native
              </h3>
              <p className="text-sm text-[#848E9C] leading-relaxed">
                First-class support for RGB++ assets bound to BTC UTXOs via isomorphic binding. Trade Bitcoin-native tokens directly.
              </p>
            </div>

            {/* Feature 4 */}
            <div className="bg-[#161A1E] border border-white/10 p-6 rounded hover:bg-[#2c3641]/50 transition-colors duration-200">
              <div className="w-10 h-10 rounded bg-[#00CC9C]/10 border border-[#00CC9C]/20 flex items-center justify-center mb-4 text-[#00CC9C]">
                <Percent className="w-5 h-5" />
              </div>
              <h3 className="font-mono font-bold text-[#d9e3f3] mb-2 text-base">
                Zero Maker Fees
              </h3>
              <p className="text-sm text-[#848E9C] leading-relaxed">
                Makers pay 0% fees. Takers pay only 0.2%. Protocol treasury receives 0.05%. The fairest fee structure in DeFi.
              </p>
            </div>

            {/* Feature 5 */}
            <div className="bg-[#161A1E] border border-white/10 p-6 rounded hover:bg-[#2c3641]/50 transition-colors duration-200">
              <div className="w-10 h-10 rounded bg-[#5dfcc9]/10 border border-[#5dfcc9]/20 flex items-center justify-center mb-4 text-[#5dfcc9]">
                <GitFork className="w-5 h-5" />
              </div>
              <h3 className="font-mono font-bold text-[#d9e3f3] mb-2 text-base">
                Multi-Hop Routing
              </h3>
              <p className="text-sm text-[#848E9C] leading-relaxed">
                Liquidity-aware Dijkstra routing splits large trades across multiple paths for best execution and minimal slippage.
              </p>
            </div>

            {/* Feature 6 */}
            <div className="bg-[#161A1E] border border-white/10 p-6 rounded hover:bg-[#2c3641]/50 transition-colors duration-200">
              <div className="w-10 h-10 rounded bg-[#cdd0d5]/10 border border-[#cdd0d5]/20 flex items-center justify-center mb-4 text-[#cdd0d5]">
                <Radio className="w-5 h-5" />
              </div>
              <h3 className="font-mono font-bold text-[#d9e3f3] mb-2 text-base">
                Real-Time Events
              </h3>
              <p className="text-sm text-[#848E9C] leading-relaxed">
                WebSocket notifications for every swap, offer update, and settlement. Stay informed as trades execute on-chain.
              </p>
            </div>

          </div>
        </div>
      </section>

      {/* How it Works / Swap in 4 Steps */}
      <section className="w-full max-w-[1440px] px-4 md:px-12 py-24">
        <div className="text-center mb-16">
          <span className="text-xs font-mono text-[#848E9C] uppercase tracking-widest mb-3 block">
            How It Works
          </span>
          <h2 className="text-2xl md:text-3xl font-bold text-[#d9e3f3] mb-3">
            Swap in 4 Steps
          </h2>
          <p className="text-sm md:text-base text-[#848E9C]">
            From listing to settlement, the entire flow is trustless and on-chain
          </p>
        </div>

        <div className="max-w-3xl mx-auto space-y-4">
          
          {/* Step 01 */}
          <div className="flex gap-6 group">
            <div className="flex flex-col items-center">
              <div className="w-12 h-12 rounded border border-white/10 bg-[#161A1E] flex items-center justify-center font-mono font-bold text-[#43e9b7] group-hover:border-[#43e9b7]/50 transition-colors">
                01
              </div>
              <div className="w-px h-16 bg-white/10 mt-2"></div>
            </div>
            <div className="pt-2 pb-6">
              <h3 className="font-mono font-bold text-[#d9e3f3] text-base mb-1.5">
                Create an Offer
              </h3>
              <p className="text-sm text-[#848E9C] leading-relaxed">
                A seller signs and broadcasts an offer specifying the RGB++ asset, amount, and BTC price. The offer is indexed and visible to all takers.
              </p>
            </div>
          </div>

          {/* Step 02 */}
          <div className="flex gap-6 group">
            <div className="flex flex-col items-center">
              <div className="w-12 h-12 rounded border border-white/10 bg-[#161A1E] flex items-center justify-center font-mono font-bold text-[#43e9b7] group-hover:border-[#43e9b7]/50 transition-colors">
                02
              </div>
              <div className="w-px h-16 bg-white/10 mt-2"></div>
            </div>
            <div className="pt-2 pb-6">
              <h3 className="font-mono font-bold text-[#d9e3f3] text-base mb-1.5">
                Accept & Route
              </h3>
              <p className="text-sm text-[#848E9C] leading-relaxed">
                A buyer accepts the offer. The system finds the optimal route through Fiber channels, splitting large amounts across multiple paths if needed.
              </p>
            </div>
          </div>

          {/* Step 03 */}
          <div className="flex gap-6 group">
            <div className="flex flex-col items-center">
              <div className="w-12 h-12 rounded border border-white/10 bg-[#161A1E] flex items-center justify-center font-mono font-bold text-[#43e9b7] group-hover:border-[#43e9b7]/50 transition-colors">
                03
              </div>
              <div className="w-px h-16 bg-white/10 mt-2"></div>
            </div>
            <div className="pt-2 pb-6">
              <h3 className="font-mono font-bold text-[#d9e3f3] text-base mb-1.5">
                Atomic Execution
              </h3>
              <p className="text-sm text-[#848E9C] leading-relaxed">
                A CKB covenant contract is deployed enforcing both legs of the trade. The seller's RGB++ asset and buyer's BTC are locked in the same transaction.
              </p>
            </div>
          </div>

          {/* Step 04 */}
          <div className="flex gap-6 group">
            <div className="flex flex-col items-center">
              <div className="w-12 h-12 rounded border border-white/10 bg-[#161A1E] flex items-center justify-center font-mono font-bold text-[#43e9b7] group-hover:border-[#43e9b7]/50 transition-colors">
                04
              </div>
            </div>
            <div className="pt-2 pb-4">
              <h3 className="font-mono font-bold text-[#d9e3f3] text-base mb-1.5">
                Settlement
              </h3>
              <p className="text-sm text-[#848E9C] leading-relaxed">
                Once both parties confirm, the covenant executes: assets swap atomically. If either side fails to confirm, funds return to their owners.
              </p>
            </div>
          </div>

        </div>

        {/* Quick Launch CTA at bottom */}
        <div className="mt-16 text-center">
          <button
            onClick={() => setActiveTab('swap')}
            className="bg-[#161A1E] hover:bg-[#212b36] border border-[#00CC9C]/30 text-[#43e9b7] font-mono text-sm px-6 py-3 rounded transition-all inline-flex items-center gap-2"
          >
            <span>Enter DEX Trading Terminal</span>
            <ArrowRight className="w-4 h-4" />
          </button>
        </div>
      </section>

    </div>
  );
};
