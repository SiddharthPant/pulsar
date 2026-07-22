# Tests and verification

## 1. Update existing user HTTP tests

`apps/server/tests/users_http.rs` currently calls public `/users` endpoints. After the route/context change:

- Change list paths to `/dashboard/users`.
- Change detail paths to `/dashboard/users/{id}`.
- Change expected links in rendered HTML to `/dashboard/users/{id}`.
- Authenticate before every test that needs the user page to reach its database query.

Use the real login endpoint in a test helper instead of constructing session internals. The helper should:

1. Clone the same test app/router used for the protected request.
2. POST `email=ada%40example.com&password=validpass` to `/auth/login`.
3. Set `Content-Type: application/x-www-form-urlencoded`.
4. Optionally set `Datastar-Request: true`; either response mode is valid, but Datastar mode also covers the SDK response.
5. Extract only the `name=value` portion of the `Set-Cookie` header.
6. Add that value as the `Cookie` header on the protected GET.

Do not disable authentication in tests. These integration tests should prove the protected route and its data behavior together.

Special cases:

- `malformed_user_id_returns_bad_request` must authenticate first; otherwise the correct result is the auth redirect, not a UUID rejection.
- `database_failure_returns_sanitized_internal_error` must log in before closing the shared pool. Login must not query PostgreSQL during this mock phase, so it can happen before the close.
- Keep the HTML escaping, order, empty state, not-found, and sanitized 500 assertions.

## 2. Add auth/page integration tests

Create `apps/server/tests/auth_http.rs` or equivalent. Cover at least:

### Public rendering

- `GET /` as anonymous returns 200, has a Login link, and does not show authenticated controls.
- `GET /auth/login` returns 200 and contains the title, email/password fields, absolute action, Datastar attribute, and no forgot-password/signup/social UI.
- The response includes `/assets/main.css` and `/assets/vendor/datastar.js` through the base layout.

### Login

- A valid normal form POST returns 303 with `Location: /dashboard` and a `Set-Cookie` header for `pulsar.sid`.
- A valid Datastar form POST returns 200 `text/event-stream` and an ExecuteScript payload navigating to `/dashboard`.
- Login rotates a pre-existing session ID rather than reusing it. Exercise this through public behavior by logging in once, then posting valid login credentials again with the first cookie and asserting that the returned cookie value differs.
- Invalid credentials do not authenticate the session.
- Invalid Datastar credentials return a PatchElements event containing `login-error` and the generic message.
- Invalid normal credentials re-render the page, preserve the email, omit the password, and show the same generic message.
- A password of exactly eight characters follows the documented rule and succeeds; seven fails.
- An already-authenticated `GET /auth/login` redirects to `/dashboard`.

### Protected pages

- Anonymous `GET /dashboard` returns 303 with `Location: /auth/login`.
- Anonymous `GET /dashboard/users` returns the same redirect and does not expose page content.
- Authenticated `GET /dashboard` returns 200 and displays the session user's name.
- Authenticated landing HTML replaces Login with Dashboard and Logout controls.

### Logout

- Normal POST logout redirects to `/`.
- Datastar POST logout returns an ExecuteScript navigation to `/`.
- Logout emits a cookie removal/update header as required by Tower Sessions.
- Reusing the old cookie after logout cannot access `/dashboard` and redirects to login.

### Cookie contract

In local mode, assert the cookie is:

- Named `pulsar.sid`.
- `HttpOnly`.
- `SameSite=Lax`.
- `Path=/`.
- Not `Secure`, because local development uses plain HTTP.

Do not copy this local `Secure=false` expectation into production configuration tests.

## 3. Update router unit tests

The existing `apps/server/src/router.rs` tests mostly remain valid. Add or adjust:

- Trailing-slash normalization should use `/dashboard/users/not-a-uuid` and authenticate if the assertion remains about UUID parsing. Alternatively, make the normalization assertion compare the two anonymous redirect statuses/locations.
- Unknown-route and request-ID behavior remain unchanged.
- Add an assertion that the old `/users` route is now 404, preventing accidental duplicate public access.

## 4. Compile gates while implementing

Askama validates template paths, inherited blocks, and referenced fields at compile time. Run this after each backend/template phase:

```bash
cargo check --workspace
```

The first successful check proves:

- All four stale template paths were replaced.
- Child templates use valid layout/block names.
- Every landing template struct has `layout.is_authenticated()` available.
- Every dashboard template struct has `layout.current_user_name` available.

## 5. Final automated verification

Run from the repository root:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

The SQLx integration tests require the repository's configured test database environment. Start PostgreSQL through the existing Compose/mise workflow before running them if it is not already available.

Also search for stale paths and unsafe credential logging:

```bash
rg -n 'auth/base\.html|hello\.html|user_list\.html|user_detail\.html|/login/handle|href="/users|tracing::.*password|password.*tracing::' apps/server
```

Expected result: no stale template/route references and no statement that logs a password. The password field will still legitimately appear in validation and `Form` extraction code.

## 6. Browser verification

Test at approximately 375px, 768px, and a desktop width:

1. Open `/` in a fresh private window; confirm Login is shown.
2. Open `/dashboard` directly; confirm the browser arrives at `/auth/login`.
3. Confirm the login card is vertically/horizontally centered, never wider than 24rem, and has no legacy 600px body offset.
4. Submit invalid credentials; confirm the inline error appears without a full-page patch or console-only message.
5. Submit valid mock credentials; confirm the address bar becomes `/dashboard`.
6. Navigate to Users and a user detail; confirm all URLs remain under `/dashboard/users`.
7. Confirm wide tables scroll within the content region on a narrow viewport.
8. Return to `/`; confirm authenticated controls are shown.
9. Logout; confirm the address bar becomes `/` and Login returns.
10. Use Back to revisit a protected URL; confirm it redirects to login rather than showing protected content.
11. Navigate the login form using only the keyboard and confirm clear focus-visible states.
12. Disable JavaScript and repeat normal form login/logout to verify the fallback redirects/rendering.

## Completion record

When implementation is finished, append the exact command outputs or a short pass/fail record here, plus any intentional deviations from the route or context contract. Do not mark the plan complete while any manual URL points to a missing handler or any test bypasses the authentication boundary.
