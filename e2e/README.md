# End-to-End Tests

Playwright test scaffolding for the `swings` SvelteKit app. Specs live under
`e2e/` and are organised by behavioural bucket rather than by page.

## Quick start

```bash
# One-time browser install.
pnpm exec playwright install

# Full run (uses the config-level webServer: `pnpm build && pnpm preview`).
pnpm test:e2e

# Interactive / headed run against a locally-running preview server.
pnpm dev                 # terminal 1 — optional (webServer will spawn if absent)
pnpm exec playwright test --headed
```

Artifacts (HTML report, traces, screenshots, videos) are written to
`playwright-report/` and `test-results/`.

## Environment variables

| Variable | Purpose | Default |
| --- | --- | --- |
| `ADMIN_EMAIL` | Bootstrap admin email seeded by the Rust backend at first boot. | `admin@swings.test` |
| `ADMIN_PASSWORD` | Bootstrap admin password. | `admin-password-1234` |
| `DATABASE_URL` | Postgres URL the backend reads from. Only relevant if you run the backend locally. | (backend default) |
| `FRONTEND_URL` | Canonical frontend origin — unused in E2E but honoured by some backend routes. | — |
| `VITE_API_URL` | Backend origin for the SSR / preview servers. E2E falls back to `http://127.0.0.1:3001`. | — |
| `PLAYWRIGHT_BASE_URL` | Override the frontend URL under test. | `http://localhost:4173` |
| `PLAYWRIGHT_PORT` | Convenience — drives the default base URL only. | `4173` |
| `PLAYWRIGHT_WEB_SERVER_COMMAND` | Override the `webServer.command`. Leave unset in CI. | `pnpm build && pnpm preview` |

## Directory map

```
e2e/
  fixtures/
    app.ts          — `test` extended with `app: AppClient` + `api: ApiClient`
    auth.ts         — `authedAdminTest` / `authedMemberTest` pre-logged-in fixtures
    helpers.ts      — `disposableEmail`, `isProblem`, `apiBaseUrl`
    selectors.ts    — role/label-based locator factories
  pages/
    HomePage.ts
    LoginPage.ts
    RegisterPage.ts
    DashboardPage.ts
    AdminUiKitPage.ts
  smoke/            — prerendered public pages (every browser engine)
    home.spec.ts
    pricing.spec.ts
    blog.spec.ts
  auth/             — authenticated flows (Chromium only)
    register-login.spec.ts
    invalid-credentials.spec.ts
    rate-limit.spec.ts
  popups/
    trigger.spec.ts
```

## Fixture usage

Always import `test` from `e2e/fixtures/app` (or `e2e/fixtures/auth` when you
need a pre-logged-in session). The plain `@playwright/test` default `test` is
intentionally bypassed so every spec inherits the `AppClient` and `ApiClient`.

```ts
import { test, expect } from '../fixtures/app';
import { HomePage } from '../pages/HomePage';

test('renders the landing page', async ({ page, app, api }) => {
    await new HomePage(page).open();
    // `app.login(...)` / `api.get(...)` when you need side channels.
});
```

Pre-authenticated:

```ts
import { authedAdminTest as test } from '../fixtures/auth';

test('admin-only flow', async ({ admin, app }) => {
    // `admin` is the `{ user, access_token, refresh_token }` bundle.
    await app.goto('/admin/_ui-kit');
});
```

## Debugging tips

- `pnpm exec playwright test --headed` — run with a visible browser window.
- `pnpm exec playwright test --debug` — opens the Playwright inspector.
- `pnpm exec playwright test --project=smoke-chromium` — single project only.
- `pnpm exec playwright test --grep "rate limit"` — filter by title.
- `pnpm exec playwright show-report` — re-open the last HTML report.
- Traces (captured on first retry) open via `pnpm exec playwright show-trace path/to/trace.zip`.

## Graceful degradation

Every backend-dependent spec probes the API with `api.isReachable()` (or a
targeted capability check like `api.popupsExist()`) and calls `test.skip(...)`
with a descriptive reason when the dependency is missing. Running E2E against
a partially-deployed system therefore surfaces skips rather than false
failures — a deliberate trade-off for a monorepo where frontend and backend
agents ship in parallel.

## CI integration (follow-up ticket)

This PR intentionally does NOT add a `.github/workflows/e2e.yml` because
Agent K's CI hardening just landed and E2E needs its own dedicated lane with
Postgres + seeded admin. Sketch for the upcoming workflow:

```yaml
# .github/workflows/e2e.yml (TODO — do not add in this PR)
name: E2E
on:
    pull_request:
        branches: [main]
    workflow_dispatch: {}

jobs:
    e2e:
        runs-on: ubuntu-latest
        services:
            postgres:
                image: postgres:16
                env:
                    POSTGRES_PASSWORD: postgres
                    POSTGRES_DB: swings_e2e
                ports: ['5432:5432']
                options: >-
                    --health-cmd=pg_isready --health-interval=5s
                    --health-timeout=2s --health-retries=20
        env:
            DATABASE_URL: postgres://postgres:postgres@localhost:5432/swings_e2e
            ADMIN_EMAIL: admin@swings.test
            ADMIN_PASSWORD: admin-password-1234
            JWT_SECRET: ci-only-secret
            STRIPE_SECRET_KEY: sk_test_dummy
            STRIPE_WEBHOOK_SECRET: whsec_dummy
            FRONTEND_URL: http://localhost:4173
            VITE_API_URL: http://127.0.0.1:3001
        steps:
            - uses: actions/checkout@v4
            - uses: pnpm/action-setup@v4
            - uses: actions/setup-node@v4
              with:
                  node-version: '24'
                  cache: 'pnpm'
            - uses: dtolnay/rust-toolchain@stable
            - run: pnpm install --frozen-lockfile
            - run: pnpm exec playwright install --with-deps
            - name: Start backend
              run: cd backend && cargo run &
            - run: pnpm test:e2e
            - if: always()
              uses: actions/upload-artifact@v4
              with:
                  name: playwright-report
                  path: playwright-report/
```
