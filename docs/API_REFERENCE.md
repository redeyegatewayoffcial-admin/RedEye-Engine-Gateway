# Enterprise API Reference

## 1. RedEye Gateway
Entry point proxy and telemetry service.

### `POST /v1/chat/completions`
- **Description:** Proxy an LLM chat completion request to the downstream model (with caching, routing, filtering).
- **Authentication:** `Authorization: Bearer <token>`
- **Headers:** `x-tenant-id`, `x-redeye-routing-strategy` (optional)
- **Request Body:**
```json
{
  "model": "meta-llama/llama-3.3-70b-instruct",
  "messages": [
    {
      "role": "user",
      "content": "Hello world"
    }
  ]
}
```
- **Response (200 OK):**
```json
{
  "id": "chatcmpl-123",
  "choices": [
    {
       "message": { "role": "assistant", "content": "Hello there!" }
    }
  ]
}
```

### `GET /health`
- **Description:** Readiness probe.
- **Authentication:** None
- **Response (200 OK):**
```json
{ "status": "ok", "service": "redeye_gateway", "version": "1.0.0" }
```

### `GET /v1/admin/metrics`
- **Description:** Live gateway usage metrics summary.
- **Authentication:** `Authorization: Bearer <token>`
- **Response (200 OK):**
```json
{
  "total_requests": 1000,
  "avg_latency_ms": 320.5,
  "total_tokens": 150000,
  "rate_limited_requests": 10,
  "traffic_series": [],
  "model_distribution": [],
  "latency_buckets": []
}
```

(Other available admin metrics endpoints: `/v1/admin/metrics/usage`, `/v1/admin/metrics/cache`, `/v1/admin/metrics/compliance`, `/v1/admin/metrics/hot-swaps`, `/v1/admin/billing/breakdown`, `/v1/admin/traces`, `/v1/admin/security/alerts`).

---

## 2. RedEye Auth
Identity and API Key management.

### `POST /v1/auth/signup`
- **Description:** Register a new workspace tenant and user.
- **Authentication:** None
- **Request Body:**
```json
{
  "email": "user@example.com",
  "password": "SecurePassword123!",
  "company_name": "Acme Corp"
}
```
- **Response (200 OK):**
```json
{
  "id": "uuid",
  "email": "user@example.com",
  "tenant_id": "uuid",
  "workspace_name": "Acme Corp",
  "onboarding_complete": false,
  "token": "jwt..."
}
```

### `POST /v1/auth/login`
- **Description:** Authenticate user with password.
- **Authentication:** None
- **Request Body:**
```json
{
  "email": "user@example.com",
  "password": "SecurePassword123!"
}
```
- **Response (200 OK):** (Same as Signup)

### `POST /v1/auth/refresh`
- **Description:** Refresh JWT access token.
- **Authentication:** Cookie `refresh_token`
- **Response (200 OK):** (New AuthResponse + updated Set-Cookie headers)

### `POST /v1/auth/onboard`
- **Description:** Setup LLM provider keys and generate virtual RedEye key.
- **Authentication:** `Authorization: Bearer <token>`
- **Request Body:**
```json
{
  "account_type": "individual",
  "provider": "openai",
  "api_key": "sk-...",
  "workspace_name": "Optional Workspace Name"
}
```
- **Response (200 OK):** Includes the generated virtual `redeye_api_key`.

### `GET /v1/auth/api-keys`
- **Description:** List virtual API keys for the workspace.
- **Authentication:** `Authorization: Bearer <token>`

### `POST /v1/auth/provider-keys`
- **Description:** Add/Update an upstream provider API key (e.g. OpenAI).
- **Authentication:** `Authorization: Bearer <token>`
- **Request Body:**
```json
{
  "provider_name": "openai",
  "provider_api_key": "sk-..."
}
```

### `POST /v1/auth/request-otp`
- **Description:** Send a one-time magic link/code.
- **Authentication:** None
- **Request Body:** `{ "email": "user@example.com" }`

### `POST /v1/auth/verify-otp`
- **Description:** Verify the OTP.
- **Authentication:** None
- **Request Body:** `{ "email": "user@example.com", "otp_code": "123456" }`

---

## 3. RedEye Tracer
Auditing and observability ingest & query.

### `POST /v1/traces/ingest`
- **Description:** Ingest trace payload from gateway.
- **Authentication:** Private Network
- **Request Body:**
```json
{
  "trace_id": "uuid",
  "session_id": "session-123",
  "tenant_id": "tenant-uuid",
  "model": "gpt-4",
  "status": 200,
  "latency_ms": 150,
  "total_tokens": 50,
  "cache_hit": false,
  "prompt_content": "...",
  "response_content": "..."
}
```
- **Response (200 OK):** `{ "ingested": true }`

### `GET /v1/traces`
- **Description:** Query traces by session.
- **Authentication:** Internal / Admin JWT
- **Query Params:** `?session_id=123&limit=50`

### `GET /v1/audit`
- **Description:** Query compliance audit logs by tenant.
- **Authentication:** Internal / Admin JWT
- **Query Params:** `?tenant_id=uuid&limit=50`

---

## 4. RedEye Compliance
Privacy and entity redaction engine.

### `POST /v1/compliance/redact`
- **Description:** Applies PII redaction rules to a given prompt payload.
- **Authentication:** Internal
- **Request Body:**
```json
{
  "payload": {
    "text": "My phone number is 555-1234"
  }
}
```
- **Response (200 OK):**
```json
{
  "sanitized_payload": { "text": "My phone number is [REDACTED_PHONE]" },
  "mapping_stored": true,
  "redacted_count": 1,
  "token_map": {}
}
```

---

## 5. RedEye Cache
Semantic Cache.

### `POST /v1/cache/lookup`
- **Description:** REST fallback to check cache.
- **Request Body:**
```json
{
  "tenant_id": "uuid",
  "model": "gpt-4",
  "prompt": "Capital of France?"
}
```

### `POST /v1/cache/store`
- **Description:** Store completion.
- **Request Body:**
```json
{
  "tenant_id": "uuid",
  "model": "gpt-4",
  "prompt": "Capital of France?",
  "response_content": "Paris"
}
```
