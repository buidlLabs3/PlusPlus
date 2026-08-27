// Prisma Client singleton instance (only used when running server-side)
// For static export builds, this module is tree-shaken out

let prisma: any = null;

if (typeof window === 'undefined') {
  // Server-side only
  try {
    const { PrismaClient } = require('@prisma/client');
    const globalForPrisma = globalThis as unknown as { prisma: any | undefined };
    prisma = globalForPrisma.prisma ?? new PrismaClient({
      log: process.env.NODE_ENV === 'development' ? ['query', 'error', 'warn'] : ['error'],
    });
    if (process.env.NODE_ENV !== 'production') globalForPrisma.prisma = prisma;
  } catch {
    // Prisma not available — running in static export mode
    prisma = null;
  }
}

export { prisma };
