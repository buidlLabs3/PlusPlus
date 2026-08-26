# ++ DEX Server — Multi-stage Docker build
# Stage 1: Build the Rust workspace
FROM rust:1.85-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace manifests first (for Docker layer caching)
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

# Create dummy source files to satisfy cargo check (for layer caching)
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

# Build dependencies only (cached layer)
RUN cargo build --release --bin plusplus-server 2>/dev/null || true

# Copy actual source code
COPY crates/ crates/

# Build the server binary
RUN cargo build --release --bin plusplus-server

# Stage 2: Runtime image
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r plusplus && useradd -r -g plusplus plusplus

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/target/release/plusplus-server /app/plusplus-server

# Copy web UI
COPY web/ /app/web/

# Copy deployment config
COPY deploy/ /app/deploy/

# Create data directory
RUN mkdir -p /app/data && chown -R plusplus:plusplus /app

# Environment
ENV PLUSPLUS_DB=/app/data/plusplus.db
ENV RUST_LOG=info

EXPOSE 3000

USER plusplus

CMD ["/app/plusplus-server"]
