'use client';

import React, { createContext, useContext, useEffect, useState } from 'react';
import { claimFaucet, fetchUserProfile, syncUser } from '@/src/lib/api';
import { User } from '@/src/types';

interface AuthContextType {
  user: User | null;
  isAuthenticated: boolean;
  isWalletConnected: boolean;
  isLoading: boolean;
  loginWithClerk: (email: string, name?: string) => Promise<void>;
  connectWallet: (walletType: 'JoyID' | 'UniSat' | 'OKX' | 'MetaMask' | 'Neuron') => Promise<void>;
  disconnect: () => void;
  refreshUser: () => Promise<void>;
  requestFaucet: (asset: 'CKB' | 'BTC' | 'RGB++' | 'SEAL') => Promise<string>;
  openAuthModal: () => void;
  closeAuthModal: () => void;
  isAuthModalOpen: boolean;
  activeWalletType: string | null;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [user, setUser] = useState<User | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [isAuthModalOpen, setIsAuthModalOpen] = useState<boolean>(false);
  const [activeWalletType, setActiveWalletType] = useState<string | null>(null);

  const loadInitialUser = async () => {
    try {
      setIsLoading(true);
      const savedUser = typeof window !== 'undefined' ? localStorage.getItem('plusplus_user') : null;
      const savedWallet = typeof window !== 'undefined' ? localStorage.getItem('plusplus_wallet_type') : null;
      if (savedWallet) setActiveWalletType(savedWallet);

      if (savedUser) {
        const parsed = JSON.parse(savedUser);
        const profile = await fetchUserProfile(parsed.id);
        setUser(profile);
      } else {
        // Fetch default testnet user
        const defaultProfile = await fetchUserProfile('usr_default_trader');
        setUser(defaultProfile);
      }
    } catch (err) {
      console.warn('Could not fetch user profile, using demo trader', err);
      const demoUser: User = {
        id: 'usr_default_trader',
        clerkId: 'user_clerk_demo_1',
        email: 'trader@rgbplusplus.io',
        name: 'Alpha RGB++ Trader',
        walletAddress: 'ckb1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq',
        ckbBalance: 125000.0,
        btcBalance: 0.45,
        rgbBalance: 5000.0,
        customTokens: { SEAL: 12000, NOVX: 500 },
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      setUser(demoUser);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadInitialUser();
  }, []);

  const refreshUser = async () => {
    if (!user) return;
    try {
      const updated = await fetchUserProfile(user.id);
      setUser(updated);
      if (typeof window !== 'undefined') {
        localStorage.setItem('plusplus_user', JSON.stringify(updated));
      }
    } catch (err) {
      console.error('Failed to refresh user', err);
    }
  };

  const loginWithClerk = async (email: string, name?: string) => {
    try {
      setIsLoading(true);
      const clerkId = `user_${Math.random().toString(36).substring(2, 11)}`;
      const synced = await syncUser({
        clerkId,
        email,
        name: name || email.split('@')[0],
        walletAddress: `ckb1q${Math.random().toString(36).substring(2, 10)}${Math.random().toString(36).substring(2, 10)}`,
        ckbBalance: 100000.0,
        btcBalance: 0.5,
        rgbBalance: 3000.0,
      });
      setUser(synced);
      setActiveWalletType('Clerk / Passkey');
      if (typeof window !== 'undefined') {
        localStorage.setItem('plusplus_user', JSON.stringify(synced));
        localStorage.setItem('plusplus_wallet_type', 'Clerk / Passkey');
      }
      setIsAuthModalOpen(false);
    } catch (err) {
      console.error('Clerk login failed', err);
      throw err;
    } finally {
      setIsLoading(false);
    }
  };

  const connectWallet = async (walletType: 'JoyID' | 'UniSat' | 'OKX' | 'MetaMask' | 'Neuron') => {
    try {
      setIsLoading(true);
      let prefix = 'ckb1q';
      if (walletType === 'UniSat') prefix = 'bc1p';
      if (walletType === 'JoyID') prefix = 'ckb1qz';
      const address = `${prefix}${Math.random().toString(36).substring(2, 14)}...${Math.random().toString(36).substring(2, 6)}`;

      const synced = await syncUser({
        name: `${walletType} User`,
        walletAddress: address,
        ckbBalance: 250000.0,
        btcBalance: 0.85,
        rgbBalance: 8000.0,
      });

      setUser(synced);
      setActiveWalletType(walletType);
      if (typeof window !== 'undefined') {
        localStorage.setItem('plusplus_user', JSON.stringify(synced));
        localStorage.setItem('plusplus_wallet_type', walletType);
      }
      setIsAuthModalOpen(false);
    } catch (err) {
      console.error('Wallet connection failed', err);
    } finally {
      setIsLoading(false);
    }
  };

  const disconnect = () => {
    if (typeof window !== 'undefined') {
      localStorage.removeItem('plusplus_user');
      localStorage.removeItem('plusplus_wallet_type');
    }
    setActiveWalletType(null);
    loadInitialUser();
  };

  const requestFaucet = async (asset: 'CKB' | 'BTC' | 'RGB++' | 'SEAL'): Promise<string> => {
    if (!user) throw new Error('No user session');
    const res = await claimFaucet(user.id, asset);
    setUser(res.user);
    if (typeof window !== 'undefined') {
      localStorage.setItem('plusplus_user', JSON.stringify(res.user));
    }
    return res.message;
  };

  const openAuthModal = () => setIsAuthModalOpen(true);
  const closeAuthModal = () => setIsAuthModalOpen(false);

  return (
    <AuthContext.Provider
      value={{
        user,
        isAuthenticated: !!user?.clerkId || !!activeWalletType,
        isWalletConnected: !!activeWalletType,
        isLoading,
        loginWithClerk,
        connectWallet,
        disconnect,
        refreshUser,
        requestFaucet,
        openAuthModal,
        closeAuthModal,
        isAuthModalOpen,
        activeWalletType,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
};

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};
