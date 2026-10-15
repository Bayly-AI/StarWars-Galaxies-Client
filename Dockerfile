# ==============================================================================
# Multi-Stage Dockerfile for Star Wars Galaxies Pre-CU Client
# Group: games | Target: swg-client
# ==============================================================================

# --- Stage 1: Build Environment ---
FROM rust:bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace source files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build release binary for swg-client-app
RUN cargo build --release --bin swg-client-app

# --- Stage 2: Runtime Environment ---
FROM debian:bookworm-slim AS runner

WORKDIR /app

# Install minimal runtime certificates
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create dedicated non-root service user
RUN groupadd -g 1000 swg && \
    useradd -u 1000 -g swg -s /bin/false -m swg

# Copy compiled client binary from builder stage
COPY --from=builder /build/target/release/swg-client-app /app/swg-client-app
RUN chown -R swg:swg /app

USER swg

EXPOSE 8080

# Built-in container health probe
HEALTHCHECK --interval=10s --timeout=5s --start-period=5s --retries=3 \
    CMD ["/app/swg-client-app", "--healthcheck"]

# Entrypoint
ENTRYPOINT ["/app/swg-client-app"]
