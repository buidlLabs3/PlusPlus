import type { Metadata } from 'next';
import './globals.css';
import { AuthProvider } from '@/src/context/AuthContext';

export const metadata: Metadata = {
  title: 'PlusPlus DEX | RGB++ Protocol on CKB & Bitcoin Fiber Network',
  description: 'Decentralized exchange for RGB++ assets on Nervos CKB and Bitcoin Fiber Network.',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <body className="bg-[#0B0E11] text-[#d9e3f3] min-h-screen antialiased">
        <AuthProvider>{children}</AuthProvider>
      </body>
    </html>
  );
}
