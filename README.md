# NexusAI Policy Engine

Enterprise AI Gateway for regulated industries — ultra-low-latency Rust reverse proxy
sitting between your applications and upstream LLM providers.

## Architecture

```
[Client App] → [NexusAI Gateway :8080] → [OpenAI / Anthropic]
                      │
           ┌──────────┼──────────┐
           ▼          ▼          ▼
        [Redis]   [Postgres]  [ClickHouse]
      Rate Limit   Config/Keys  Telemetry
```

## Phase 1 — Quick Start (Infrastructure Only)

### Prerequisites
- Docker ≥ 24.x
- Docker Compose ≥ 2.x

### Spin up the stack

```bash
# 1. Copy environment template
cp .env.example .env

# 2. Start all infrastructure services
docker compose up -d

# 3. Verify all services are healthy
docker compose ps

# 4. Check Postgres schema
docker exec -it nexusai_postgres psql -U nexusai -d nexusai -c "\dt"

# 5. Check ClickHouse tables
docker exec -it nexusai_clickhouse \
  clickhouse-client -u nexusai --password clickhouse_secret \
  --query "SHOW TABLES FROM nexusai_telemetry"

# 6. Ping Redis
docker exec -it nexusai_redis redis-cli -a redis_secret ping
```

### Expected output
```
NAME                  STATUS
nexusai_postgres      healthy
nexusai_redis         healthy
nexusai_clickhouse    healthy
```

### Tear down
```bash
docker compose down -v   # -v removes volumes (full reset)
```

## Build Phases

| Phase | Description                          | Status      |
|-------|--------------------------------------|-------------|
| 1     | Docker Compose (Postgres/Redis/CH)   | ✅ Complete  |
| 2     | Rust Axum proxy → OpenAI             | 🔜 Next      |
| 3     | Redis rate limiting                  | ⏳ Planned   |
| 4     | Async ClickHouse telemetry           | ⏳ Planned   |
| 5     | React dashboard                      | ⏳ Planned   |
