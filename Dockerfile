# ==============================================================================
# GXE-Search-Queue — multi-stage Rust build
# ==============================================================================

# ---- Builder stage -------------------------------------------------------
FROM rust:1-slim-bookworm AS builder

WORKDIR /app

# Cache dependencies: copy manifests first, then vendor/build deps.
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
COPY build.rs ./

RUN cargo build --release

# ---- Runtime stage -------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# CA certificates for HTTPS and curl for healthchecks
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Non-root user
RUN groupadd -r gxe && useradd -r -g gxe gxe

# Copy the compiled binary (adjust the path to match the crate name)
COPY --from=builder /app/target/release/gxe-search-queue /usr/local/bin/gxe-search-queue

# Configuration directory (mount configs here if needed)
COPY config/ ./config/

USER gxe

EXPOSE 8080 8081

# Entry point can be overridden with, e.g., specific subcommands
ENTRYPOINT ["gxe-search-queue"]
CMD ["serve"]
