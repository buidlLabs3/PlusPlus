'use client';

import React, { useState } from 'react';
import { X, Check, Globe, Zap, Database, ShieldCheck } from 'lucide-react';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({ isOpen, onClose }) => {
  const [network, setNetwork] = useState<string>('testnet');
  const [rpcUrl, setRpcUrl] = useState<string>('https://testnet.ckbapp.dev');
  const [fiberUrl, setFiberUrl] = useState<string>('https://fiber-node-1.nervos.org/rpc');
  const [saved, setSaved] = useState(false);

  if (!isOpen) return null;

  const handleSave = (e: React.FormEvent) => {
    e.preventDefault();
    setSaved(true);
    setTimeout(() => {
      setSaved(false);
      onClose();
    }, 500);
  };

  return (
    <div className="fixed inset-0 bg-black/80 backdrop-blur-md flex items-center justify-center z-50 p-4">
      <div className="bg-[#161A1E] border border-white/10 rounded-xl max-w-md w-full p-6 shadow-2xl space-y-5">
        
        <div className="flex justify-between items-center pb-3 border-b border-white/5">
          <div className="flex items-center gap-2">
            <Globe className="w-5 h-5 text-[#43e9b7]" />
            <h3 className="font-bold text-base text-[#d9e3f3]">Network & RPC Configuration</h3>
          </div>
          <button
            onClick={onClose}
            className="text-[#848E9C] hover:text-[#d9e3f3] p-1 rounded"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSave} className="space-y-4 font-mono text-xs">
          <div>
            <label className="text-[#848E9C] block mb-1">Target Network</label>
            <div className="grid grid-cols-3 gap-2">
              {['testnet', 'devnet', 'mainnet'].map((net) => (
                <button
                  type="button"
                  key={net}
                  onClick={() => setNetwork(net)}
                  className={`p-2 rounded border uppercase text-center transition-all ${
                    network === net
                      ? 'bg-[#43e9b7]/15 border-[#43e9b7] text-[#43e9b7] font-bold'
                      : 'border-white/10 text-[#848E9C] hover:text-[#d9e3f3]'
                  }`}
                >
                  {net}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="text-[#848E9C] block mb-1">CKB Node RPC Endpoint</label>
            <input
              type="text"
              value={rpcUrl}
              onChange={(e) => setRpcUrl(e.target.value)}
              className="w-full bg-[#0a141f] border border-white/10 focus:border-[#43e9b7] rounded px-3 py-2 text-[#d9e3f3] focus:outline-none"
            />
          </div>

          <div>
            <label className="text-[#848E9C] block mb-1">Fiber Channel Gateway</label>
            <input
              type="text"
              value={fiberUrl}
              onChange={(e) => setFiberUrl(e.target.value)}
              className="w-full bg-[#0a141f] border border-white/10 focus:border-[#43e9b7] rounded px-3 py-2 text-[#d9e3f3] focus:outline-none"
            />
          </div>

          <div className="bg-[#0a141f] rounded p-3 text-[11px] space-y-1 text-[#848E9C]">
            <div className="flex justify-between">
              <span>Database Layer</span>
              <span className="text-[#43e9b7]">Prisma ORM + In-Memory Store</span>
            </div>
            <div className="flex justify-between">
              <span>Auth Service</span>
              <span className="text-[#00CC9C]">Clerk React SDK (Universal)</span>
            </div>
            <div className="flex justify-between">
              <span>Covenant Smart Contract</span>
              <span className="text-[#d9e3f3]">Type ID Script v1.4</span>
            </div>
          </div>

          <button
            type="submit"
            className="w-full bg-[#43e9b7] hover:bg-[#35dfae] text-[#003829] font-bold py-3 rounded flex items-center justify-center gap-2 transition-colors"
          >
            {saved ? (
              <>
                <Check className="w-4 h-4" />
                <span>Saved & Synchronized</span>
              </>
            ) : (
              <span>Apply Configuration</span>
            )}
          </button>
        </form>

      </div>
    </div>
  );
};
