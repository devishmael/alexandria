# ==============================================================================
# Alexandria Deterministic Kernel — Multi-Stage Production Container
# ==============================================================================

# Stage 1: Build the Rust kernel binary
FROM rust:1.85-bullseye AS builder

WORKDIR /usr/src/alexandria

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY tests/ ./tests/

# Compile with maximum release optimizations
RUN cargo build --release --locked

# Stage 2: Minimal runtime image with Node.js for sandboxed TDD execution
FROM node:20-bullseye-slim

LABEL maintainer="Devi Ishmael"
LABEL org.opencontainers.image.title="Alexandria Kernel"
LABEL org.opencontainers.image.description="Deterministic Agent Harness & Kernel-Space Arbitrator"
LABEL org.opencontainers.image.licenses="MIT"

WORKDIR /workspace

# Install ca-certificates and git
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled Rust binary from builder stage
COPY --from=builder /usr/src/alexandria/target/release/alexandria /usr/local/bin/alexandria

# Expose standard workspace mount
VOLUME ["/workspace"]

# Default entrypoint: Alexandria MCP server over stdio
ENTRYPOINT ["alexandria"]
CMD ["mcp"]
