# CSS changes

## Goal

Replace the mixed partial auth styles and legacy global page styles in `assets/main.css` with one layout-aware stylesheet. The login page should match the earlier shadcn reference: near-black full viewport, compact centered card, subtle neutral border, restrained typography, stacked fields, and a high-contrast full-width Login button.

No Tailwind, CSS framework, external font, or component library is needed.

## Current CSS problems

- `body { max-width: 600px; margin: 40px auto; ... }` affects auth, landing, and dashboard bodies.
- `.auth-page` is styled but no template emits that class; the actual shell is `.auth-layout`.
- Auth inputs, labels, form spacing, button, focus state, error state, and muted text are unfinished.
- The old table/card rules use light colors while `:root { color-scheme: dark; }` applies globally.
- There are no landing or dashboard layout rules.
- The trailing comma in the `.auth-card` `box-shadow` declaration makes that declaration invalid.

## Stylesheet organization

Use native cascade layers so layout and component ordering is explicit:

```css
@layer reset, tokens, base, layouts, components;
```

Suggested sections:

1. `reset`: box sizing, body margin, inherited controls, responsive media.
2. `tokens`: neutral color, radius, shadow, spacing, and container custom properties.
3. `base`: body typography, links, buttons, focus-visible, shared container.
4. `layouts`: landing, auth, and dashboard shells.
5. `components`: auth card/form, navigation, tables, empty state, detail card.

Do not use global width/margin rules on `body`. Width belongs to `.site-container`, `.auth-layout__content`, and `.dashboard-main`.

## Foundation

Start with a small modern reset:

```css
@layer reset {
  *,
  *::before,
  *::after {
    box-sizing: border-box;
  }

  html {
    min-block-size: 100%;
  }

  body {
    min-block-size: 100vh;
    min-block-size: 100svh;
    margin: 0;
  }

  button,
  input,
  textarea,
  select {
    font: inherit;
  }

  img,
  svg {
    display: block;
    max-inline-size: 100%;
  }
}
```

Use logical properties (`inline-size`, `block-size`, `padding-inline`) and responsive functions (`min()`, `clamp()`). Use `:focus-visible` for keyboard focus rather than removing outlines.

## Tokens

Keep shared neutral tokens independent of any one layout. An appropriate dark shadcn-like palette is:

```css
@layer tokens {
  :root {
    --font-sans: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont,
      "Segoe UI", sans-serif;

    --color-background: oklch(0.13 0.005 286);
    --color-surface: oklch(0.17 0.006 286);
    --color-surface-raised: oklch(0.205 0.007 286);
    --color-border: oklch(0.30 0.008 286);
    --color-foreground: oklch(0.985 0 0);
    --color-muted: oklch(0.71 0.014 286);
    --color-primary: oklch(0.94 0 0);
    --color-primary-hover: oklch(0.86 0 0);
    --color-primary-foreground: oklch(0.16 0.006 286);
    --color-danger: oklch(0.70 0.19 24);
    --color-focus: oklch(0.74 0.02 286);

    --radius-sm: 0.375rem;
    --radius-md: 0.625rem;
    --radius-lg: 0.75rem;
    --site-container: 72rem;
  }
}
```

If landing pages will eventually use a light theme, scope `color-scheme: dark` and dark body colors to `.auth-body` and `.dashboard-body` instead of declaring it at `:root`.

## Auth shell and card

The selectors must match the new layout:

```css
@layer layouts {
  .auth-body {
    color-scheme: dark;
    background: var(--color-background);
    color: var(--color-foreground);
  }

  .auth-layout {
    min-block-size: 100svh;
    display: grid;
    place-items: center;
    padding: clamp(1rem, 4vw, 2rem);
  }

  .auth-layout__content {
    inline-size: min(100%, 24rem);
  }
}
```

Card and typography:

```css
@layer components {
  .auth-card {
    display: grid;
    gap: 1.5rem;
    padding: clamp(1.25rem, 4vw, 1.5rem);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow:
      0 1px 2px rgb(0 0 0 / 25%),
      0 12px 32px rgb(0 0 0 / 20%);
  }

  .auth-card__header {
    display: grid;
    gap: 0.375rem;
  }

  .auth-card__header :is(h1, p) {
    margin: 0;
  }

  .auth-card__header h1 {
    font-size: 1.0625rem;
    font-weight: 600;
    line-height: 1.35;
    letter-spacing: -0.015em;
  }

  .auth-card__header p {
    color: var(--color-muted);
    font-size: 0.875rem;
  }
}
```

Form controls:

```css
.auth-form {
  display: grid;
  gap: 1rem;
}

.auth-field {
  display: grid;
  gap: 0.5rem;
}

.auth-field label {
  font-size: 0.875rem;
  font-weight: 500;
}

.auth-field input {
  inline-size: 100%;
  min-block-size: 2.5rem;
  padding-inline: 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface-raised);
  color: var(--color-foreground);
  outline: 0;
  transition: border-color 140ms ease, box-shadow 140ms ease,
    background-color 140ms ease;
}

.auth-field input::placeholder {
  color: color-mix(in oklch, var(--color-muted), transparent 20%);
}

.auth-field input:hover {
  border-color: color-mix(in oklch, var(--color-border), white 12%);
}

.auth-field input:focus-visible {
  border-color: var(--color-focus);
  box-shadow: 0 0 0 3px color-mix(in oklch, var(--color-focus), transparent 72%);
}

.auth-form__error {
  min-block-size: 1.25rem;
  margin: -0.25rem 0 0;
  color: var(--color-danger);
  font-size: 0.8125rem;
}

.auth-form__submit {
  min-block-size: 2.5rem;
  border: 0;
  border-radius: var(--radius-sm);
  background: var(--color-primary);
  color: var(--color-primary-foreground);
  font-weight: 600;
  cursor: pointer;
  transition: background-color 140ms ease, transform 80ms ease;
}

.auth-form__submit:hover {
  background: var(--color-primary-hover);
}

.auth-form__submit:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.auth-form__submit:active {
  transform: translateY(1px);
}

.auth-form__submit:focus-visible {
  outline: 3px solid color-mix(in oklch, var(--color-focus), transparent 45%);
  outline-offset: 2px;
}
```

Do not style validation solely with red borders; the visible error text and `role="alert"` carry the message.

## Shared and landing styles

Add:

- `.app-body` for shared font, line height, text rendering, and base colors.
- `.site-container { inline-size: min(100% - 2rem, var(--site-container)); margin-inline: auto; }`.
- `.site-brand` with a compact semibold treatment.
- `.landing-body` as a three-row grid (`auto 1fr auto`) with `min-block-size: 100svh`.
- `.landing-header` with a subtle bottom border.
- `.landing-header__inner` as a flex row with `justify-content: space-between`.
- `.landing-nav` as an aligned flex row, including a zero-margin logout form.
- `.landing-main` with responsive block padding.
- `.landing-footer` with a top border and muted text.

Do not let generic `a` styling force dashboard/auth colors. Prefer component selectors and `text-decoration-thickness`/`text-underline-offset` for readable link states.

## Dashboard styles

The existing layout expects:

- `.dashboard-layout`: two-column grid with a fixed/minmax sidebar and fluid workspace.
- `.dashboard-sidebar`: full-height dark surface, brand, and vertical nav.
- `.dashboard-workspace`: `min-inline-size: 0` to prevent table overflow from expanding the grid.
- `.dashboard-header`: aligned name/logout controls with a bottom border.
- `.dashboard-main`: constrained content width and responsive padding.
- `.dashboard-page`, `.page-header`, `.page-eyebrow`: page rhythm.

At a narrow breakpoint (around `48rem`), collapse the sidebar into a top section or reduce the layout to one column. The basic version can place brand and nav horizontally before the workspace; it must not leave the content narrower than the login card.

## User page components

Add scoped styles for:

- `.table-card`: bordered surface with rounded corners.
- `.table-scroll`: `overflow-x: auto`.
- `.table-card table`: full width, collapsed borders, no global table margin.
- Table headings: muted small text; table rows: subtle separators and hover state.
- `.empty-state`: bordered, padded, muted surface.
- `.detail-card`: bordered surface and grid spacing.
- `.detail-list`: zero margin and grid gaps.
- `.detail-list__item`: two-column label/value grid, collapsing to one column on small screens.
- `.detail-list dd`: `overflow-wrap: anywhere` for UUIDs/emails.
- `.button-link`: inline-flex button-like back action.

Delete the legacy global `.card`, `.meta-item`, `.label`, `.back-btn`, `table`, `th`, `td`, and `a` rules after the templates stop using those generic class names.

## Motion and accessibility

Finish with a reduced-motion override:

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    scroll-behavior: auto !important;
    transition-duration: 0.01ms !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
  }
}
```

Verify contrast for muted text, danger text, borders, and focus rings against their actual backgrounds. Native controls must retain clear hover, focus-visible, active, and disabled states.
