'use client';

import React, { useState } from 'react';
import { useAuth } from '@/src/context/AuthContext';
import { ActiveTab } from '@/src/types';
import { 
  ArrowRightLeft, 
  Store, 
  PieChart, 
  Network as NetworkIcon, 
  FileText, 
  Settings as SettingsIcon, 
  Sparkles, 
  Wallet, 
  LogOut, 
  ChevronDown, 
  ExternalLink,
  ShieldCheck,
  Coins
} from 'lucide-react';

interface TopNavbarProps {
  activeTab: ActiveTab;
  setActiveTab: (tab: ActiveTab) => void;
  onOpenSettings: () => void;
  onOpenFaucet: () => void;
}

export const TopNavbar: React.FC<TopNavbarProps> = ({
  activeTab,
  setActiveTab,
  onOpenSettings,
  onOpenFaucet,
}) => {
  const { user, isAuthenticated, isWalletConnected, activeWalletType, openAuthModal, disconnect } = useAuth();
  const [isUserMenuOpen, setIsUserMenuOpen] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  const navItems: { id: ActiveTab; label: string; icon: React.ReactNode }[] = [
    { id: 'swap', label: 'Swap', icon: <ArrowRightLeft className="w-4 h-4" /> },
    { id: 'marketplace', label: 'Marketplace', icon: <Store className="w-4 h-4" /> },
    { id: 'portfolio', label: 'Portfolio', icon: <PieChart className="w-4 h-4" /> },
    { id: 'network', label: 'Network', icon: <NetworkIcon className="w-4 h-4" /> },
    { id: 'docs', label: 'Docs', icon: <FileText className="w-4 h-4" /> },
  ];

  return (
    <header className="bg-[#161A1E]/80 backdrop-blur-md sticky top-0 z-50 border-b border-white/5 w-full">
      <div className="max-w-[1440px] mx-auto px-4 md:px-12 h-16 flex items-center justify-between">
        
        {/* Brand & Logo - Serves as Overview */}
        <div className="flex items-center gap-8">
          <button 
            id="navbar-brand-overview"
            onClick={() => setActiveTab('overview')}
            className={`flex items-center gap-2.5 text-left group focus:outline-none px-2 py-1 rounded transition-all ${
              activeTab === 'overview' ? 'bg-[#43e9b7]/10 border border-[#43e9b7]/30 ring-1 ring-[#43e9b7]/20' : 'hover:opacity-90'
            }`}
            title="Overview & Stats"
          >
            <div className={`w-8 h-8 rounded border flex items-center justify-center transition-transform group-hover:scale-105 ${
              activeTab === 'overview' ? 'bg-[#00CC9C] text-[#003829] border-[#00CC9C]' : 'bg-[#00CC9C]/10 border-[#00CC9C]/30 text-[#00CC9C]'
            }`}>
              <span className="font-mono font-black text-sm">++</span>
            </div>
            <div>
              <div className="flex items-center gap-1.5">
                <span className="text-xl font-bold tracking-tight text-[#43e9b7]">PlusPlus</span>
                <span className="text-[11px] font-mono text-[#848E9C] tracking-wide uppercase">DEX</span>
              </div>
              <span className="text-[9px] font-mono text-[#848E9C] block -mt-1 group-hover:text-[#43e9b7]">Overview</span>
            </div>
          </button>

          {/* Desktop Navigation Tabs - Horizontal, NO sidebar */}
          <nav className="hidden md:flex items-center gap-1 lg:gap-2">
            {navItems.map((item) => {
              const isActive = activeTab === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => setActiveTab(item.id)}
                  className={`px-3 py-1.5 rounded font-mono text-sm transition-all duration-150 flex items-center gap-2 ${
                    isActive
                      ? 'text-[#43e9b7] bg-[#43e9b7]/10 font-semibold border-b-2 border-[#43e9b7]'
                      : 'text-[#848E9C] hover:text-[#d9e3f3] hover:bg-white/5'
                  }`}
                >
                  {item.label}
                </button>
              );
            })}
          </nav>
        </div>

        {/* Right Controls & Wallet */}
        <div className="flex items-center gap-3">
          
          {/* Testnet Badge */}
          <div className="hidden lg:flex items-center gap-1.5 text-xs font-mono text-[#ffb874] bg-[#ffb874]/10 px-2.5 py-1 rounded border border-[#ffb874]/20">
            <span className="w-1.5 h-1.5 rounded-full bg-[#00CC9C] animate-pulse"></span>
            <span>CKB Testnet</span>
          </div>

          {/* Faucet Claim Button */}
          <button
            onClick={onOpenFaucet}
            className="hidden sm:flex items-center gap-1.5 bg-[#17202b] hover:bg-[#212b36] text-[#43e9b7] border border-[#00CC9C]/20 px-3 py-1.5 rounded text-xs font-mono transition-colors"
            title="Claim Testnet CKB, BTC & RGB++"
          >
            <Coins className="w-3.5 h-3.5" />
            <span>Faucet</span>
          </button>

          {/* Connect Wallet / User Profile */}
          {isAuthenticated && user ? (
            <div className="relative">
              <button
                onClick={() => setIsUserMenuOpen(!isUserMenuOpen)}
                className="bg-[#17202b] hover:bg-[#212b36] border border-white/10 text-[#d9e3f3] px-3 py-1.5 rounded flex items-center gap-2 text-xs font-mono transition-colors"
              >
                <div className="w-2 h-2 rounded-full bg-[#00CC9C]"></div>
                <span className="max-w-[100px] truncate">{user.name || user.walletAddress.substring(0, 10)}</span>
                <ChevronDown className="w-3.5 h-3.5 text-[#848E9C]" />
              </button>

              {/* User Dropdown */}
              {isUserMenuOpen && (
                <div className="absolute right-0 mt-2 w-64 bg-[#161A1E] border border-white/10 rounded-lg shadow-2xl p-3 z-50 text-xs font-mono">
                  <div className="pb-2 mb-2 border-b border-white/5">
                    <div className="text-[#848E9C] text-[10px] uppercase tracking-wider">Account / Provider</div>
                    <div className="font-semibold text-[#d9e3f3] mt-0.5">{activeWalletType || 'Clerk Auth'}</div>
                    <div className="text-[11px] text-[#848E9C] truncate mt-1">{user.walletAddress}</div>
                  </div>

                  <div className="space-y-1.5 py-1 text-xs">
                    <div className="flex justify-between text-[#848E9C]">
                      <span>CKB Balance</span>
                      <span className="text-[#43e9b7] font-semibold">{user.ckbBalance.toLocaleString()} CKB</span>
                    </div>
                    <div className="flex justify-between text-[#848E9C]">
                      <span>BTC Balance</span>
                      <span className="text-[#F7931A] font-semibold">{user.btcBalance.toFixed(4)} BTC</span>
                    </div>
                    <div className="flex justify-between text-[#848E9C]">
                      <span>RGB++ Balance</span>
                      <span className="text-[#00CC9C] font-semibold">{user.rgbBalance.toLocaleString()} RGB++</span>
                    </div>
                  </div>

                  <div className="pt-2 mt-2 border-t border-white/5 flex flex-col gap-1">
                    <button
                      onClick={() => {
                        setActiveTab('portfolio');
                        setIsUserMenuOpen(false);
                      }}
                      className="w-full text-left py-1.5 px-2 rounded hover:bg-white/5 text-[#d9e3f3] flex items-center justify-between"
                    >
                      <span>View Portfolio</span>
                      <ExternalLink className="w-3 h-3 text-[#848E9C]" />
                    </button>
                    <button
                      onClick={() => {
                        disconnect();
                        setIsUserMenuOpen(false);
                      }}
                      className="w-full text-left py-1.5 px-2 rounded hover:bg-red-500/10 text-red-400 flex items-center gap-1.5"
                    >
                      <LogOut className="w-3.5 h-3.5" />
                      <span>Disconnect</span>
                    </button>
                  </div>
                </div>
              )}
            </div>
          ) : (
            <button
              onClick={openAuthModal}
              className="bg-[#43e9b7] hover:bg-[#35dfae] text-[#003829] font-semibold text-xs px-4 py-2 rounded flex items-center gap-1.5 transition-all active:scale-95 shadow-sm"
            >
              <Wallet className="w-3.5 h-3.5" />
              <span>Connect Wallet</span>
            </button>
          )}

          {/* Settings Button */}
          <button
            onClick={onOpenSettings}
            className="text-[#848E9C] hover:text-[#43e9b7] p-2 rounded hover:bg-white/5 transition-colors"
            title="Settings & Network Config"
          >
            <SettingsIcon className="w-4 h-4" />
          </button>

          {/* Mobile Menu Toggle */}
          <button
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="md:hidden text-[#848E9C] hover:text-[#d9e3f3] p-2"
          >
            <div className="w-5 h-4 flex flex-col justify-between">
              <span className="w-full h-0.5 bg-current rounded"></span>
              <span className="w-full h-0.5 bg-current rounded"></span>
              <span className="w-full h-0.5 bg-current rounded"></span>
            </div>
          </button>
        </div>
      </div>

      {/* Mobile Drawer Menu */}
      {mobileMenuOpen && (
        <div className="md:hidden bg-[#0B0E11] border-b border-white/10 px-4 py-3 space-y-2">
          {navItems.map((item) => (
            <button
              key={item.id}
              onClick={() => {
                setActiveTab(item.id);
                setMobileMenuOpen(false);
              }}
              className={`w-full text-left px-3 py-2 rounded font-mono text-sm flex items-center gap-3 ${
                activeTab === item.id
                  ? 'text-[#43e9b7] bg-[#43e9b7]/10'
                  : 'text-[#848E9C] hover:text-[#d9e3f3]'
              }`}
            >
              {item.icon}
              <span>{item.label}</span>
            </button>
          ))}
          <div className="pt-2 border-t border-white/5 flex gap-2">
            <button
              onClick={() => {
                onOpenFaucet();
                setMobileMenuOpen(false);
              }}
              className="flex-1 bg-[#17202b] text-[#43e9b7] py-2 rounded text-xs font-mono text-center"
            >
              Claim Faucet
            </button>
          </div>
        </div>
      )}
    </header>
  );
};
