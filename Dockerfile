# -------- Build stage --------
FROM rustlang/rust:nightly AS builder

WORKDIR /usr/src/app
COPY . .

# Build the release binary
RUN cargo build --release

# -------- Runtime stage --------
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the compiled binary
COPY --from=builder /usr/src/app/target/release/redeem-rs /usr/local/bin/redeem-rs

WORKDIR /usr/local/bin

# Set the binary as the entrypoint
ENTRYPOINT ["redeem-rs"]