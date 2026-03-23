# 👁️ RedEye AI Engine

![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)
![Axum](https://img.shields.io/badge/Framework-Axum-blue.svg)
![Tokio](https://img.shields.io/badge/Async-Tokio-yellow.svg)
![PostgreSQL](https://img.shields.io/badge/DB-PostgreSQL-blue.svg)
![Redis](https://img.shields.io/badge/Cache-Redis-red.svg)
![ClickHouse](https://img.shields.io/badge/OLAP-ClickHouse-yellowgreen.svg)

**RedEye AI Engine** is a high-performance, ultra-low-latency AI API Gateway built in Rust. It serves as a unified, production-ready enterprise proxy for LLM providers (OpenAI, Gemini, Groq, Anthropic, etc.), offering zero-copy streaming, deep telemetry, compliance enforcement, and intelligent routing.

Designed to handle **10,000+ concurrent requests per second (RPS)** with sub-5ms internal latency overhead.

---

## 🚀 Key Features & Enterprise Optimizations

- **⚡ True Zero-Copy SSE Streaming:** Passes streaming text chunks from the upstream LLM directly to the client without buffering in memory, completely eliminating OOM (Out-Of-Memory) risks under heavy load.
- **📊 MPSC Telemetry Batching:** Telemetry and request logs are offloaded to an asynchronous Tokio `mpsc` channel. Background workers aggregate logs and flush them to ClickHouse via bulk HTTP requests (1000 items or 1s intervals), preventing TCP TIME_WAIT socket exhaustion.
- **🛡️ CPU-Offloaded PII Redaction:** Heavy regex compliance checks run inside `tokio::task::spawn_blocking`, ensuring the async runtime executor is never blocked during massive payload parsing.
- **🚥 Intelligent Circuit Breakers & Adaptive Routing:** Automatically detects upstream 5xx errors or timeouts. Temporarily opens the circuit and seamlessly reroutes traffic to fallback providers (e.g., Groq) with zero client-side interruptions.
- **⏳ Token-Bucket Rate Limiting:** Enforces strict per-tenant token consumption limits using atomic Lua scripts in Redis.
- **🧠 Semantic Caching:** (Via `redeye_cache`) Vector-based semantic similarity lookups to serve identical queries locally, reducing LLM API costs and improving response times.
- **🏗️ Clean Architecture:** Strictly modularized domain, usecases, and infrastructure layers within a Rust Cargo Workspace.

## 🏗️ Architecture Flow

1. **Client Request** → Authenticated via JWT/API Key (Postgres → Redis cached).
2. **Compliance Layer** → Real-time PII detection and fail-closed redaction.
3. **Semantic Cache** → Checks for semantically identical previous responses.
4. **Rate Limiter** → Decrements Redis token bucket based on estimated payload size.
5. **Circuit Breaker** → Routes to Primary LLM (or Fallback if Primary is degraded).
6. **Streaming Proxy** → Streams SSE back to client instantly.
7. **Async Telemetry** → Non-blocking logs sent to ClickHouse & Tracer via `mpsc` batches.

## 📦 Project Structure (Cargo Workspace)

This repository is structured as a microservices workspace:

```text
.
├── redeye_gateway/      # Core API Gateway (Routing, Stream Proxy, Auth, Telemetry)
├── redeye_cache/        # Semantic Caching service (Vector Embeddings)
├── redeye_compliance/   # Advanced Data Privacy, OPA, and PII Engine
├── redeye_tracer/       # Deep trace ingestion and session analytics
├── redeye_auth/         # Tenant Identity & Key Management
└── redeye_dashboard/    # React/Tauri based Admin Dashboard
```

## 🛠️ Getting Started

### Prerequisites
- **Rust** (1.70+)
- **Docker** & **Docker Compose**
- **Node.js** (for dashboard UI)

### 1. Start Infrastructure (Databases)
The engine relies on Postgres, Redis, and ClickHouse. Start them using the provided compose file:
```bash
docker compose up -d
```

### 2. Configure Environment Variables
Copy `.env.example` to `.env` and fill in your credentials:
```env
GATEWAY_PORT=8080
DATABASE_URL=postgres://user:pass@localhost:5432/redeye
REDIS_URL=redis://localhost:6379/0
CLICKHOUSE_URL=http://localhost:8123
CACHE_URL=http://localhost:8081
COMPLIANCE_URL=http://localhost:8083
TRACER_URL=http://localhost:8082
```

### 3. Run the Gateway (Release Mode Recommended)
*Note: Due to CPU-intensive regex and async routing, always run the gateway in `--release` mode for performance testing (sub-5ms latency).*

```bash
cargo run --release -p redeye_gateway
```

### 4. Test the Endpoint
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer <YOUR_TENANT_API_KEY>" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama-3.3-70b-versatile",
    "messages": [{"role": "user", "content": "Hello, world!"}],
    "stream": true
  }'
```

## 🧪 Testing

The codebase includes robust unit and async integration tests using Rust's native `#[tokio::test]`. 

To run the gateway tests (e.g., verifying MPSC channels and Regex behavior):
```bash
cargo test -p redeye_gateway
```

## 📄 License

This project is dual-licensed under the [MIT License](LICENSE-MIT) and [Apache 2.0 License](LICENSE-APACHE). Enterprise compliance and semantic caching features may require commercial licensing.

---
*Built for the future of reliable, massive-scale AI systems.*
```
