# Basic pages and login/logout flow

## Goal

Finish the current server-rendered pages around the new three-layout hierarchy:

```text
layouts/base.html
├── layouts/landing.html
│   └── pages/index.html
├── layouts/dashboard.html
│   ├── pages/dashboard/index.html
│   └── pages/dashboard/users/{index,view}.html
└── layouts/auth.html
    └── pages/auth/login.html
```

The finished flow is:

1. An anonymous visitor sees a Login link on `/`.
2. `/auth/login` renders a centered, dark, shadcn-like login card.
3. A successful login rotates the session ID, stores a typed mock user in Tower Sessions' `MemoryStore`, and navigates to `/dashboard`.
4. `/dashboard` and `/dashboard/users/*` reject anonymous requests with a redirect to `/auth/login`.
5. Landing pages receive an optional landing layout context. Dashboard pages receive a mandatory dashboard layout context.
6. Logout flushes the server-side session and navigates back to `/`.

This is deliberately a working mock authentication boundary, not production password authentication. The existing [`../nats-session-auth.md`](../nats-session-auth.md) remains the plan for PostgreSQL password hashes, `axum-login`, CSRF protection, and the NATS session store. The page/context structure in this plan is intended to survive that replacement.

## Current blockers found on 2026-07-22

`cargo check --workspace` currently fails because four Rust template paths still name deleted files:

- `auth.rs` points at `auth/login.html`.
- `home.rs` points at `hello.html`.
- `users.rs` points at `user_list.html` and `user_detail.html`.

There are also runtime/template mismatches:

- Login and user pages extend the deleted `auth/base.html` and override the deleted `body` block.
- `layouts/auth.html` provides `content`, but its CSS currently targets `.auth-page`, which is not in the layout.
- User pages live under the dashboard template tree but are still public at `/users` and do not receive dashboard layout context.
- `layouts/dashboard.html` links to `/dashboard`, but no dashboard index handler or page exists.
- `layouts/dashboard.html` links to `/users`, which does not match the desired dashboard route hierarchy.
- Invalid login writes only to the browser console.
- The login handler logs the submitted password. This must be removed.
- The global `body` rule in `assets/main.css` constrains every layout to 600px and conflicts with auth/dashboard shells.

The vendored Datastar files do exist at `assets/vendor/datastar.js` and `assets/vendor/datastar.js.map`; no asset download is needed.

## Target route contract

| Method | Path | Access | Response |
| --- | --- | --- | --- |
| GET | `/` | Public | Landing page; Login or Dashboard/Logout controls depend on session |
| GET | `/auth/login` | Public | Login page; authenticated visitors redirect to `/dashboard` |
| POST | `/auth/login` | Public | Validate mock credentials, create session, navigate to `/dashboard`, or show an inline error |
| POST | `/auth/logout` | Authenticated in normal UI | Flush session and navigate to `/` |
| GET | `/dashboard` | Protected | Basic dashboard overview |
| GET | `/dashboard/users` | Protected | User directory |
| GET | `/dashboard/users/{id}` | Protected | User detail |

Use absolute URLs in templates. Do not retain `/auth/login/handle` or the top-level `/users` routes.

## Architectural decisions

### Layout context follows the layout boundary

Do not add unrelated variables to a global Askama runtime map. Each template struct has one field named `layout` whose type matches its parent layout:

- `LandingLayoutContext`: optional authentication state and `is_authenticated()`.
- `DashboardLayoutContext`: mandatory authenticated user data and the auth rejection behavior.
- Auth pages: no shared authentication context is required to render.

This keeps Askama compile-time checking and prevents every page handler from manually inventing an `is_authenticated` boolean.

### Authentication enforcement is an extractor

Implement `FromRequestParts` for `DashboardLayoutContext`. A dashboard handler cannot be called without extracting the authenticated user, and the extracted value is also exactly what its template needs. A missing session redirects to `/auth/login`; a session-store error remains a 500 and must not be treated as anonymous.

### Datastar requests and ordinary form requests both work

Datastar sends `Datastar-Request: true`. For Datastar submissions:

- Use the Rust SDK's `ExecuteScript` event for top-level navigation.
- Use `PatchElements` to replace the inline error element.

For a normal browser form submission, return an Axum `Redirect` on success and re-render the login page on failure. This keeps the form usable if the script fails to load.

### Route hierarchy matches the template hierarchy

Dashboard user pages move from `/users` to `/dashboard/users`. Add a minimal dashboard index rather than leaving the existing Overview and post-login links broken.

## Documents

- [`01-backend.md`](01-backend.md): session model, layout contexts, handlers, router, and response behavior.
- [`02-templates.md`](02-templates.md): exact inheritance fixes and page markup.
- [`03-css.md`](03-css.md): stylesheet organization and the shadcn-like login visual specification.
- [`04-tests-and-verification.md`](04-tests-and-verification.md): test migrations, new auth coverage, and completion gates.

## Implementation order

1. Add the typed session user and both layout contexts in `page.rs`/`auth.rs`.
2. Add the protected dashboard route and update router nesting.
3. Update home, auth, and user template paths and handler template fields.
4. Fix page inheritance and add `pages/dashboard/index.html`.
5. Replace the conflicting CSS with layout-aware base, auth, landing, dashboard, and user-page styles.
6. Update existing HTTP tests to authenticate before protected requests.
7. Add login, logout, redirect, conditional-navigation, and protection tests.
8. Run all verification commands from `04-tests-and-verification.md` and perform the browser checklist.

## Definition of done

- `cargo check --workspace`, formatting, Clippy, and tests pass.
- No Rust template path references a deleted template.
- No page extends `auth/base.html` or overrides `body`.
- Anonymous dashboard requests redirect without querying the page data.
- A successful login sets `pulsar.sid`, rotates the ID, and reaches `/dashboard`.
- Invalid credentials display one generic inline error and do not create an authenticated session.
- Logout clears the session and the old cookie can no longer access protected pages.
- The login card visually matches the earlier dark shadcn reference without Tailwind, social login, signup, or forgot-password UI.
- Landing, auth, and dashboard layouts work at narrow and wide viewport sizes without inheriting the old 600px body constraint.
