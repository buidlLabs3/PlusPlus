'use client';

import React from 'react';
import { ActiveTab } from '@/src/types';

interface FooterProps {
  setActiveTab: (tab: ActiveTab) => void;
}

export const Footer: React.FC<FooterProps> = ({ setActiveTab }) => {
  return (
    <footer className="w-full border-t border-white/5 bg-[#0B0E11] text-xs font-mono text-[#848E9C] py-8">
      <div className="max-w-[1440px] mx-auto px-4 md:px-12 flex flex-col md:flex-row items-center justify-between gap-4">
        
        {/* Left copyright notice matching Image 1 */}
        <div>
          © 2024 PlusPlus DEX — Built on <span className="text-[#43e9b7]">Nervos CKB</span> · Powered by <span className="text-[#F7931A]">Fiber Network</span> · <span className="text-[#00CC9C]">RGB++ Protocol</span>
        </div>

        {/* Right Links */}
        <div className="flex items-center gap-6">
          <button onClick={() => setActiveTab('docs')} className="hover:text-[#d9e3f3] transition-colors">
            Documentation
          </button>
          <a
            href="https://github.com/nervosnetwork"
            target="_blank"
            rel="noreferrer"
            className="hover:text-[#d9e3f3] transition-colors"
          >
            GitHub
          </a>
          <a
            href="https://discord.gg/nervosnetwork"
            target="_blank"
            rel="noreferrer"
            className="hover:text-[#d9e3f3] transition-colors"
          >
            Discord
          </a>
          <span className="text-white/20">|</span>
          <span className="text-[#00CC9C] flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-[#00CC9C] animate-pulse"></span>
            <span>All Systems Operational</span>
          </span>
        </div>

      </div>
    </footer>
  );
};
