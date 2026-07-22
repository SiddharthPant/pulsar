# Template changes

## Inheritance contract

The layouts establish these blocks:

| Layout | Child block | Required child field |
| --- | --- | --- |
| `layouts/base.html` | `page` | None |
| `layouts/landing.html` | `content` | `layout: LandingLayoutContext` |
| `layouts/dashboard.html` | `content` | `layout: DashboardLayoutContext` |
| `layouts/auth.html` | `content` | None |

Every page must extend its direct layout and override `content`. No page should extend `layouts/base.html` directly, extend the deleted `auth/base.html`, or override the deleted `body` block.

## 1. Landing page

`apps/server/templates/pages/index.html` already has the correct inheritance and block name. It only needs enough content to exercise the layout:

- A clear page heading and one short description.
- An authenticated call-to-action linking to `/dashboard`, or a public call-to-action linking to `/auth/login`, if desired.
- Keep authentication conditionals in `layouts/landing.html`; do not duplicate them in the page unless the page content genuinely differs.

## 2. Auth login page

Replace `apps/server/templates/pages/auth/login.html` with markup shaped like this:

```html
{% extends "layouts/auth.html" %}

{% block title %}Login | Pulsar{% endblock %}

{% block content %}
    <section class="auth-card" aria-labelledby="login-title">
        <header class="auth-card__header">
            <h1 id="login-title">Login to your account</h1>
            <p>Enter your email below to login to your account.</p>
        </header>

        <form
            id="login-form"
            class="auth-form"
            method="post"
            action="/auth/login"
            data-on:submit="@post('/auth/login', {contentType: 'form'})"
        >
            <div class="auth-field">
                <label for="email">Email</label>
                <input
                    id="email"
                    name="email"
                    type="email"
                    value="{{ email }}"
                    autocomplete="username"
                    placeholder="m@example.com"
                    required
                    autofocus
                >
            </div>

            <div class="auth-field">
                <label for="password">Password</label>
                <input
                    id="password"
                    name="password"
                    type="password"
                    autocomplete="current-password"
                    minlength="8"
                    required
                >
            </div>

            <p id="login-error" class="auth-form__error" role="alert" aria-live="polite">
                {% match error %}
                    {% when Some with (message) %}{{ message }}
                    {% when None %}
                {% endmatch %}
            </p>

            <button class="auth-form__submit" type="submit">Login</button>
        </form>
    </section>
{% endblock %}
```

Important details:

- The POST URL is absolute. This removes the relative-URL ambiguity that previously contributed to login routing confusion.
- `method` and `action` preserve an ordinary form fallback.
- Datastar automatically prevents the default submit event for a form with `data-on:submit`; no manual JavaScript is needed.
- `contentType: 'form'` sends URL-encoded form fields accepted by Axum's `Form<LoginInput>`.
- The error node always exists, allowing `PatchElements` to morph it by ID.
- Keep the submitted email after a normal validation failure; never put the password back into HTML.
- Do not add forgot-password, signup, social login, or separator rows.

## 3. Dashboard overview page

Add `apps/server/templates/pages/dashboard/index.html`:

```html
{% extends "layouts/dashboard.html" %}

{% block title %}Dashboard | Pulsar{% endblock %}

{% block content %}
    <section class="dashboard-page" aria-labelledby="dashboard-title">
        <header class="page-header">
            <p class="page-eyebrow">Overview</p>
            <h1 id="dashboard-title">Dashboard</h1>
            <p>Welcome back, {{ layout.current_user_name }}.</p>
        </header>
    </section>
{% endblock %}
```

This makes the existing `/dashboard` brand/Overview links and post-login destination real.

## 4. User directory page

In `apps/server/templates/pages/dashboard/users/index.html`:

- Extend `layouts/dashboard.html`.
- Rename the overridden block from `body` to `content`.
- Change detail links to `/dashboard/users/{{ user.id }}`.
- Wrap the page in `.dashboard-page` and use a `.page-header`.
- Put the table in a `.table-scroll` wrapper so narrow viewports scroll the table instead of overflowing the dashboard.
- Keep the current empty state and table semantics.

Suggested shape:

```html
{% extends "layouts/dashboard.html" %}

{% block title %}User Directory | Pulsar{% endblock %}

{% block content %}
    <section class="dashboard-page" aria-labelledby="users-title">
        <header class="page-header">
            <p class="page-eyebrow">Administration</p>
            <h1 id="users-title">User Directory</h1>
        </header>

        {% if users.is_empty() %}
            <div class="empty-state">
                <p>No registered users found in the system.</p>
            </div>
        {% else %}
            <div class="table-card table-scroll">
                <!-- Preserve the existing semantic table. -->
            </div>
        {% endif %}
    </section>
{% endblock %}
```

## 5. User detail page

In `apps/server/templates/pages/dashboard/users/view.html`:

- Extend `layouts/dashboard.html`.
- Rename `body` to `content`.
- Change the back link to `/dashboard/users`.
- Replace generic `.card`, `.meta-item`, `.label`, and `.back-btn` names with dashboard-scoped classes such as `.detail-card`, `.detail-list`, `.detail-list__item`, and `.button-link`.
- Use a semantic definition list for label/value pairs.

Suggested content structure:

```html
<section class="dashboard-page" aria-labelledby="user-title">
    <header class="page-header">
        <p class="page-eyebrow">User profile</p>
        <h1 id="user-title">{{ user.full_name }}</h1>
    </header>

    <div class="detail-card">
        <dl class="detail-list">
            <div class="detail-list__item">
                <dt>Database Key ID</dt>
                <dd>{{ user.id }}</dd>
            </div>
            <div class="detail-list__item">
                <dt>Full Name</dt>
                <dd>{{ user.full_name }}</dd>
            </div>
            <div class="detail-list__item">
                <dt>Email Address</dt>
                <dd>{{ user.email }}</dd>
            </div>
        </dl>
        <a class="button-link" href="/dashboard/users">&larr; Back to Directory</a>
    </div>
</section>
```

## 6. Required small layout corrections

The hierarchy is correct, but two navigation details must change for the finished flow.

### `layouts/dashboard.html`

- Change the Users link from `/users` to `/dashboard/users`.
- Make logout a real form so it works with and without Datastar:

```html
<form
    method="post"
    action="/auth/logout"
    data-on:submit="@post('/auth/logout', {contentType: 'form'})"
>
    <button type="submit">Logout</button>
</form>
```

The layout can continue to render `{{ layout.current_user_name }}`.

### `layouts/landing.html`

Keep the existing Login link for anonymous users. For authenticated users, render the Dashboard link and the same POST logout form. This fulfills the Login/Logout conditional behavior without forcing every landing handler to pass separate booleans.

Do not add authentication references to `layouts/base.html` or `layouts/auth.html`.

## 7. Base and auth layouts

No structural rewrite is required. Keep:

- Base ownership of doctype, metadata, title, favicon, shared CSS, vendored Datastar script, `head`, `body_class`, and `page`.
- Auth ownership of the full-height centered shell and `content`.

Formatting the body block over multiple lines is optional, but preserve the `app-body` class because the replacement CSS uses it for global sizing/reset behavior.
