'use client';

import React, { useState } from 'react';
import { useAuth } from '@/src/context/AuthContext';
import { 
  X, 
  Mail, 
  Lock, 
  ShieldCheck, 
  Wallet, 
  KeyRound, 
  Sparkles, 
  ArrowRight, 
  Loader2,
  CheckCircle2
} from 'lucide-react';

export const ClerkAuthModal: React.FC = () => {
  const { isAuthModalOpen, closeAuthModal, loginWithClerk, connectWallet, isLoading } = useAuth();
  
  const [authTab, setAuthTab] = useState<'clerk' | 'wallets'>('clerk');
  const [email, setEmail] = useState('');
  const [name, setName] = useState('');
  const [isVerifying, setIsVerifying] = useState(false);
  const [verificationCode, setVerificationCode] = useState('');
  const [codeSent, setCodeSent] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isAuthModalOpen) return null;

  const handleSendCode = (e: React.FormEvent) => {
    e.preventDefault();
    if (!email || !email.includes('@')) {
      setError('Please enter a valid email address.');
      return;
    }
    setError(null);
    setCodeSent(true);
    setVerificationCode('749210'); // Simulated Clerk OTP
  };

  const handleVerifyClerk = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      setIsVerifying(true);
      setError(null);
      await loginWithClerk(email, name || email.split('@')[0]);
    } catch (err: any) {
      setError(err.message || 'Verification failed');
    } finally {
      setIsVerifying(false);
    }
  };

  const handleSocialLogin = async (provider: string) => {
    try {
      setIsVerifying(true);
      setError(null);
      await loginWithClerk(`${provider.toLowerCase()}@rgbplusplus.io`, `${provider} User`);
    } catch (err: any) {
      setError(err.message || 'Social login failed');
    } finally {
      setIsVerifying(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/80 backdrop-blur-md flex items-center justify-center z-50 p-4">
      <div className="bg-[#161A1E] border border-white/10 rounded-2xl max-w-md w-full p-6 sm:p-7 shadow-2xl space-y-6 relative overflow-hidden">
        
        {/* Top Accent Gradient */}
        <div className="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-[#43e9b7] via-[#00CC9C] to-[#F7931A]"></div>

        {/* Modal Header */}
        <div className="flex justify-between items-center pb-2">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-[#43e9b7]/10 border border-[#43e9b7]/30 flex items-center justify-center text-[#43e9b7]">
              <ShieldCheck className="w-4 h-4" />
            </div>
            <div>
              <h3 className="font-bold text-base text-[#d9e3f3]">PlusPlus Authentication</h3>
              <p className="text-[11px] text-[#848E9C] font-mono">
                Powered by Clerk Auth & Prisma ORM
              </p>
            </div>
          </div>
          
          <button
            onClick={closeAuthModal}
            className="text-[#848E9C] hover:text-[#d9e3f3] p-1.5 rounded-full hover:bg-white/5 transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Tab Switcher */}
        <div className="flex bg-[#0a141f] rounded-lg p-1 border border-white/5 font-mono text-xs">
          <button
            onClick={() => setAuthTab('clerk')}
            className={`flex-1 py-1.5 rounded flex items-center justify-center gap-2 transition-all ${
              authTab === 'clerk'
                ? 'bg-[#2c3641] text-[#43e9b7] font-semibold shadow'
                : 'text-[#848E9C] hover:text-[#d9e3f3]'
            }`}
          >
            <Mail className="w-3.5 h-3.5" />
            <span>Clerk Sign In</span>
          </button>

          <button
            onClick={() => setAuthTab('wallets')}
            className={`flex-1 py-1.5 rounded flex items-center justify-center gap-2 transition-all ${
              authTab === 'wallets'
                ? 'bg-[#2c3641] text-[#43e9b7] font-semibold shadow'
                : 'text-[#848E9C] hover:text-[#d9e3f3]'
            }`}
          >
            <Wallet className="w-3.5 h-3.5" />
            <span>Web3 Wallets</span>
          </button>
        </div>

        {error && (
          <div className="bg-red-500/10 border border-red-500/30 text-red-400 p-2.5 rounded text-xs font-mono">
            {error}
          </div>
        )}

        {/* Clerk Auth Tab */}
        {authTab === 'clerk' && (
          <div className="space-y-4">
            {!codeSent ? (
              <form onSubmit={handleSendCode} className="space-y-3.5">
                <div>
                  <label className="text-[11px] font-mono text-[#848E9C] block mb-1">
                    Email Address
                  </label>
                  <input
                    type="email"
                    placeholder="trader@nervos.org"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    className="w-full bg-[#0a141f] border border-white/10 focus:border-[#43e9b7] rounded-lg px-3.5 py-2.5 text-sm font-mono text-[#d9e3f3] placeholder:text-[#848E9C]/40 focus:outline-none transition-colors"
                    required
                  />
                </div>

                <div>
                  <label className="text-[11px] font-mono text-[#848E9C] block mb-1">
                    Display Name (Optional)
                  </label>
                  <input
                    type="text"
                    placeholder="Satoshi Nakamoto"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    className="w-full bg-[#0a141f] border border-white/10 focus:border-[#43e9b7] rounded-lg px-3.5 py-2.5 text-sm font-mono text-[#d9e3f3] placeholder:text-[#848E9C]/40 focus:outline-none transition-colors"
                  />
                </div>

                <button
                  type="submit"
                  className="w-full bg-[#43e9b7] hover:bg-[#35dfae] text-[#003829] font-bold text-sm py-3 rounded-lg transition-all flex items-center justify-center gap-2 active:scale-98"
                >
                  <span>Continue with Clerk</span>
                  <ArrowRight className="w-4 h-4" />
                </button>

                <div className="relative my-4">
                  <div className="absolute inset-0 flex items-center">
                    <div className="w-full border-t border-white/5"></div>
                  </div>
                  <div className="relative flex justify-center text-[10px] uppercase font-mono">
                    <span className="bg-[#161A1E] px-2 text-[#848E9C]">Or sign in with</span>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-2 font-mono text-xs">
                  <button
                    type="button"
                    onClick={() => handleSocialLogin('Google')}
                    className="flex items-center justify-center gap-2 bg-[#0a141f] hover:bg-[#2c3641] border border-white/10 p-2.5 rounded-lg text-[#d9e3f3] transition-colors"
                  >
                    <span>Google</span>
                  </button>

                  <button
                    type="button"
                    onClick={() => handleSocialLogin('GitHub')}
                    className="flex items-center justify-center gap-2 bg-[#0a141f] hover:bg-[#2c3641] border border-white/10 p-2.5 rounded-lg text-[#d9e3f3] transition-colors"
                  >
                    <span>GitHub</span>
                  </button>
                </div>
              </form>
            ) : (
              <form onSubmit={handleVerifyClerk} className="space-y-4">
                <div className="text-center">
                  <div className="text-xs text-[#848E9C] mb-1">We sent a verification code to</div>
                  <div className="text-sm font-mono font-semibold text-[#43e9b7]">{email}</div>
                </div>

                <div>
                  <label className="text-[11px] font-mono text-[#848E9C] block mb-1">
                    6-Digit Verification Code
                  </label>
                  <input
                    type="text"
                    value={verificationCode}
                    onChange={(e) => setVerificationCode(e.target.value)}
                    className="w-full bg-[#0a141f] border border-[#43e9b7] rounded-lg px-3.5 py-2.5 text-center text-xl tracking-widest font-mono text-[#d9e3f3] focus:outline-none"
                    maxLength={6}
                    required
                  />
                </div>

                <button
                  type="submit"
                  disabled={isVerifying}
                  className="w-full bg-[#43e9b7] hover:bg-[#35dfae] text-[#003829] font-bold text-sm py-3 rounded-lg transition-all flex items-center justify-center gap-2 disabled:opacity-50"
                >
                  {isVerifying ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      <span>Syncing with Prisma DB...</span>
                    </>
                  ) : (
                    <>
                      <span>Complete Sign In</span>
                      <CheckCircle2 className="w-4 h-4" />
                    </>
                  )}
                </button>

                <div className="text-center">
                  <button
                    type="button"
                    onClick={() => setCodeSent(false)}
                    className="text-xs font-mono text-[#848E9C] hover:text-[#d9e3f3] underline"
                  >
                    Use different email
                  </button>
                </div>
              </form>
            )}
          </div>
        )}

        {/* Web3 Wallets Tab */}
        {authTab === 'wallets' && (
          <div className="space-y-2.5 font-mono text-xs">
            
            {/* JoyID */}
            <button
              onClick={() => connectWallet('JoyID')}
              className="w-full bg-[#0a141f] hover:bg-[#2c3641] border border-white/10 hover:border-[#43e9b7]/40 p-3.5 rounded-xl flex items-center justify-between text-left transition-all group"
            >
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-[#43e9b7]/10 flex items-center justify-center text-[#43e9b7] font-bold">
                  J
                </div>
                <div>
                  <div className="font-bold text-[#d9e3f3] group-hover:text-[#43e9b7] flex items-center gap-1.5">
                    <span>JoyID Passkey</span>
                    <span className="text-[9px] bg-[#00CC9C]/15 text-[#00CC9C] px-1.5 py-0.2 rounded">Recommended</span>
                  </div>
                  <div className="text-[11px] text-[#848E9C]">Biometrics & TouchID for CKB</div>
                </div>
              </div>
              <ArrowRight className="w-4 h-4 text-[#848E9C] group-hover:text-[#43e9b7]" />
            </button>

            {/* UniSat */}
            <button
              onClick={() => connectWallet('UniSat')}
              className="w-full bg-[#0a141f] hover:bg-[#2c3641] border border-white/10 hover:border-[#F7931A]/40 p-3.5 rounded-xl flex items-center justify-between text-left transition-all group"
            >
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-[#F7931A]/10 flex items-center justify-center text-[#F7931A] font-bold">
                  ₿
                </div>
                <div>
                  <div className="font-bold text-[#d9e3f3] group-hover:text-[#F7931A]">UniSat Bitcoin Wallet</div>
                  <div className="text-[11px] text-[#848E9C]">Direct BTC UTXO signing</div>
                </div>
              </div>
              <ArrowRight className="w-4 h-4 text-[#848E9C] group-hover:text-[#F7931A]" />
            </button>

            {/* OKX */}
            <button
              onClick={() => connectWallet('OKX')}
              className="w-full bg-[#0a141f] hover:bg-[#2c3641] border border-white/10 hover:border-white/30 p-3.5 rounded-xl flex items-center justify-between text-left transition-all group"
            >
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-white/10 flex items-center justify-center text-white font-bold">
                  O
                </div>
                <div>
                  <div className="font-bold text-[#d9e3f3]">OKX Multi-Chain Wallet</div>
                  <div className="text-[11px] text-[#848E9C]">CKB & Taproot supported</div>
                </div>
              </div>
              <ArrowRight className="w-4 h-4 text-[#848E9C]" />
            </button>

            {/* Neuron */}
            <button
              onClick={() => connectWallet('Neuron')}
              className="w-full bg-[#0a141f] hover:bg-[#2c3641] border border-white/10 hover:border-white/30 p-3.5 rounded-xl flex items-center justify-between text-left transition-all group"
            >
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-[#35dfae]/10 flex items-center justify-center text-[#35dfae] font-bold">
                  N
                </div>
                <div>
                  <div className="font-bold text-[#d9e3f3]">Neuron CKB Node</div>
                  <div className="text-[11px] text-[#848E9C]">Native Full Node connection</div>
                </div>
              </div>
              <ArrowRight className="w-4 h-4 text-[#848E9C]" />
            </button>

          </div>
        )}

      </div>
    </div>
  );
};
