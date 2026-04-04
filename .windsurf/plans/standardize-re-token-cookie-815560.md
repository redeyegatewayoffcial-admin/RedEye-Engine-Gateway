# Standardize JWT Cookie as `re_token` Across All Auth Methods

## Summary
Ensure all authentication methods (Google SSO, GitHub SSO, email/password login, OTP login) consistently set the JWT as an HttpOnly cookie named `re_token` with Secure and SameSite=Lax attributes before redirecting to the frontend.

## Current State Analysis

### Cookie Names Currently Used
- **`auth_token`** - Used in Google callback, GitHub callback, and refresh endpoint
- **`refresh_token`** - Used consistently across all methods for refresh tokens
- **`re_token`** - NOT currently used (this is what we need to implement)

### SameSite Settings Currently Used
- **Google/GitHub callbacks**: `SameSite=Strict`
- **Standard login/OTP**: Only sets refresh_token with `SameSite=Strict`
- **Refresh endpoint**: `SameSite=Strict`

### What's Missing
| Auth Method | Sets JWT Cookie? | Cookie Name | SameSite |
|-------------|-----------------|-------------|----------|
| Google callback | ✓ Yes | `auth_token` | Strict |
| GitHub callback | ✓ Yes | `auth_token` | Strict |
| Standard login | ✗ NO | N/A | N/A |
| OTP verify | ✗ NO | N/A | N/A |
| Refresh endpoint | ✓ Yes | `auth_token` | Strict |

## Required Changes

### 1. Change Cookie Name: `auth_token` → `re_token`
All JWT cookies should be named `re_token` for consistency.

### 2. Change SameSite: `Strict` → `Lax`
SameSite=Lax is required for OAuth redirects to work properly when returning from Google/GitHub.

### 3. Add Missing JWT Cookies
Standard login and OTP verify need to set the JWT as a cookie (currently they only set refresh_token).

### 4. Ensure Cookie is Set Before All Redirects
Google and GitHub callbacks already do this correctly - need to verify login/OTP responses also include the cookie in the response headers.

## Implementation Plan

### Files to Modify
- `redeye_auth/src/api/handlers.rs`

### Detailed Changes

#### 1. Google Callback (lines 732-735)
```rust
// BEFORE:
let jwt_cookie = format!(
    "auth_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
    jwt
);

// AFTER:
let jwt_cookie = format!(
    "re_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Lax",
    jwt
);
```

#### 2. GitHub Callback (lines 909-912)
```rust
// BEFORE:
let jwt_cookie = format!(
    "auth_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
    jwt
);

// AFTER:
let jwt_cookie = format!(
    "re_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Lax",
    jwt
);
```

#### 3. Refresh Endpoint (lines 289-292)
```rust
// BEFORE:
let jwt_cookie = format!(
    "auth_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Strict",
    jwt
);

// AFTER:
let jwt_cookie = format!(
    "re_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Lax",
    jwt
);
```

#### 4. Standard Login (after line 205)
Add JWT cookie to the response:
```rust
let jwt_cookie = format!(
    "re_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Lax",
    token
);

let mut headers = HeaderMap::new();
headers.insert(SET_COOKIE, HeaderValue::from_str(&refresh_cookie).unwrap());
headers.append(SET_COOKIE, HeaderValue::from_str(&jwt_cookie).unwrap()); // NEW

Ok((headers, Json(AuthResponse { ... })))
```

#### 5. OTP Verify (after line 577)
Add JWT cookie to the response:
```rust
let jwt_cookie = format!(
    "re_token={}; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Lax",
    token
);

let mut headers = HeaderMap::new();
headers.insert(SET_COOKIE, HeaderValue::from_str(&refresh_cookie).unwrap());
headers.append(SET_COOKIE, HeaderValue::from_str(&jwt_cookie).unwrap()); // NEW

Ok((headers, Json(AuthResponse { ... })))
```

## Cookie Structure Summary

All auth methods will set this identical cookie structure:
```
re_token=<JWT>; HttpOnly; Secure; Path=/; Max-Age=604800; SameSite=Lax
```

Where:
- **Name**: `re_token` (consistent across all methods)
- **HttpOnly**: Prevents JavaScript access (XSS protection)
- **Secure**: HTTPS only in production
- **Path**: `/` - Available for all paths
- **Max-Age**: 604800 seconds (7 days)
- **SameSite**: `Lax` - Allows cross-site redirects (needed for OAuth)

## Testing Steps

1. Test Google OAuth login → Check `re_token` cookie is set
2. Test GitHub OAuth login → Check `re_token` cookie is set
3. Test email/password login → Check `re_token` cookie is set
4. Test OTP login → Check `re_token` cookie is set
5. Test token refresh → Check `re_token` cookie is updated
6. Verify all cookies have SameSite=Lax in browser dev tools

## Security Considerations

1. **SameSite=Lax vs Strict**: Lax is needed for OAuth redirects but still provides CSRF protection for POST requests from third-party sites.
2. **HttpOnly**: Prevents XSS attacks from stealing the JWT.
3. **Secure**: Ensures cookie is only sent over HTTPS in production.
4. **Consistent naming**: Using `re_token` everywhere makes debugging and cookie management easier.

## Verification

After changes, verify with:
```bash
cargo check -p redeye_auth
```

Expected: No compilation errors.
