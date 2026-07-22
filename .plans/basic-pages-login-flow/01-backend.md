# Backend changes

## 1. Represent the authenticated session once

Keep the session identity in `apps/server/src/auth.rs`, beside the login/logout lifecycle. Replace the raw `"user_pid"` string value with one serialized value under a namespaced key:

```rust
const AUTH_USER_KEY: &str = "auth.user";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SessionUser {
    pub user_pid: String,
    pub display_name: String,
}

impl SessionUser {
    pub(crate) async fn from_session(
        session: &Session,
    ) -> Result<Option<Self>, AppError> {
        session
            .get(AUTH_USER_KEY)
            .await
            .map_err(anyhow::Error::from)
            .map_err(AppError::from)
    }
}
```

Add `Serialize` to the existing Serde imports. The current fake login can create a fixed public ID and use the normalized submitted email as the temporary display name. Do not store the password.

This single object avoids multiple session reads and prevents the login handler, landing context, and dashboard context from using different keys.

## 2. Replace `PageContext` with layout-specific contexts

Update `apps/server/src/page.rs` to contain the two view models. `LandingLayoutContext` is constructed explicitly by public-page handlers. `DashboardLayoutContext` is an Axum extractor so it also guards protected handlers.

The intended public shape is:

```rust
#[derive(Clone, Debug)]
pub struct LandingLayoutContext {
    authenticated: bool,
}

impl LandingLayoutContext {
    pub async fn from_session(session: &Session) -> Result<Self, AppError> {
        Ok(Self {
            authenticated: SessionUser::from_session(session).await?.is_some(),
        })
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }
}

#[derive(Clone, Debug)]
pub struct DashboardLayoutContext {
    pub user_pid: String,
    pub current_user_name: String,
}
```

Implement the protected extractor along these lines:

```rust
impl<S> FromRequestParts<S> for DashboardLayoutContext
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;

        let user = SessionUser::from_session(&session)
            .await
            .map_err(IntoResponse::into_response)?
            .ok_or_else(|| Redirect::to("/auth/login").into_response())?;

        Ok(Self {
            user_pid: user.user_pid,
            current_user_name: user.display_name,
        })
    }
}
```

Required imports include `axum::extract::FromRequestParts`, `axum::http::request::Parts`, `axum::response::{IntoResponse, Redirect, Response}`, `tower_sessions::Session`, and `crate::auth::SessionUser`.

Important behavior:

- `None` means genuinely anonymous and becomes a redirect.
- A Tower Sessions load failure becomes `AppError::Internal` and then a 500 response.
- The extractor must run before a dashboard handler starts its database query. Put the `layout` argument before `State`/`Path` in handler signatures for clarity.

Do not put these values in `AppState`; router state is application-wide and is not per-browser authentication state.

## 3. Update the home handler

In `apps/server/src/home.rs`:

- Rename `HelloTemplate` to `HomeTemplate`.
- Change the Askama path to `pages/index.html`.
- Change the template field from `page: PageContext` to `layout: LandingLayoutContext`.
- Build the landing context from the session.

Target structure:

```rust
#[derive(Template)]
#[template(path = "pages/index.html")]
struct HomeTemplate {
    layout: LandingLayoutContext,
}

async fn home(session: Session) -> Result<impl IntoResponse, AppError> {
    let layout = LandingLayoutContext::from_session(&session).await?;
    Ok(HtmlTemplate::new(HomeTemplate { layout }))
}
```

## 4. Add the dashboard overview handler

Create `apps/server/src/dashboard.rs` and declare `mod dashboard;` in `apps/server/src/lib.rs`.

The dashboard module should own the complete dashboard subtree:

```rust
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .nest("/users", users::routes())
}

#[derive(Template)]
#[template(path = "pages/dashboard/index.html")]
struct DashboardTemplate {
    layout: DashboardLayoutContext,
}

async fn index(layout: DashboardLayoutContext) -> impl IntoResponse {
    HtmlTemplate::new(DashboardTemplate { layout })
}
```

The module imports `crate::users`; `users::routes()` remains `pub(crate)` through the existing private module boundary. There is no need for a second authentication middleware because every dashboard handler accepts `DashboardLayoutContext`.

## 5. Update user handlers

In `apps/server/src/users.rs`:

- Point `UserListTemplate` at `pages/dashboard/users/index.html`.
- Point `UserDetailTemplate` at `pages/dashboard/users/view.html`.
- Add `layout: DashboardLayoutContext` to both template structs.
- Extract the layout in both handlers and pass it to the template.

Target signatures and construction:

```rust
struct UserListTemplate {
    layout: DashboardLayoutContext,
    users: Vec<User>,
}

struct UserDetailTemplate {
    layout: DashboardLayoutContext,
    user: User,
}

async fn list_users(
    layout: DashboardLayoutContext,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // existing query
    Ok(HtmlTemplate::new(UserListTemplate { layout, users }))
}

async fn get_user(
    layout: DashboardLayoutContext,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // existing query
    Ok(HtmlTemplate::new(UserDetailTemplate { layout, user }))
}
```

Keep the existing query error context, `NotFound` behavior, ordering, and Askama escaping.

## 6. Make auth routes resource-shaped

In `apps/server/src/auth.rs`, replace the route pair `/login` plus `/login/handle` with one resource route:

```rust
Router::new()
    .route("/login", get(login_page).post(login))
    .route("/logout", post(logout))
```

Update the template path and fields:

```rust
#[derive(Template)]
#[template(path = "pages/auth/login.html")]
struct LoginTemplate {
    email: String,
    error: Option<&'static str>,
}
```

Use an empty email and `None` for GET. If a user is already authenticated, GET `/auth/login` should return `Redirect::to("/dashboard")` rather than rendering another login form.

### Temporary credential rule

Keep the mock rule explicit and isolated in a helper. It should trim the email, reject an empty email, and require at least eight password characters. It must not log either credential.

```rust
fn mock_credentials_are_valid(input: &LoginInput) -> bool {
    !input.email.trim().is_empty() && input.password.chars().count() >= 8
}
```

The browser's `type="email"` validation is not server validation, but adding an email-parsing dependency only for this temporary mock does not improve the eventual auth boundary. Real email lookup and Argon2 verification belong to the existing NATS/auth plan.

### Successful login

On success, in this order:

1. Normalize the email with `trim()` (and lowercase it if it is used as identity).
2. Call `session.cycle_id()` to prevent session fixation.
3. Insert one `SessionUser` under `AUTH_USER_KEY`.
4. For a Datastar request, return an SDK `ExecuteScript` event containing `window.location.replace("/dashboard")`.
5. For an ordinary form request, return `Redirect::to("/dashboard")`.

Keep the redirect path a server constant/static string; do not interpolate untrusted form data into JavaScript.

### Invalid login

Always use the generic public message `Invalid email or password.`

- Datastar request: return HTTP 200 with a `PatchElements` SSE event replacing `#login-error`. Datastar treats non-200 fetch responses as failures/retries, so the inline validation patch should be a successful transport response.
- Ordinary form request: return a re-rendered `LoginTemplate` with the submitted email, the generic error, and an appropriate 4xx status such as 422.
- Never return the submitted password to the template.

### Datastar request detection and SDK helpers

Datastar 1.x sends the `Datastar-Request: true` header. Extract `HeaderMap` and centralize the check:

```rust
fn is_datastar_request(headers: &HeaderMap) -> bool {
    headers.contains_key("datastar-request")
}
```

Both `ExecuteScript::write_as_axum_sse_event()` and `PatchElements::write_as_axum_sse_event()` produce Axum SSE events. Wrap one event with the existing `Sse::new(tokio_stream::once(...))` pattern and convert it into `Response`. Returning `Result<Response, AppError>` from mixed-response handlers avoids incompatible `impl IntoResponse` branch types.

Put that transport boilerplate in `apps/server/src/response.rs` instead of repeating it in each auth branch:

```rust
pub fn datastar_event(event: Event) -> Response {
    Sse::new(once(Ok::<Event, Infallible>(event))).into_response()
}
```

This helper owns only the one-event Axum SSE envelope. `auth.rs` remains responsible for deciding whether the SDK event is `ExecuteScript` or `PatchElements`.

For the error patch, send a stable element with the same ID as the page placeholder:

```html
<p id="login-error" class="auth-form__error" role="alert">Invalid email or password.</p>
```

### Logout

Logout should:

1. Call `session.flush()`.
2. Return a Datastar `ExecuteScript` navigation to `/` when the request header is present.
3. Otherwise return `Redirect::to("/")`.

Do not use GET for logout.

## 7. Update router assembly and session-cookie settings

In `apps/server/src/router.rs`:

- Import `dashboard` instead of mounting `users` directly.
- Replace `.nest("/users", users::routes())` with `.nest("/dashboard", dashboard::routes())`.
- Apply the session layer around all dynamic browser routes so both login and page contexts can extract `Session`.
- Merge `/assets` outside that session-wrapped router. Static asset requests do not need session extraction, and an authenticated browser should not refresh/write its sliding session merely by loading CSS or JavaScript.

For the local MemoryStore phase, make the intended cookie behavior explicit:

```rust
use tower_sessions::cookie::SameSite;

let session_layer = SessionManagerLayer::new(MemoryStore::default())
    .with_name("pulsar.sid")
    .with_http_only(true)
    .with_same_site(SameSite::Lax)
    .with_secure(false)
    .with_path("/")
    .with_expiry(Expiry::OnInactivity(CookieDuration::days(1)))
    .with_always_save(true);
```

`with_always_save(true)` is needed if the one-day `OnInactivity` value is intended to slide on reads. Without it, an unmodified session is not saved on each request.

Assemble a `browser_routes` router containing home, auth, and dashboard, apply `session_layer` to that router, and then merge it with the static asset service before applying the global timeout/tracing layers. The fallback does not require a session.

`MemoryStore` is process-local and resets on restart. That is the requested behavior for this stage; do not add an authentication array to `AppState`.

## 8. Remaining files

- `apps/server/src/response.rs`: keep `HtmlTemplate` unchanged and add the small `datastar_event` helper described above.
- `apps/server/src/error.rs`: the dashboard extractor can use `Response` as its rejection, so no redirect variant is required.
- `apps/server/src/config.rs`: no new setting is required for the local-only MemoryStore stage. The secure-cookie environment split belongs with deployment authentication work.
- Cargo manifests: the needed dependencies and features already exist.
