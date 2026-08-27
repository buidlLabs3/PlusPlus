'use client';

import React, { useEffect, useState } from 'react';
import { useAuth } from '@/src/context/AuthContext';
import { TopNavbar } from '@/src/components/TopNavbar';
import { HeroOverview } from '@/src/components/HeroOverview';
import { SwapInterface } from '@/src/components/SwapInterface';
import { Marketplace } from '@/src/components/Marketplace';
import { PortfolioView } from '@/src/components/PortfolioView';
import { NetworkView } from '@/src/components/NetworkView';
import { DocsView } from '@/src/components/DocsView';
import { Footer } from '@/src/components/Footer';
import { ClerkAuthModal } from '@/src/components/ClerkAuthModal';
import { CreateOfferModal } from '@/src/components/CreateOfferModal';
import { FaucetModal } from '@/src/components/FaucetModal';
import { SettingsModal } from '@/src/components/SettingsModal';
import { 
  fetchAssets, 
  fetchNodes, 
  fetchOffers, 
  fetchStats, 
  fetchSwaps 
} from '@/src/lib/api';
import { ActiveTab, Asset, FiberNode, NetworkStats, Offer, Swap } from '@/src/types';

export default function Home() {
  const [activeTab, setActiveTab] = useState<ActiveTab>('overview');
  const [stats, setStats] = useState<NetworkStats | null>(null);
  const [offers, setOffers] = useState<Offer[]>([]);
  const [swaps, setSwaps] = useState<Swap[]>([]);
  const [assets, setAssets] = useState<Asset[]>([]);
  const [nodes, setNodes] = useState<FiberNode[]>([]);
  const [loading, setLoading] = useState<boolean>(true);

  // Global Modals
  const [isCreateOfferOpen, setIsCreateOfferOpen] = useState<boolean>(false);
  const [isFaucetOpen, setIsFaucetOpen] = useState<boolean>(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);

  const loadData = async () => {
    try {
      setLoading(true);
      const [s, off, swp, ast, nd] = await Promise.all([
        fetchStats().catch(() => ({
          activeOffers: 3,
          totalSwaps: 2,
          fiberNodes: 2,
          makerFee: '0%',
          takerFee: '0.2%',
          activeChannels: 10,
          connectedPeers: 23,
          settlementSpeedMs: 420,
        })),
        fetchOffers().catch(() => []),
        fetchSwaps().catch(() => []),
        fetchAssets().catch(() => []),
        fetchNodes().catch(() => []),
      ]);

      setStats(s);
      setOffers(off);
      setSwaps(swp);
      setAssets(ast);
      setNodes(nd);
    } catch (err) {
      console.error('Error fetching Next.js DEX data', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
    const interval = setInterval(() => {
      fetchStats().then(setStats).catch(() => {});
      fetchOffers().then(setOffers).catch(() => {});
      fetchSwaps().then(setSwaps).catch(() => {});
    }, 15000);
    return () => clearInterval(interval);
  }, []);

  const handleOfferCreated = (newOffer: Offer) => {
    setOffers((prev) => [newOffer, ...prev]);
    if (stats) {
      setStats({ ...stats, activeOffers: stats.activeOffers + 1 });
    }
  };

  const handleOfferCancelled = (offerId: string) => {
    setOffers((prev) => prev.filter((o) => o.id !== offerId));
    if (stats) {
      setStats({ ...stats, activeOffers: Math.max(0, stats.activeOffers - 1) });
    }
  };

  const handleSwapSuccess = (newSwap: Swap) => {
    setSwaps((prev) => [newSwap, ...prev]);
    if (stats) {
      setStats({
        ...stats,
        totalSwaps: stats.totalSwaps + 1,
        activeOffers: Math.max(0, stats.activeOffers - 1),
      });
    }
    fetchOffers().then(setOffers).catch(() => {});
  };

  return (
    <div className="min-h-screen bg-[#0B0E11] text-[#d9e3f3] flex flex-col font-sans selection:bg-[#43e9b7]/30 selection:text-[#43e9b7]">
      
      {/* Top Navbar (Strictly NO sidebar) */}
      <TopNavbar
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onOpenFaucet={() => setIsFaucetOpen(true)}
      />

      {/* Main Content Area (Clean background without boxed grid) */}
      <main className="flex-1 flex flex-col items-center w-full">
        
        {/* Tab 1: Overview (Hero Landing) */}
        {activeTab === 'overview' && (
          <HeroOverview
            stats={stats}
            setActiveTab={setActiveTab}
            onOpenCreateOffer={() => setIsCreateOfferOpen(true)}
          />
        )}

        {/* Tab 2: Swap Terminal (Split view matching Image 3) */}
        {activeTab === 'swap' && (
          <div className="w-full max-w-[1440px] px-4 md:px-12 py-8">
            <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 items-start">
              {/* Left Column: Swap Card */}
              <div className="lg:col-span-5">
                <SwapInterface
                  assets={assets}
                  onSwapSuccess={handleSwapSuccess}
                  onOpenCreateOffer={() => setIsCreateOfferOpen(true)}
                />
              </div>

              {/* Right Column: Live Order Book Marketplace */}
              <div className="lg:col-span-7">
                <Marketplace
                  offers={offers}
                  onOpenCreateOffer={() => setIsCreateOfferOpen(true)}
                  onOfferFilled={handleSwapSuccess}
                  onOfferCancelled={handleOfferCancelled}
                />
              </div>
            </div>
          </div>
        )}

        {/* Tab 3: Marketplace (Full Screen Order Book) */}
        {activeTab === 'marketplace' && (
          <div className="w-full max-w-[1440px] px-4 md:px-12 py-8">
            <Marketplace
              offers={offers}
              onOpenCreateOffer={() => setIsCreateOfferOpen(true)}
              onOfferFilled={handleSwapSuccess}
              onOfferCancelled={handleOfferCancelled}
            />
          </div>
        )}

        {/* Tab 4: Portfolio & Swaps (Matching Image 5) */}
        {activeTab === 'portfolio' && (
          <PortfolioView
            swaps={swaps}
            assets={assets}
            onRefresh={loadData}
            onOpenFaucet={() => setIsFaucetOpen(true)}
          />
        )}

        {/* Tab 5: Fiber Network & Topology */}
        {activeTab === 'network' && (
          <NetworkView
            nodes={nodes}
            stats={stats}
            onRefresh={loadData}
          />
        )}

        {/* Tab 6: Protocol Documentation */}
        {activeTab === 'docs' && <DocsView />}

      </main>

      {/* Footer */}
      <Footer setActiveTab={setActiveTab} />

      {/* Global Modals */}
      <ClerkAuthModal />
      <CreateOfferModal
        isOpen={isCreateOfferOpen}
        onClose={() => setIsCreateOfferOpen(false)}
        onOfferCreated={handleOfferCreated}
      />
      <FaucetModal
        isOpen={isFaucetOpen}
        onClose={() => setIsFaucetOpen(false)}
      />
      <SettingsModal
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
      />

    </div>
  );
}
