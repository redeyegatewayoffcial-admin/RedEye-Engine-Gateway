# RedEye Policy Engine

Enterprise AI Gateway for regulated industries — ultra-low-latency Rust reverse proxy
sitting between your applications and upstream LLM providers.

## Architecture

```
[Client App] → [RedEye Gateway :8080] → [OpenAI / Anthropic]
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
docker exec -it RedEye_postgres psql -U RedEye -d RedEye -c "\dt"

# 5. Check ClickHouse tables
docker exec -it RedEye_clickhouse \
  clickhouse-client -u RedEye --password clickhouse_secret \
  --query "SHOW TABLES FROM RedEye_telemetry"

# 6. Ping Redis
docker exec -it RedEye_redis redis-cli -a redis_secret ping
```

### Expected output
```
NAME                  STATUS
redeye_postgres      healthy
redeye_redis         healthy
redeye_clickhouse    healthy
```

### Tear down
```bash
docker compose down -v   # -v removes volumes (full reset)
```

## Build Phases

| Phase | Description                          | Status      |
|-------|--------------------------------------|-------------|
| 1     | Docker Compose (Postgres/Redis/CH)   | ✅ Complete |
| 2     | Rust Axum proxy → OpenAI             | ✅ Complete |
| 3     | Redis rate limiting                  | ✅ Complete |
| 4     | Async ClickHouse telemetry           | ✅ Complete |
| 5     | React dashboard                      | ✅ Complete |
