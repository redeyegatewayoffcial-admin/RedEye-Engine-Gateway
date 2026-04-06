# Fix CORS Credentials Header

## Summary
Add `.allow_credentials(true)` to CORS configuration in `redeye_tracer`, `redeye_compliance`, and `redeye_gateway` services to support credentialed requests from the frontend.

## Problem
The frontend sends requests with `credentials: 'include'` for HttpOnly cookie support. However, three backend services are missing `.allow_credentials(true)` in their CORS layer, causing the browser to block requests with this error:

> "A cross-origin resource sharing (CORS) request was blocked because it was configured to include credentials but the Access-Control-Allow-Credentials response header was not set to true."

## Affected Services

| Service | File | Has `allow_credentials(true)` |
|---------|------|------------------------------|
| redeye_auth | `src/api/router.rs:56` | ✓ Yes |
| redeye_tracer | `src/main.rs:86-90` | ✗ **Missing** |
| redeye_compliance | `src/main.rs:100-104` | ✗ **Missing** |
| redeye_gateway | `src/api/routes.rs:85-89` | ✗ **Missing** |

## Required Changes

### 1. redeye_tracer/src/main.rs (line 86-90)
```rust
// BEFORE:
CorsLayer::new()
    .allow_origin(origins)
    .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])

// AFTER:
CorsLayer::new()
    .allow_origin(origins)
    .allow_credentials(true)  // <-- ADD THIS
    .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])
```

### 2. redeye_compliance/src/main.rs (line 100-104)
```rust
// BEFORE:
CorsLayer::new()
    .allow_origin(origins)
    .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])

// AFTER:
CorsLayer::new()
    .allow_origin(origins)
    .allow_credentials(true)  // <-- ADD THIS
    .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])
```

### 3. redeye_gateway/src/api/routes.rs (line 85-89)
```rust
// BEFORE:
CorsLayer::new()
    .allow_origin(origins)
    .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])

// AFTER:
CorsLayer::new()
    .allow_origin(origins)
    .allow_credentials(true)  // <-- ADD THIS
    .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])
```

## Security Notes
- All services already use specific origins (not wildcard `*`) via `DASHBOARD_URL` env var or localhost fallbacks
- `allow_credentials(true)` is safe because origins are explicitly whitelisted
- This matches the frontend's `credentials: 'include'` setting already implemented

## Additional Issue
The tracer service also needs `CLICKHOUSE_URL` environment variable. Add to `.env`:
```
CLICKHOUSE_URL=http://localhost:8123
```

## Verification
After changes, verify with:
```bash
cargo check --all
```

All 71 blocked CORS requests should now succeed.
