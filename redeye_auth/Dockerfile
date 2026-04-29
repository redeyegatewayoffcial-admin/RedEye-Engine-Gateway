# # Build Stage
# FROM rustlang/rust:nightly-slim as builder

# WORKDIR /app

# # Install system dependencies
# RUN apt-get update && apt-get install -y pkg-config libssl-dev protobuf-compiler && rm -rf /var/lib/apt/lists/*

# # Copy workspace configuration
# COPY Cargo.toml Cargo.lock ./
# COPY redeye_auth/Cargo.toml ./redeye_auth/
# COPY redeye_gateway/Cargo.toml ./redeye_gateway/
# COPY redeye_cache/Cargo.toml ./redeye_cache/
# COPY redeye_tracer/Cargo.toml ./redeye_tracer/
# COPY redeye_compliance/Cargo.toml ./redeye_compliance/
# COPY redeye_config/Cargo.toml ./redeye_config/

# # Create dummy source to cache dependencies
# # 2. Create dummy source for ALL members to satisfy Cargo workspace parser
# RUN mkdir -p redeye_auth/src && echo "fn main() {}" > redeye_auth/src/main.rs
# RUN mkdir -p redeye_gateway/src && echo "fn main() {}" > redeye_gateway/src/main.rs
# RUN mkdir -p redeye_cache/src && echo "fn main() {}" > redeye_cache/src/main.rs
# RUN mkdir -p redeye_tracer/src && echo "fn main() {}" > redeye_tracer/src/main.rs
# RUN mkdir -p redeye_compliance/src && echo "fn main() {}" > redeye_compliance/src/main.rs

# RUN mkdir -p redeye_config/src && touch redeye_config/src/lib.rs

# # Copy actual source
# COPY redeye_auth/src ./redeye_auth/src
# COPY redeye_auth/migrations ./redeye_auth/migrations

# # Rebuild with actual source
# RUN touch redeye_auth/src/main.rs && cargo build --release --package redeye_auth

# # Runtime Stage
# FROM debian:bookworm-slim

# WORKDIR /app

# RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# COPY --from=builder /app/target/release/redeye_auth /usr/local/bin/

# EXPOSE 8084

# CMD ["redeye_auth"]
