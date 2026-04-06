# Update Gateway Auth Middleware for Cookie-Based JWT

## Summary
Extend the gateway auth middleware to extract JWT from the `auth_token` HttpOnly cookie when the `Authorization: Bearer` header is missing.

## Current Behavior
The middleware currently only checks:
1. `Authorization: Bearer <token>` header for JWT
2. `x-api-key: <key>` header for API keys
3. Returns 401 if neither is present

## Required Changes

### Cookie Names from Phase 3
- **`auth_token`** - Contains the JWT (HttpOnly, Secure, SameSite=Strict)
- **`refresh_token`** - Contains the refresh token (not a JWT, used for /refresh endpoint)

### Implementation Plan

Modify `redeye_gateway/src/api/middleware/auth.rs`:

1. **Add cookie parsing logic** after Authorization header check (around line 56)
2. **Extract `auth_token` cookie** if no Bearer token found
3. **Use same JWT validation** as Bearer tokens

### Code Changes

```rust
// In auth_middleware function, after checking x-api-key (around line 66)
// Add cookie fallback:

if token_opt.is_none() {
    // Check for auth_token cookie as fallback
    if let Some(cookie_header) = req.headers().get(axum::http::header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            // Parse cookies and look for auth_token
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(value) = cookie.strip_prefix("auth_token=") {
                    token_opt = Some((value.to_string(), false));
                    break;
                }
            }
        }
    }
}
```

### Detailed Steps

1. **Add COOKIE header import** if not already present:
   ```rust
   use axum::http::header;
   ```

2. **Insert cookie check logic** after the x-api-key check (line 66), before the `match token_opt` block:
   
   ```rust
   // 4. Cookie Fallback (for HttpOnly auth_token cookie)
   if token_opt.is_none() {
       if let Some(cookie_header) = req.headers().get(header::COOKIE) {
           if let Ok(cookie_str) = cookie_header.to_str() {
               // Look for auth_token in cookies
               for cookie in cookie_str.split(';') {
                   let cookie = cookie.trim();
                   if let Some(token) = cookie.strip_prefix("auth_token=") {
                       // Validate it looks like a JWT (3 base64url parts separated by dots)
                       if token.split('.').count() == 3 {
                           token_opt = Some((token.to_string(), false));
                           tracing::debug!("JWT extracted from auth_token cookie");
                           break;
                       }
                   }
               }
           }
       }
   }
   ```

3. **JWT validation remains the same** - the `handle_jwt` function will validate the token the same way regardless of whether it came from header or cookie.

### Security Considerations

1. **No security downgrade**: Cookie-based auth is equally secure since:
   - `auth_token` is HttpOnly (XSS protection)
   - `auth_token` is Secure (HTTPS-only)
   - `auth_token` is SameSite=Strict (CSRF protection)
   - Same JWT validation logic is used

2. **Token format validation**: Check that cookie value is a valid JWT format (3 base64url parts) before accepting it

3. **Logging**: Add debug log when token is extracted from cookie (don't log the token itself)

### Testing Steps

1. Start gateway service
2. Login via auth service (which sets `auth_token` cookie)
3. Make request to gateway WITHOUT Authorization header but WITH the cookie
4. Request should succeed (200) instead of 401

### Fallback Priority

1. Authorization: Bearer (JWT) - highest priority
2. x-api-key (API key)
3. auth_token cookie (JWT) - NEW fallback
4. Return 401 if all fail

### Files to Modify

- `redeye_gateway/src/api/middleware/auth.rs`
  - Add cookie extraction logic (lines 66-75 approximate)
  - No changes needed to `handle_jwt` or `verify_jwt` functions

### Verification

```bash
cargo check -p redeye_gateway
```

Expected: No compilation errors, same behavior as before for header-based auth, new cookie-based auth works as fallback.
