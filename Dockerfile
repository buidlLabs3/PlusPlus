# ++ DEX Server — Multi-stage Docker build
# Stage 1a: Build the Next.js frontend
FROM node:20-slim AS frontend

WORKDIR /app/web
COPY web/package.json web/bun.lock* ./
RUN npm install --legacy-peer-deps
COPY web/ ./
RUN npm run build

# Stage 1b: Build the Rust backend
FROM rust:1.90-slim AS backend

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace manifests for layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/offer-protocol/Cargo.toml crates/offer-protocol/Cargo.toml
COPY crates/swap-executor/Cargo.toml crates/swap-executor/Cargo.toml
COPY crates/channel-manager/Cargo.toml crates/channel-manager/Cargo.toml
COPY crates/router/Cargo.toml crates/router/Cargo.toml
COPY crates/settlement/Cargo.toml crates/settlement/Cargo.toml
COPY crates/incentives/Cargo.toml crates/incentives/Cargo.toml
COPY crates/indexer/Cargo.toml crates/indexer/Cargo.toml
COPY crates/fiber-client/Cargo.toml crates/fiber-client/Cargo.toml
COPY crates/plusplus-cli/Cargo.toml crates/plusplus-cli/Cargo.toml
COPY crates/plusplus-server/Cargo.toml crates/plusplus-server/Cargo.toml

# Create dummy sources for dependency caching
RUN mkdir -p crates/offer-protocol/src crates/swap-executor/src crates/channel-manager/src \
    crates/router/src crates/settlement/src crates/incentives/src \
    crates/indexer/src crates/fiber-client/src \
    crates/plusplus-cli/src crates/plusplus-server/src && \
    echo 'pub fn dummy() {}' > crates/offer-protocol/src/lib.rs && \
    echo 'pub fn dummy() {}' > crates/swap-executor/src/lib.rs && \
    echo 'pub fn dummy() {}' > crates/channel-manager/src/lib.rs && \
    echo 'pub fn dummy() {}' > crates/router/src/lib.rs && \
    echo 'pub fn dummy() {}' > crates/settlement/src/lib.rs && \
    echo 'pub fn dummy() {}' > crates/incentives/src/lib.rs && \
    echo 'pub fn dummy() {}' > crates/indexer/src/lib.rs && \
    echo 'pub fn dummy() {}' > crates/fiber-client/src/lib.rs && \
    echo 'fn main() {}' > crates/plusplus-cli/src/main.rs && \
    echo 'fn main() {}' > crates/plusplus-server/src/main.rs

RUN cargo build --release --bin plusplus-server 2>/dev/null || true

# Copy real source code
COPY crates/ crates/
RUN find crates/ -name "*.rs" -exec touch {} +

# Build the real binary
RUN cargo build --release --bin plusplus-server

# Stage 2: Runtime image
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r plusplus && useradd -r -g plusplus plusplus

WORKDIR /app

# Copy the Rust binary
COPY --from=backend /app/target/release/plusplus-server /app/plusplus-server

# Copy the Next.js static export
COPY --from=frontend /app/web/out /app/web

# Create data directory
RUN mkdir -p /app/data && chown -R plusplus:plusplus /app

ENV PLUSPLUS_DB=/app/data/plusplus.db
ENV PLUSPLUS_WEB_DIR=/app/web
ENV RUST_LOG=info

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -f http://localhost:3000/info || exit 1

EXPOSE 3000

USER plusplus

CMD ["/app/plusplus-server"]
