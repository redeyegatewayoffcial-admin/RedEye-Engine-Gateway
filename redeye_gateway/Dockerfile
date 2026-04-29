# # Build Stage
# FROM rustlang/rust:nightly-slim as builder

# WORKDIR /app

# # Install system dependencies
# RUN apt-get update && apt-get install -y \
#     pkg-config \
#     libssl-dev \
#     protobuf-compiler \
#     g++ \
#     && rm -rf /var/lib/apt/lists/*

# # Copy workspace configuration
# COPY Cargo.toml Cargo.lock ./
# COPY redeye_gateway/Cargo.toml ./redeye_gateway/
# # Copy other internal dependencies if any (workspace members)
# COPY redeye_cache/Cargo.toml ./redeye_cache/
# COPY redeye_tracer/Cargo.toml ./redeye_tracer/
# COPY redeye_compliance/Cargo.toml ./redeye_compliance/
# COPY redeye_auth/Cargo.toml ./redeye_auth/
# COPY redeye_config/Cargo.toml ./redeye_config/


# COPY proto/ ./proto/
# COPY redeye_gateway/build.rs ./redeye_gateway/
# COPY redeye_cache/build.rs ./redeye_cache/

# # Create dummy sources to cache dependencies
# RUN mkdir -p redeye_gateway/src && echo "fn main() {}" > redeye_gateway/src/main.rs
# RUN mkdir -p redeye_cache/src && echo "fn main() {}" > redeye_cache/src/lib.rs
# RUN mkdir -p redeye_tracer/src && echo "fn main() {}" > redeye_tracer/src/lib.rs
# RUN mkdir -p redeye_compliance/src && echo "fn main() {}" > redeye_compliance/src/lib.rs
# RUN mkdir -p redeye_auth/src && echo "fn main() {}" > redeye_auth/src/lib.rs
# RUN mkdir -p redeye_config/src && echo "fn main() {}" > redeye_config/src/lib.rs

# RUN cargo build --release --package redeye_gateway

# # Copy actual source
# COPY redeye_gateway/src ./redeye_gateway/src
# COPY redeye_gateway/migrations ./redeye_gateway/migrations
# COPY redeye_cache/src ./redeye_cache/src
# COPY redeye_tracer/src ./redeye_tracer/src
# COPY redeye_compliance/src ./redeye_compliance/src
# COPY redeye_auth/src ./redeye_auth/src
# COPY redeye_config/src ./redeye_config/src

# # Rebuild with actual source
# RUN touch redeye_gateway/src/main.rs && cargo build --release --package redeye_gateway

# # Runtime Stage
# FROM debian:bookworm-slim

# WORKDIR /app

# RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# COPY --from=builder /app/target/release/redeye_gateway /usr/local/bin/

# EXPOSE 8080

# CMD ["redeye_gateway"]
