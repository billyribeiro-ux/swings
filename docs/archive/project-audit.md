# Project Audit

## Section 1: Frontend

### 1.1 package.json

```json
// package.json
{
	"name": "swings",
	"private": true,
	"version": "0.0.1",
	"type": "module",
	"engines": {
		"node": ">=24.14.1"
	},
	"pnpm": {
		"peerDependencyRules": {
			"allowedVersions": {
				"vite-plugin-devtools-json>vite": "*"
			}
		}
	},
	"scripts": {
		"dev": "vite dev",
		"dev:api": "cd backend && cargo run",
		"dev:all": "concurrently -k -n web,api -c cyan,magenta \"pnpm dev\" \"cd backend && cargo run\"",
		"build": "rm -rf .svelte-kit && vite build",
		"preview": "vite preview",
		"prepare": "(svelte-kit sync || echo '') && simple-git-hooks",
		"check": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json",
		"check:watch": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json --watch",
		"lint": "eslint .",
		"format": "prettier --write .",
		"test:unit": "vitest",
		"test:browser": "vitest --config vitest.browser.config.ts --run",
		"test": "npm run test:unit -- --run && npm run test:e2e",
		"test:e2e": "playwright test",
		"ci:seo": "node scripts/seo-audit.mjs",
		"ci:frontend": "pnpm check && pnpm lint && pnpm ci:seo && pnpm test:unit -- --run && pnpm build",
		"ci:backend": "cargo fmt --manifest-path backend/Cargo.toml --check && cargo clippy --manifest-path backend/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path backend/Cargo.toml && cargo build --manifest-path backend/Cargo.toml",
		"ci:all": "pnpm ci:frontend && pnpm ci:backend"
	},
	"simple-git-hooks": {
		"pre-commit": "pnpm lint && pnpm test:unit -- --run"
	},
	"devDependencies": {
		"@eslint/compat": "^2.0.5",
		"@eslint/js": "^10.0.1",
		"@playwright/test": "^1.59.1",
		"@sveltejs/adapter-auto": "^7.0.1",
		"@sveltejs/adapter-netlify": "^6.0.4",
		"@sveltejs/adapter-vercel": "^6.3.3",
		"@sveltejs/kit": "^2.57.1",
		"@sveltejs/vite-plugin-svelte": "^7.0.0",
		"@types/node": "^25.6.0",
		"@types/three": "^0.183.1",
		"@vitest/browser-playwright": "^4.1.4",
		"concurrently": "^9.2.1",
		"eslint": "^10.2.0",
		"eslint-config-prettier": "^10.1.8",
		"eslint-plugin-svelte": "^3.17.0",
		"globals": "^17.4.0",
		"playwright": "^1.59.1",
		"prettier": "^3.8.2",
		"prettier-plugin-svelte": "^3.5.1",
		"simple-git-hooks": "^2.13.1",
		"svelte": "^5.55.3",
		"svelte-check": "^4.4.6",
		"typescript": "^6.0.2",
		"typescript-eslint": "^8.58.1",
		"vite": "^8.0.8",
		"vite-plugin-devtools-json": "^1.0.0",
		"vitest": "^4.1.4",
		"vitest-browser-svelte": "^2.1.0"
	},
	"dependencies": {
		"@date-fns/tz": "^1.4.1",
		"@threlte/core": "^8.5.9",
		"@threlte/extras": "^9.14.5",
		"@tiptap/core": "^3.22.3",
		"@tiptap/extension-character-count": "^3.22.3",
		"@tiptap/extension-code-block-lowlight": "^3.22.3",
		"@tiptap/extension-color": "^3.22.3",
		"@tiptap/extension-highlight": "^3.22.3",
		"@tiptap/extension-image": "^3.22.3",
		"@tiptap/extension-link": "^3.22.3",
		"@tiptap/extension-placeholder": "^3.22.3",
		"@tiptap/extension-subscript": "^3.22.3",
		"@tiptap/extension-superscript": "^3.22.3",
		"@tiptap/extension-table": "^3.22.3",
		"@tiptap/extension-table-cell": "^3.22.3",
		"@tiptap/extension-table-header": "^3.22.3",
		"@tiptap/extension-table-row": "^3.22.3",
		"@tiptap/extension-task-item": "^3.22.3",
		"@tiptap/extension-task-list": "^3.22.3",
		"@tiptap/extension-text-align": "^3.22.3",
		"@tiptap/extension-text-style": "^3.22.3",
		"@tiptap/extension-typography": "^3.22.3",
		"@tiptap/extension-underline": "^3.22.3",
		"@tiptap/extension-youtube": "^3.22.3",
		"@tiptap/pm": "^3.22.3",
		"@tiptap/starter-kit": "^3.22.3",
		"@types/d3": "^7.4.3",
		"apexcharts": "^5.10.6",
		"d3": "^7.9.0",
		"date-fns": "^4.1.0",
		"gsap": "^3.14.2",
		"lowlight": "^3.3.0",
		"phosphor-svelte": "^3.1.0",
		"stripe": "^22.0.1",
		"three": "^0.183.2"
	}
}
```

### 1.2 svelte.config.js

```javascript
// svelte.config.js
import process from 'node:process';
import adapter from '@sveltejs/adapter-vercel';

/**
 * Service worker registration policy (must stay aligned with `src/hooks.client.ts` + `$lib/client/service-worker-dev-policy`).
 * @see https://svelte.dev/docs/kit/configuration#serviceWorker
 */
const allowServiceWorkerInDev =
	process.env.PUBLIC_SERVICE_WORKER_IN_DEV === '1' ||
	process.env.PUBLIC_SERVICE_WORKER_IN_DEV === 'true';

const registerServiceWorker = process.env.NODE_ENV === 'production' || allowServiceWorkerInDev;

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter({
			runtime: 'nodejs22.x'
		}),
		serviceWorker: {
			register: registerServiceWorker
		},
		prerender: {
			handleHttpError: 'warn',
			handleMissingId: 'warn',
			crawl: true,
			entries: [
				'/',
				'/about',
				'/courses',
				'/blog',
				'/pricing',
				'/pricing/monthly',
				'/pricing/annual'
			]
		}
	}
};

export default config;
```

### 1.3 vite.config.ts

```typescript
// vite.config.ts
import devtoolsJson from 'vite-plugin-devtools-json';
import { defineConfig } from 'vitest/config';
import { sveltekit } from '@sveltejs/kit/vite';

export default defineConfig({
	plugins: [sveltekit(), devtoolsJson()],
	build: {
		// Large, intentional vendor chunks in this app make Vite's default 500k warning too noisy.
		chunkSizeWarningLimit: 10000
	},
	server: {
		proxy: {
			// Rust API (pnpm dev + cargo run in backend). Browser uses same-origin /api via getPublicApiBase().
			'/api': {
				target: 'http://127.0.0.1:3001',
				changeOrigin: true
			}
		}
	},
	// Vitest 4: files using `vitest/browser` must run in browser mode (`pnpm exec playwright install` + dedicated project).
	// Default `pnpm test:unit` runs Node tests only; e2e/ is Playwright (`pnpm exec playwright test`).
	test: {
		include: ['src/**/*.{test,spec}.ts'],
		exclude: ['src/**/*.svelte.spec.ts']
	}
});
```

### 1.4 tsconfig.json

```json
// tsconfig.json
{
	"extends": "./.svelte-kit/tsconfig.json",
	"compilerOptions": {
		"rewriteRelativeImportExtensions": true,
		"allowJs": true,
		"checkJs": true,
		"esModuleInterop": true,
		"forceConsistentCasingInFileNames": true,
		"resolveJsonModule": true,
		"skipLibCheck": true,
		"sourceMap": true,
		"strict": true,
		"moduleResolution": "bundler"
	}
	// Path aliases are handled by https://svelte.dev/docs/kit/configuration#alias
	// except $lib which is handled by https://svelte.dev/docs/kit/configuration#files
	//
	// To make changes to top-level options such as include and exclude, we recommend extending
	// the generated config; see https://svelte.dev/docs/kit/configuration#typescript
}
```

### 1.5 src/app.html

```html
// src/app.html
<!doctype html>
<html lang="en">
	<head>
		<meta charset="utf-8" />
		<meta name="viewport" content="width=device-width, initial-scale=1" />
		<meta name="theme-color" content="#0c1b2e" />
		<meta name="color-scheme" content="dark light" />
		<link rel="icon" href="%sveltekit.assets%/favicon.svg" type="image/svg+xml" />
		<link rel="preconnect" href="https://fonts.googleapis.com" />
		<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
		<link
			href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=Montserrat:wght@400;500;600;700;800&display=swap"
			rel="stylesheet"
			media="print"
			onload="this.media = 'all'"
		/>
		<noscript>
			<link
				href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=Montserrat:wght@400;500;600;700;800&display=swap"
				rel="stylesheet"
			/>
		</noscript>
		%sveltekit.head%
	</head>

	<body data-sveltekit-preload-data="hover">
		<div style="display: contents">%sveltekit.body%</div>
	</body>
</html>
```

### 1.6 src/app.d.ts

```typescript
// src/app.d.ts
// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		interface Error {
			message: string;
			code?: string;
			id?: string;
		}

		interface Locals {
			user?: {
				id: string;
				email: string;
				name: string;
				role: 'member' | 'admin';
			};
		}

		// PageData is augmented per-route via `+page.ts` / `+page.server.ts` load
		// return types — leave the shared shape empty so route-specific data flows
		// through the generated `PageProps` types.
		// eslint-disable-next-line @typescript-eslint/no-empty-object-type
		interface PageData {}

		// eslint-disable-next-line @typescript-eslint/no-empty-object-type
		interface PageState {}

		// eslint-disable-next-line @typescript-eslint/no-empty-object-type
		interface Platform {}
	}
}

export {};
```

### 1.7 Directory tree of `src/`

```
src/
├── lib
│   ├── analytics
│   │   ├── AnalyticsBeacon.svelte
│   │   ├── constants.ts
│   │   └── cta.ts
│   ├── api
│   │   ├── client.ts
│   │   ├── publicApiBase.ts
│   │   ├── resolvePublicApiBase.test.ts
│   │   ├── resolvePublicApiBase.ts
│   │   └── types.ts
│   ├── assets
│   │   └── favicon.svg
│   ├── client
│   │   ├── service-worker-dev-policy.test.ts
│   │   └── service-worker-dev-policy.ts
│   ├── components
│   │   ├── admin
│   │   │   ├── analytics
│   │   │   │   └── AnalyticsDashboard3d.svelte
│   │   │   ├── admin-nav.ts
│   │   │   ├── AdminSiteBar.svelte
│   │   │   └── CommandPalette.svelte
│   │   ├── charts
│   │   │   ├── CohortHeatmap.svelte
│   │   │   ├── FunnelChart.svelte
│   │   │   ├── GrowthChart.svelte
│   │   │   ├── HeroChart.svelte
│   │   │   ├── MiniChart.svelte
│   │   │   └── RevenueChart.svelte
│   │   ├── editor
│   │   │   ├── BlogEditor.svelte
│   │   │   ├── EditorToolbar.svelte
│   │   │   ├── MediaLibrary.svelte
│   │   │   ├── post-editor-utils.test.ts
│   │   │   ├── post-editor-utils.ts
│   │   │   ├── PostEditor.svelte
│   │   │   └── SlashMenu.svelte
│   │   ├── landing
│   │   │   ├── About.svelte
│   │   │   ├── Courses.svelte
│   │   │   ├── FinalCta.svelte
│   │   │   ├── GreeksPdfCta.svelte
│   │   │   ├── Hero.svelte
│   │   │   ├── Pricing.svelte
│   │   │   ├── SampleAlertCard.svelte
│   │   │   ├── ScheduleCountdowns.svelte
│   │   │   ├── Testimonials.svelte
│   │   │   ├── WhatYouGet.svelte
│   │   │   ├── WhoItsFor.svelte
│   │   │   └── WhyDifferent.svelte
│   │   ├── popups
│   │   │   ├── PopupEngine.svelte
│   │   │   └── PopupRenderer.svelte
│   │   ├── traders
│   │   │   ├── TraderCard.svelte
│   │   │   ├── TraderProfile.svelte
│   │   │   └── TradersModal.svelte
│   │   └── ui
│   │       ├── Button.svelte
│   │       ├── ConfirmDialog.svelte
│   │       ├── DateRangePicker.svelte
│   │       ├── EmptyState.svelte
│   │       ├── FloatingButton.svelte
│   │       ├── Footer.svelte
│   │       ├── LivePriceTicker.svelte
│   │       ├── Nav.svelte
│   │       ├── ScrollReveal.svelte
│   │       ├── SectionHeader.svelte
│   │       ├── Skeleton.svelte
│   │       └── Toast.svelte
│   ├── data
│   │   ├── alerts.ts
│   │   ├── courses.ts
│   │   ├── pricing.ts
│   │   └── traders.ts
│   ├── seo
│   │   ├── config.ts
│   │   ├── jsonld.ts
│   │   └── Seo.svelte
│   ├── stores
│   │   ├── auth.svelte.ts
│   │   ├── modal.svelte.ts
│   │   └── toast.svelte.ts
│   ├── utils
│   │   ├── animations.ts
│   │   ├── apexCandlestick.ts
│   │   ├── chartData.ts
│   │   ├── chartThemes.ts
│   │   ├── checkout.ts
│   │   ├── tradingSchedule.test.ts
│   │   └── tradingSchedule.ts
│   └── index.ts
├── routes
│   ├── about
│   │   └── +page.svelte
│   ├── admin
│   │   ├── analytics
│   │   │   └── +page.svelte
│   │   ├── author
│   │   │   └── +page.svelte
│   │   ├── blog
│   │   │   ├── [id]
│   │   │   │   └── +page.svelte
│   │   │   ├── categories
│   │   │   │   └── +page.svelte
│   │   │   ├── media
│   │   │   │   └── +page.svelte
│   │   │   ├── new
│   │   │   │   └── +page.svelte
│   │   │   ├── tags
│   │   │   │   └── +page.svelte
│   │   │   └── +page.svelte
│   │   ├── coupons
│   │   │   ├── [id]
│   │   │   │   └── +page.svelte
│   │   │   ├── new
│   │   │   │   └── +page.svelte
│   │   │   └── +page.svelte
│   │   ├── courses
│   │   │   ├── [id]
│   │   │   │   └── +page.svelte
│   │   │   ├── new
│   │   │   │   └── +page.svelte
│   │   │   └── +page.svelte
│   │   ├── forgot-password
│   │   │   ├── +page.svelte
│   │   │   └── +page.ts
│   │   ├── members
│   │   │   ├── [id]
│   │   │   │   └── +page.svelte
│   │   │   └── +page.svelte
│   │   ├── popups
│   │   │   ├── [id]
│   │   │   │   └── +page.svelte
│   │   │   ├── new
│   │   │   │   └── +page.svelte
│   │   │   └── +page.svelte
│   │   ├── reset-password
│   │   │   ├── +page.svelte
│   │   │   └── +page.ts
│   │   ├── settings
│   │   │   └── +page.svelte
│   │   ├── subscriptions
│   │   │   ├── plans
│   │   │   │   └── +page.svelte
│   │   │   └── +page.svelte
│   │   ├── watchlists
│   │   │   ├── [id]
│   │   │   │   └── +page.svelte
│   │   │   ├── new
│   │   │   │   └── +page.svelte
│   │   │   └── +page.svelte
│   │   ├── +layout.svelte
│   │   ├── +layout.ts
│   │   └── +page.svelte
│   ├── api
│   │   ├── create-checkout-session
│   │   │   └── +server.ts
│   │   └── greeks-pdf
│   │       └── +server.ts
│   ├── blog
│   │   ├── [slug]
│   │   │   ├── +page.server.ts
│   │   │   ├── +page.svelte
│   │   │   └── +page.ts
│   │   ├── category
│   │   │   └── [slug]
│   │   │       ├── +page.svelte
│   │   │       └── +page.ts
│   │   ├── tag
│   │   │   └── [slug]
│   │   │       ├── +page.svelte
│   │   │       └── +page.ts
│   │   ├── +page.server.ts
│   │   └── +page.svelte
│   ├── courses
│   │   ├── [slug]
│   │   │   ├── +page.svelte
│   │   │   └── +page.ts
│   │   └── +page.svelte
│   ├── dashboard
│   │   ├── account
│   │   │   └── +page.svelte
│   │   ├── courses
│   │   │   ├── [slug]
│   │   │   │   ├── +page.svelte
│   │   │   │   └── +page.ts
│   │   │   └── +page.svelte
│   │   ├── watchlists
│   │   │   ├── [id]
│   │   │   │   └── +page.svelte
│   │   │   └── +page.svelte
│   │   ├── +layout.svelte
│   │   ├── +layout.ts
│   │   └── +page.svelte
│   ├── login
│   │   ├── +page.svelte
│   │   └── +page.ts
│   ├── pricing
│   │   ├── annual
│   │   │   └── +page.svelte
│   │   ├── monthly
│   │   │   └── +page.svelte
│   │   ├── +page.svelte
│   │   └── +page.ts
│   ├── privacy
│   │   └── +page.svelte
│   ├── register
│   │   ├── +page.svelte
│   │   └── +page.ts
│   ├── sitemap.xml
│   │   └── +server.ts
│   ├── success
│   │   ├── +page.svelte
│   │   └── +page.ts
│   ├── terms
│   │   └── +page.svelte
│   ├── +layout.svelte
│   ├── +layout.ts
│   ├── +page.svelte
│   ├── layout.css
│   └── page.svelte.spec.ts
├── styles
│   ├── global.css
│   ├── reset.css
│   └── tokens.css
├── app.css
├── app.d.ts
├── app.html
├── hooks.client.ts
├── hooks.server.ts
└── service-worker.ts
```

### 1.8 Layout files (`+layout.svelte`, `+layout.ts`, `+layout.server.ts`)

#### `src/routes/+layout.svelte`

```svelte
<script lang="ts">
	import '../app.css';
	import { browser } from '$app/environment';
	import { page } from '$app/state';
	import Nav from '$lib/components/ui/Nav.svelte';
	import Footer from '$lib/components/ui/Footer.svelte';
	import FloatingButton from '$lib/components/ui/FloatingButton.svelte';
	import TradersModal from '$lib/components/traders/TradersModal.svelte';
	import AnalyticsBeacon from '$lib/analytics/AnalyticsBeacon.svelte';
	import AdminSiteBar from '$lib/components/admin/AdminSiteBar.svelte';
	import PopupEngine from '$lib/components/popups/PopupEngine.svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { organizationSchema, webSiteSchema, buildJsonLd } from '$lib/seo/jsonld';
	import { SITE } from '$lib/seo/config';

	let { children } = $props();

	const appRoutes = ['/dashboard', '/admin', '/login', '/register'];
	const isAppRoute = $derived(appRoutes.some((r) => page.url.pathname.startsWith(r)));
	const noindexRoutes = ['/success'];
	const isNoindexRoute = $derived(
		isAppRoute || noindexRoutes.some((r) => page.url.pathname.startsWith(r))
	);

	/** Offset nav when WordPress-style admin bar is visible */
	const wpAdminOffset = $derived(
		!isAppRoute &&
			auth.isAuthenticated &&
			auth.isAdmin &&
			!['/dashboard', '/login', '/register'].some((p) => page.url.pathname.startsWith(p))
	);

	const _globalJsonLd = buildJsonLd([organizationSchema(), webSiteSchema()]);
</script>

// src/routes/+layout.svelte
<svelte:head>
	<link rel="icon" href="/favicon.svg" type="image/svg+xml" />
	<meta name="author" content="Billy Ribeiro" />
	<meta name="publisher" content={SITE.name} />
	{#if isNoindexRoute}
		<meta name="robots" content="noindex, nofollow" />
	{/if}
	<script type="application/ld+json">
{_globalJsonLd}
	</script>
</svelte:head>

<AnalyticsBeacon />

{#if isAppRoute}
	{@render children()}
{:else}
	<div
		class="public-shell"
		class:public-shell--wp-admin={wpAdminOffset}
		data-sveltekit-preload-data="hover"
	>
		<AdminSiteBar />
		<Nav />

		<main>
			{@render children()}
		</main>

		<Footer />
	</div>
	<FloatingButton />
	<TradersModal />
{/if}

{#if browser}
	<PopupEngine />
{/if}

<style>
	:global(.public-shell--wp-admin .nav) {
		top: 2.5rem;
	}
</style>
```

#### `src/routes/+layout.ts`

```typescript
// src/routes/+layout.ts
export const prerender = true;
export const trailingSlash = 'never';
```

#### `src/routes/admin/+layout.svelte`

```svelte
<script lang="ts">
	import { browser } from '$app/environment';
	import { page } from '$app/state';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { AuthResponse, UserResponse } from '$lib/api/types';
	import ChartBar from 'phosphor-svelte/lib/ChartBar';
	import PresentationChart from 'phosphor-svelte/lib/PresentationChart';
	import Users from 'phosphor-svelte/lib/Users';
	import ListChecks from 'phosphor-svelte/lib/ListChecks';
	import Article from 'phosphor-svelte/lib/Article';
	import UserCircle from 'phosphor-svelte/lib/UserCircle';
	import SignOut from 'phosphor-svelte/lib/SignOut';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import CaretDoubleLeft from 'phosphor-svelte/lib/CaretDoubleLeft';
	import CaretDoubleRight from 'phosphor-svelte/lib/CaretDoubleRight';
	import List from 'phosphor-svelte/lib/List';
	import X from 'phosphor-svelte/lib/X';
	import CaretDown from 'phosphor-svelte/lib/CaretDown';
	import GraduationCap from 'phosphor-svelte/lib/GraduationCap';
	import CreditCard from 'phosphor-svelte/lib/CreditCard';
	import Tag from 'phosphor-svelte/lib/Tag';
	import ChatCircleDots from 'phosphor-svelte/lib/ChatCircleDots';
	import Gear from 'phosphor-svelte/lib/Gear';
	import CommandPalette from '$lib/components/admin/CommandPalette.svelte';
	import { SITE } from '$lib/seo/config';
	import {
		blogAdminItems,
		courseAdminItems,
		couponAdminItems,
		popupAdminItems,
		publicAdminRoutes,
		subscriptionAdminItems
	} from '$lib/components/admin/admin-nav';

	let { children } = $props();

	let paletteOpen = $state(false);

	const SIDEBAR_COLLAPSE_KEY = 'admin-sidebar-collapsed';
	let sidebarCollapsed = $state(false);

	function handleGlobalKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
			e.preventDefault();
			paletteOpen = !paletteOpen;
		}
	}

	$effect(() => {
		window.addEventListener('keydown', handleGlobalKey);
		if (browser) {
			sidebarCollapsed = localStorage.getItem(SIDEBAR_COLLAPSE_KEY) === '1';
		}
		return () => window.removeEventListener('keydown', handleGlobalKey);
	});

	let mobileMenuOpen = $state(false);
	let blogSubmenuOpen = $state(false);
	let courseSubmenuOpen = $state(false);
	let subscriptionSubmenuOpen = $state(false);
	let couponSubmenuOpen = $state(false);
	let popupSubmenuOpen = $state(false);

	function toggleSidebarCollapsed() {
		sidebarCollapsed = !sidebarCollapsed;
		if (typeof localStorage !== 'undefined') {
			localStorage.setItem(SIDEBAR_COLLAPSE_KEY, sidebarCollapsed ? '1' : '0');
		}
		if (sidebarCollapsed) {
			blogSubmenuOpen = false;
			courseSubmenuOpen = false;
			subscriptionSubmenuOpen = false;
			couponSubmenuOpen = false;
			popupSubmenuOpen = false;
		}
	}

	$effect(() => {
		if (sidebarCollapsed) {
			blogSubmenuOpen = false;
			courseSubmenuOpen = false;
			subscriptionSubmenuOpen = false;
			couponSubmenuOpen = false;
			popupSubmenuOpen = false;
		}
	});

	const isPublicRoute = $derived(publicAdminRoutes.some((r) => page.url.pathname.startsWith(r)));

	/** True only after /api/auth/me succeeds — avoids child pages firing admin APIs with stale localStorage JWTs. */
	let adminSessionReady = $state(false);
	let adminSessionCheckInFlight = $state(false);

	$effect(() => {
		if (isPublicRoute) {
			adminSessionReady = true;
			return;
		}
		if (!auth.isAuthenticated || !auth.isAdmin) {
			adminSessionReady = false;
			adminSessionCheckInFlight = false;
			return;
		}
		if (adminSessionReady) return;
		if (adminSessionCheckInFlight) return;

		adminSessionCheckInFlight = true;
		void (async () => {
			try {
				const me = await api.get<UserResponse>('/api/auth/me');
				auth.setUser(me);
				adminSessionReady = true;
			} catch {
				auth.logout();
				adminSessionReady = false;
			} finally {
				adminSessionCheckInFlight = false;
			}
		})();
	});

	let email = $state('');
	let password = $state('');
	let loginError = $state('');
	let loginLoading = $state(false);

	async function handleLogin(e: Event) {
		e.preventDefault();
		loginError = '';
		loginLoading = true;

		try {
			const res = await api.post<AuthResponse>(
				'/api/auth/login',
				{ email, password },
				{ skipAuth: true }
			);

			if (res.user.role.toLowerCase() !== 'admin') {
				loginError = 'Access denied. Admin credentials required.';
				loginLoading = false;
				return;
			}

			auth.setAuth(res.user, res.access_token, res.refresh_token);
			adminSessionReady = true;
		} catch (err) {
			if (err instanceof ApiError) {
				loginError = err.status === 401 ? 'Invalid email or password' : err.message;
			} else {
				loginError = 'Something went wrong. Please try again.';
			}
		} finally {
			loginLoading = false;
		}
	}

	function handleLogout() {
		auth.logout();
	}

	const navItems = [
		{ href: '/admin', label: 'Dashboard', icon: ChartBar },
		{ href: '/admin/analytics', label: 'Analytics', icon: PresentationChart },
		{ href: '/admin/members', label: 'Members', icon: Users },
		{ href: '/admin/watchlists', label: 'Watchlists', icon: ListChecks },
		{ href: '/admin/author', label: 'Author Profile', icon: UserCircle }
	];
</script>

// src/routes/admin/+layout.svelte
{#if isPublicRoute}
	{@render children()}
{:else if !auth.isAuthenticated || !auth.isAdmin}
	<div class="admin-login">
		<div class="admin-login__card">
			<div class="admin-login__header">
				<a href="/" class="admin-login__logo">
					<span class="admin-login__logo-brand">{SITE.logoBrandPrimary}</span>
					<span class="admin-login__logo-accent">{SITE.logoBrandAccent}</span>
				</a>
				<span class="admin-login__badge">Admin</span>
				<h1 class="admin-login__title">Admin Login</h1>
				<p class="admin-login__subtitle">Enter your credentials to access the admin panel</p>
			</div>

			{#if loginError}
				<div class="admin-login__error">{loginError}</div>
			{/if}

			<form onsubmit={handleLogin} class="admin-login__form">
				<div class="admin-login__field">
					<label for="admin-email" class="admin-login__label">Email</label>
					<input
						id="admin-email"
						name="email"
						type="email"
						bind:value={email}
						required
						autocomplete="email"
						class="admin-login__input"
						placeholder="admin@example.com"
					/>
				</div>

				<div class="admin-login__field">
					<label for="admin-password" class="admin-login__label">Password</label>
					<input
						id="admin-password"
						name="password"
						type="password"
						bind:value={password}
						required
						autocomplete="current-password"
						class="admin-login__input"
						placeholder="Enter your password"
					/>
				</div>

				<button type="submit" disabled={loginLoading} class="admin-login__submit">
					{loginLoading ? 'Signing in...' : 'Sign In'}
				</button>
			</form>

			<a href="/admin/forgot-password" class="admin-login__forgot">Forgot password?</a>
			<a href="/" class="admin-login__back">← Back to site</a>
		</div>
	</div>
{:else if !adminSessionReady}
	<div class="admin-login">
		<div class="admin-login__card">
			<p class="admin-login__subtitle">Validating session…</p>
		</div>
	</div>
{:else}
	<div class="admin">
		<!-- Mobile Menu Overlay -->
		{#if mobileMenuOpen}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="admin__overlay"
				role="button"
				tabindex="-1"
				onclick={() => (mobileMenuOpen = false)}
				onkeydown={(e) => e.key === 'Escape' && (mobileMenuOpen = false)}
			></div>
		{/if}

		<!-- Mobile Top Bar -->
		<header class="admin__mobile-header">
			<button class="admin__menu-toggle" onclick={() => (mobileMenuOpen = !mobileMenuOpen)}>
				{#if mobileMenuOpen}
					<X size={24} weight="bold" />
				{:else}
					<List size={24} weight="bold" />
				{/if}
			</button>
			<a href="/" class="admin__mobile-logo">
				<span class="admin__logo-brand">{SITE.logoBrandPrimary}</span>
				<span class="admin__logo-accent">{SITE.logoBrandAccent}</span>
			</a>
			<span class="admin__badge admin__badge--mobile">Admin</span>
		</header>

		<aside
			class="admin__sidebar"
			class:admin__sidebar--open={mobileMenuOpen}
			class:admin__sidebar--collapsed={sidebarCollapsed}
		>
			<div class="admin__sidebar-top">
				<a href="/" class="admin__logo" title={SITE.name}>
					<span class="admin__logo-brand">{SITE.logoBrandPrimary}</span>
					<span class="admin__logo-accent">{SITE.logoBrandAccent}</span>
				</a>
				<div class="admin__sidebar-top-actions">
					<span class="admin__badge">Admin</span>
					<button
						type="button"
						class="admin__sidebar-pin"
						onclick={toggleSidebarCollapsed}
						aria-pressed={sidebarCollapsed}
						aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
						title={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
					>
						{#if sidebarCollapsed}
							<CaretDoubleRight size={18} weight="bold" />
						{:else}
							<CaretDoubleLeft size={18} weight="bold" />
						{/if}
					</button>
				</div>
			</div>

			<nav class="admin__nav">
				{#each navItems as item (item.href)}
					<a
						href={item.href}
						class="admin__nav-link"
						class:admin__nav-link--active={page.url.pathname === item.href}
						onclick={() => (mobileMenuOpen = false)}
					>
						<item.icon size={20} weight="duotone" />
						<span>{item.label}</span>
					</a>
				{/each}

				<div class="admin__nav-section">
					<button
						class="admin__nav-link admin__nav-link--header"
						onclick={() => (blogSubmenuOpen = !blogSubmenuOpen)}
					>
						<Article size={20} weight="duotone" />
						<span>Blog</span>
						<CaretDown
							size={16}
							class="admin__nav-caret{blogSubmenuOpen ? ' admin__nav-caret--open' : ''}"
						/>
					</button>
					{#if blogSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each blogAdminItems as item (item.href)}
								<a
									href={item.href}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										blogSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<div class="admin__nav-section">
					<button
						class="admin__nav-link admin__nav-link--header"
						onclick={() => (courseSubmenuOpen = !courseSubmenuOpen)}
					>
						<GraduationCap size={20} weight="duotone" />
						<span>Courses</span>
						<CaretDown
							size={16}
							class="admin__nav-caret{courseSubmenuOpen ? ' admin__nav-caret--open' : ''}"
						/>
					</button>
					{#if courseSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each courseAdminItems as item (item.href)}
								<a
									href={item.href}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										courseSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<div class="admin__nav-section">
					<button
						class="admin__nav-link admin__nav-link--header"
						onclick={() => (subscriptionSubmenuOpen = !subscriptionSubmenuOpen)}
					>
						<CreditCard size={20} weight="duotone" />
						<span>Subscriptions</span>
						<CaretDown
							size={16}
							class="admin__nav-caret{subscriptionSubmenuOpen ? ' admin__nav-caret--open' : ''}"
						/>
					</button>
					{#if subscriptionSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each subscriptionAdminItems as item (item.href)}
								<a
									href={item.href}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										subscriptionSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<div class="admin__nav-section">
					<button
						class="admin__nav-link admin__nav-link--header"
						onclick={() => (couponSubmenuOpen = !couponSubmenuOpen)}
					>
						<Tag size={20} weight="duotone" />
						<span>Coupons</span>
						<CaretDown
							size={16}
							class="admin__nav-caret{couponSubmenuOpen ? ' admin__nav-caret--open' : ''}"
						/>
					</button>
					{#if couponSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each couponAdminItems as item (item.href)}
								<a
									href={item.href}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										couponSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<div class="admin__nav-section">
					<button
						class="admin__nav-link admin__nav-link--header"
						onclick={() => (popupSubmenuOpen = !popupSubmenuOpen)}
					>
						<ChatCircleDots size={20} weight="duotone" />
						<span>Popups</span>
						<CaretDown
							size={16}
							class="admin__nav-caret{popupSubmenuOpen ? ' admin__nav-caret--open' : ''}"
						/>
					</button>
					{#if popupSubmenuOpen}
						<div class="admin__nav-submenu">
							{#each popupAdminItems as item (item.href)}
								<a
									href={item.href}
									class="admin__nav-sublink"
									onclick={() => {
										mobileMenuOpen = false;
										popupSubmenuOpen = false;
									}}
								>
									{item.label}
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<a
					href="/admin/settings"
					class="admin__nav-link"
					class:admin__nav-link--active={page.url.pathname === '/admin/settings'}
					onclick={() => (mobileMenuOpen = false)}
				>
					<Gear size={20} weight="duotone" />
					<span>Settings</span>
				</a>
			</nav>

			<div class="admin__sidebar-footer">
				<a
					href="/dashboard"
					class="admin__nav-link admin__nav-link--back"
					onclick={() => (mobileMenuOpen = false)}
				>
					<ArrowLeft size={18} />
					<span>Member Dashboard</span>
				</a>
				<button onclick={handleLogout} class="admin__logout">
					<SignOut size={20} weight="duotone" />
					<span>Sign Out</span>
				</button>
			</div>
		</aside>

		<div class="admin__main">
			<header class="admin__main-topbar">
				<a href="/" class="admin__view-site">View site</a>
			</header>
			<div class="admin__content">
				{@render children()}
			</div>
		</div>
		<CommandPalette open={paletteOpen} onClose={() => (paletteOpen = false)} />
	</div>
{/if}

<style>
	/* Mobile-first base styles */
	.admin {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
		background-color: var(--color-navy-deep);
	}

	.admin__overlay {
		display: none;
		position: fixed;
		inset: 0;
		background-color: rgba(0, 0, 0, 0.6);
		z-index: 40;
	}

	.admin__mobile-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.75rem 1rem;
		background-color: var(--color-navy);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		position: sticky;
		top: 0;
		z-index: 30;
	}

	.admin__menu-toggle {
		width: 2.75rem;
		height: 2.75rem;
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		cursor: pointer;
		transition: background-color 200ms var(--ease-out);
	}

	.admin__menu-toggle:hover {
		background-color: rgba(255, 255, 255, 0.1);
	}

	.admin__mobile-logo {
		display: flex;
		gap: 0.3rem;
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
	}

	.admin__badge--mobile {
		display: inline-block;
	}

	.admin__sidebar {
		position: fixed;
		top: 0;
		left: 0;
		width: 16rem;
		height: 100vh;
		background-color: var(--color-navy);
		border-right: 1px solid rgba(255, 255, 255, 0.06);
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		z-index: 50;
		transform: translateX(-100%);
		transition: transform 300ms var(--ease-out);
		overflow-x: hidden;
		overflow-y: auto;
		box-sizing: border-box;
	}

	.admin__sidebar--open {
		transform: translateX(0);
	}

	.admin__overlay {
		display: block;
	}

	.admin__sidebar-top {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 0.65rem;
		margin-bottom: 2rem;
		padding: 0 0.25rem;
		min-width: 0;
	}

	.admin__sidebar-top-actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 0.5rem;
		flex-wrap: nowrap;
		min-width: 0;
	}

	.admin__sidebar-pin {
		display: none;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		background: rgba(255, 255, 255, 0.05);
		color: var(--color-grey-400);
		cursor: pointer;
		flex-shrink: 0;
		transition:
			background-color 200ms var(--ease-out),
			color 200ms var(--ease-out);
	}

	.admin__sidebar-pin:hover {
		background: rgba(255, 255, 255, 0.1);
		color: var(--color-white);
	}

	.admin__main-topbar {
		display: none;
	}

	.admin__logo {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
		min-width: 0;
		line-height: 1.2;
	}

	.admin__logo-brand {
		color: var(--color-white);
	}

	.admin__logo-accent {
		color: var(--color-teal);
	}

	.admin__badge {
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: #f59e0b;
		background-color: rgba(245, 158, 11, 0.12);
		padding: 0.25rem 0.6rem;
		border-radius: var(--radius-full);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
		flex-shrink: 0;
	}

	.admin__badge--mobile {
		display: none;
	}

	.admin__nav {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		flex: 1;
	}

	.admin__nav-link {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem;
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		cursor: pointer;
		background: none;
		border: none;
		width: 100%;
		text-align: left;
		transition: all 200ms var(--ease-out);
	}

	.admin__nav-link:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.05);
	}

	.admin__nav-link--active {
		color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.1);
	}

	.admin__nav-link--back {
		color: var(--color-teal);
		margin-bottom: 0.5rem;
	}

	.admin__nav-link--header {
		justify-content: space-between;
	}

	:global(.admin__nav-caret) {
		transition: transform 200ms var(--ease-out);
	}

	:global(.admin__nav-caret--open) {
		transform: rotate(180deg);
	}

	.admin__nav-section {
		display: flex;
		flex-direction: column;
	}

	.admin__nav-submenu {
		display: flex;
		flex-direction: column;
	}

	.admin__nav-sublink {
		display: block;
		padding: 0.5rem 0.75rem 0.5rem 2.75rem;
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		text-decoration: none;
		border-radius: var(--radius-md);
		transition: all 200ms var(--ease-out);
	}

	.admin__nav-sublink:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.05);
	}

	.admin__sidebar-footer {
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		padding-top: 1rem;
	}

	.admin__logout {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		width: 100%;
		padding: 0.75rem;
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		background: none;
		border: none;
		cursor: pointer;
		transition: all 200ms var(--ease-out);
	}

	.admin__logout:hover {
		color: #fca5a5;
		background-color: rgba(239, 68, 68, 0.08);
	}

	.admin__main {
		flex: 1 1 auto;
		min-width: 0;
		overflow-y: auto;
	}

	.admin__content {
		padding: 1rem;
	}

	/* Tablet breakpoint (768px+) */
	@media (min-width: 768px) {
		.admin {
			flex-direction: row;
		}

		.admin__mobile-header {
			display: none;
		}

		.admin__sidebar {
			position: sticky;
			top: 0;
			align-self: flex-start;
			height: 100vh;
			flex: 0 0 16rem;
			max-width: 16rem;
			width: 16rem;
			transform: translateX(0);
			transition:
				flex-basis 240ms var(--ease-out),
				max-width 240ms var(--ease-out),
				width 240ms var(--ease-out),
				padding 240ms var(--ease-out);
		}

		.admin__sidebar-pin {
			display: flex;
		}

		.admin__sidebar--collapsed {
			flex: 0 0 4.35rem;
			max-width: 4.35rem;
			width: 4.35rem;
			padding: 1rem 0.4rem;
			align-items: center;
		}

		.admin__sidebar--collapsed .admin__sidebar-top {
			align-items: center;
			padding: 0;
		}

		.admin__sidebar--collapsed .admin__sidebar-top-actions {
			justify-content: center;
			width: 100%;
		}

		.admin__sidebar--collapsed .admin__logo-accent,
		.admin__sidebar--collapsed .admin__badge {
			display: none;
		}

		.admin__sidebar--collapsed .admin__logo-brand {
			font-size: 0.65rem;
			line-height: 1.15;
		}

		.admin__sidebar--collapsed .admin__nav-link span,
		.admin__sidebar--collapsed .admin__logout span,
		.admin__sidebar--collapsed .admin__nav-link--back span {
			display: none;
		}

		.admin__sidebar--collapsed .admin__nav-link,
		.admin__sidebar--collapsed .admin__logout {
			justify-content: center;
		}

		.admin__sidebar--collapsed :global(.admin__nav-caret) {
			display: none;
		}

		.admin__sidebar--collapsed .admin__nav-submenu {
			display: none;
		}

		.admin__overlay {
			display: none !important;
		}

		.admin__main-topbar {
			display: flex;
			justify-content: flex-end;
			align-items: center;
			padding: 0.65rem 1.25rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
			background: rgba(0, 0, 0, 0.2);
		}

		.admin__view-site {
			font-size: var(--fs-sm);
			font-weight: var(--w-semibold);
			color: var(--color-teal-light);
			text-decoration: none;
		}

		.admin__view-site:hover {
			text-decoration: underline;
		}

		.admin__content {
			padding: 1.5rem;
		}
	}

	/* Desktop breakpoint (1024px+) */
	@media (min-width: 1024px) {
		.admin__content {
			padding: 2rem;
			max-width: 1400px;
		}
	}

	/* Admin Login */
	.admin-login {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
		background: linear-gradient(145deg, var(--color-navy-deep) 0%, var(--color-navy) 100%);
	}

	.admin-login__card {
		width: 100%;
		max-width: 26rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		padding: 1.5rem;
	}

	.admin-login__header {
		text-align: center;
		margin-bottom: 1.5rem;
	}

	.admin-login__logo {
		display: inline-flex;
		gap: 0.35rem;
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
		margin-bottom: 0.75rem;
	}

	.admin-login__logo-brand {
		color: var(--color-white);
	}

	.admin-login__logo-accent {
		color: var(--color-teal);
	}

	.admin-login__badge {
		display: inline-block;
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: #f59e0b;
		background-color: rgba(245, 158, 11, 0.12);
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: 1rem;
	}

	.admin-login__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 0.5rem;
	}

	.admin-login__subtitle {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.admin-login__error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: 1.5rem;
		text-align: center;
	}

	.admin-login__form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.admin-login__field {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.admin-login__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}

	.admin-login__input {
		width: 100%;
		padding: 0.75rem 1rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-base);
		transition: border-color 200ms var(--ease-out);
	}

	.admin-login__input::placeholder {
		color: var(--color-grey-500);
	}

	.admin-login__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.admin-login__submit {
		width: 100%;
		padding: 0.85rem;
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94));
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-base);
		border-radius: var(--radius-lg);
		border: none;
		cursor: pointer;
		transition:
			opacity 200ms var(--ease-out),
			transform 200ms var(--ease-out);
	}

	.admin-login__submit:hover:not(:disabled) {
		opacity: 0.9;
		transform: translateY(-1px);
	}

	.admin-login__submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.admin-login__forgot {
		display: block;
		text-align: center;
		margin-top: 1rem;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		text-decoration: none;
		font-weight: var(--w-medium);
		transition: opacity 200ms;
	}

	.admin-login__forgot:hover {
		opacity: 0.8;
	}

	.admin-login__back {
		display: block;
		text-align: center;
		margin-top: 0.75rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		transition: color 200ms;
	}

	.admin-login__back:hover {
		color: var(--color-teal);
	}

	@media (min-width: 480px) {
		.admin-login__card {
			padding: 2rem;
		}

		.admin-login__title {
			font-size: var(--fs-2xl);
		}

		.admin-login__form {
			gap: 1.25rem;
		}
	}

	@media (min-width: 768px) {
		.admin-login {
			padding: 2rem;
		}

		.admin-login__card {
			padding: 2.5rem;
		}
	}
</style>
```

#### `src/routes/admin/+layout.ts`

```typescript
// src/routes/admin/+layout.ts
export const prerender = false;
export const ssr = false;
```

#### `src/routes/dashboard/+layout.svelte`

```svelte
<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { onMount } from 'svelte';
	import House from 'phosphor-svelte/lib/House';
	import ListChecks from 'phosphor-svelte/lib/ListChecks';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import UserCircle from 'phosphor-svelte/lib/UserCircle';
	import SignOut from 'phosphor-svelte/lib/SignOut';
	import { SITE } from '$lib/seo/config';

	let { children } = $props();

	onMount(() => {
		if (!auth.isAuthenticated) {
			goto('/login');
		}
	});

	function handleLogout() {
		auth.logout();
		goto('/login');
	}

	const navItems = [
		{ href: '/dashboard', label: 'Overview', icon: House },
		{ href: '/dashboard/watchlists', label: 'Watchlists', icon: ListChecks },
		{ href: '/dashboard/courses', label: 'Courses', icon: BookOpen },
		{ href: '/dashboard/account', label: 'Account', icon: UserCircle }
	];
</script>

// src/routes/dashboard/+layout.svelte
{#if auth.isAuthenticated}
	<div class="dash">
		<aside class="dash__sidebar">
			<a href="/" class="dash__logo">
				<span class="dash__logo-brand">{SITE.logoBrandPrimary}</span>
				<span class="dash__logo-accent">{SITE.logoBrandAccent}</span>
			</a>

			<nav class="dash__nav">
				{#each navItems as item (item.href)}
					<a href={item.href} class="dash__nav-link">
						<item.icon size={20} weight="duotone" />
						<span>{item.label}</span>
					</a>
				{/each}
			</nav>

			<div class="dash__sidebar-footer">
				{#if auth.isAdmin}
					<a href="/admin" class="dash__nav-link dash__nav-link--admin"> Admin Panel </a>
				{/if}
				<button onclick={handleLogout} class="dash__logout">
					<SignOut size={20} weight="duotone" />
					<span>Sign Out</span>
				</button>
			</div>
		</aside>

		<div class="dash__main">
			<header class="dash__header">
				<div>
					<h2 class="dash__greeting">Welcome back, {auth.user?.name?.split(' ')[0]}</h2>
				</div>
			</header>

			<div class="dash__content">
				{@render children()}
			</div>
		</div>
	</div>
{/if}

<style>
	.dash {
		display: flex;
		min-height: 100vh;
		background-color: var(--color-navy-deep);
	}

	.dash__sidebar {
		width: 16rem;
		background-color: var(--color-navy);
		border-right: 1px solid rgba(255, 255, 255, 0.06);
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		position: sticky;
		top: 0;
		height: 100vh;
	}

	.dash__logo {
		display: flex;
		gap: 0.3rem;
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		text-decoration: none;
		margin-bottom: 2rem;
		padding: 0 0.5rem;
	}

	.dash__logo-brand {
		color: var(--color-white);
	}

	.dash__logo-accent {
		color: var(--color-teal);
	}

	.dash__nav {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		flex: 1;
	}

	.dash__nav-link {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.65rem 0.75rem;
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		transition: all 200ms var(--ease-out);
	}

	.dash__nav-link:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.05);
	}

	.dash__nav-link--admin {
		color: var(--color-teal);
		border: 1px solid rgba(15, 164, 175, 0.2);
		justify-content: center;
		margin-bottom: 0.5rem;
	}

	.dash__sidebar-footer {
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		padding-top: 1rem;
	}

	.dash__logout {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		width: 100%;
		padding: 0.65rem 0.75rem;
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		background: none;
		border: none;
		cursor: pointer;
		transition: all 200ms var(--ease-out);
	}

	.dash__logout:hover {
		color: #fca5a5;
		background-color: rgba(239, 68, 68, 0.08);
	}

	.dash__main {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow-y: auto;
	}

	.dash__header {
		padding: 1.5rem 2rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.dash__greeting {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.dash__content {
		flex: 1;
		padding: 2rem;
	}

	@media (max-width: 768px) {
		.dash {
			flex-direction: column;
		}

		.dash__sidebar {
			width: 100%;
			height: auto;
			position: relative;
			flex-direction: row;
			flex-wrap: wrap;
			align-items: center;
			padding: 1rem;
		}

		.dash__nav {
			flex-direction: row;
			gap: 0.25rem;
			flex: initial;
		}

		.dash__sidebar-footer {
			border-top: none;
			padding-top: 0;
			margin-left: auto;
		}

		.dash__content {
			padding: 1rem;
		}
	}
</style>
```

#### `src/routes/dashboard/+layout.ts`

```typescript
// src/routes/dashboard/+layout.ts
export const prerender = false;
export const ssr = false;
```

**Note:** No `+layout.server.ts` files exist in this repository.

### 1.9 `+page.server.ts` and `+server.ts` files

#### `src/routes/api/create-checkout-session/+server.ts`

```typescript
// src/routes/api/create-checkout-session/+server.ts
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import Stripe from 'stripe';
import { env } from '$env/dynamic/private';
import { env as publicEnv } from '$env/dynamic/public';

export const POST: RequestHandler = async ({ request }) => {
	try {
		const { priceId } = await request.json();

		if (!priceId) {
			return json({ error: 'Price ID is required' }, { status: 400 });
		}

		if (!env.STRIPE_SECRET_KEY) {
			return json({ error: 'Stripe is not configured' }, { status: 500 });
		}

		const stripe = new Stripe(env.STRIPE_SECRET_KEY, {
			apiVersion: '2026-03-25.dahlia'
		});

		const session = await stripe.checkout.sessions.create({
			mode: 'subscription',
			payment_method_types: ['card'],
			line_items: [
				{
					price: priceId,
					quantity: 1
				}
			],
			success_url: `${publicEnv.PUBLIC_APP_URL || 'http://localhost:5173'}/success?session_id={CHECKOUT_SESSION_ID}`,
			cancel_url: `${publicEnv.PUBLIC_APP_URL || 'http://localhost:5173'}/pricing?canceled=true`,
			allow_promotion_codes: true,
			billing_address_collection: 'required'
		});

		return json({ sessionId: session.id, url: session.url });
	} catch (error) {
		console.error('Stripe checkout error:', error);
		return json(
			{ error: error instanceof Error ? error.message : 'Failed to create checkout session' },
			{ status: 500 }
		);
	}
};
```

#### `src/routes/api/greeks-pdf/+server.ts`

```typescript
// src/routes/api/greeks-pdf/+server.ts
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request }) => {
	try {
		const { email } = await request.json();

		if (!email || !email.includes('@')) {
			return json({ error: 'Valid email is required' }, { status: 400 });
		}

		// TODO: Integrate with email service (e.g., SendGrid, Mailgun, Resend)
		// For now, we'll just log and return success
		console.log(`Greeks PDF requested by: ${email}`);

		// In production, you would:
		// 1. Add email to your mailing list (e.g., ConvertKit, Mailchimp)
		// 2. Send email with PDF attachment or download link
		// 3. Track conversion in analytics

		// Example with a hypothetical email service:
		// await emailService.send({
		//   to: email,
		//   subject: 'Your Free Options Greeks Guide',
		//   template: 'greeks-pdf',
		//   attachments: [{ filename: 'options-greeks-guide.pdf', path: '/pdfs/greeks.pdf' }]
		// });

		return json({
			success: true,
			message: 'PDF sent successfully'
		});
	} catch (error) {
		console.error('Greeks PDF request error:', error);
		return json({ error: 'Failed to process request. Please try again.' }, { status: 500 });
	}
};
```

#### `src/routes/blog/+page.server.ts`

```typescript
// src/routes/blog/+page.server.ts
import { getPublicApiBase } from '$lib/api/publicApiBase';
import type { PageServerLoad } from './$types';
import type { BlogPostListItem, BlogCategory, PaginatedResponse } from '$lib/api/types';

const API = getPublicApiBase();

export const load: PageServerLoad = async ({ url, fetch }) => {
	let page: number;
	try {
		page = Number(url.searchParams.get('page') || '1');
	} catch {
		page = 1;
	}
	const per_page = 12;

	const [postsRes, catsRes] = await Promise.allSettled([
		fetch(`${API}/api/blog/posts?page=${page}&per_page=${per_page}`),
		fetch(`${API}/api/blog/categories`)
	]);

	let posts: BlogPostListItem[] = [];
	let total = 0;
	let totalPages = 1;
	let categories: BlogCategory[] = [];

	if (postsRes.status === 'fulfilled' && postsRes.value.ok) {
		const data: PaginatedResponse<BlogPostListItem> = await postsRes.value.json();
		posts = data.data;
		total = data.total;
		totalPages = data.total_pages;
	}

	if (catsRes.status === 'fulfilled' && catsRes.value.ok) {
		categories = await catsRes.value.json();
	}

	return { posts, categories, total, totalPages, page };
};
```

#### `src/routes/blog/[slug]/+page.server.ts`

```typescript
// src/routes/blog/[slug]/+page.server.ts
import { getPublicApiBase } from '$lib/api/publicApiBase';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import type { BlogPostResponse } from '$lib/api/types';

const API = getPublicApiBase();

export const load: PageServerLoad = async ({ params, fetch }) => {
	const res = await fetch(`${API}/api/blog/posts/${params.slug}`);

	if (res.status === 404) {
		error(404, 'Post not found');
	}

	if (!res.ok) {
		error(500, 'Failed to load post');
	}

	const post: BlogPostResponse = await res.json();

	return { post };
};
```

#### `src/routes/sitemap.xml/+server.ts`

```typescript
// src/routes/sitemap.xml/+server.ts
import { getPublicApiBase } from '$lib/api/publicApiBase';
import { courses } from '$lib/data/courses';
import { SITE } from '$lib/seo/config';
import type { BlogCategory, BlogTag, BlogPostListItem, PaginatedResponse } from '$lib/api/types';
import type { RequestHandler } from './$types';

const API_BASE = getPublicApiBase();
const PAGE_SIZE = 200;

const staticPages = [
	{ path: '/', priority: '1.0', changefreq: 'weekly' },
	{ path: '/about', priority: '0.8', changefreq: 'monthly' },
	{ path: '/courses', priority: '0.9', changefreq: 'weekly' },
	{ path: '/blog', priority: '0.8', changefreq: 'weekly' },
	{ path: '/pricing', priority: '0.7', changefreq: 'monthly' },
	{ path: '/pricing/monthly', priority: '0.7', changefreq: 'monthly' },
	{ path: '/pricing/annual', priority: '0.7', changefreq: 'monthly' },
	{ path: '/terms', priority: '0.3', changefreq: 'yearly' },
	{ path: '/privacy', priority: '0.3', changefreq: 'yearly' }
];

export const GET: RequestHandler = async () => {
	const today = new Date().toISOString().split('T')[0];

	const coursePages: SitemapEntry[] = courses.map((c) => ({
		path: `/courses/${c.slug}`,
		priority: '0.8',
		changefreq: 'monthly',
		lastmod: today
	}));

	const staticEntries: SitemapEntry[] = staticPages.map((page) => ({ ...page, lastmod: today }));
	let blogPages: SitemapEntry[] = [];
	let categoryPages: SitemapEntry[] = [];
	let tagPages: SitemapEntry[] = [];

	try {
		const [postsRes, categoriesRes, tagsRes] = await Promise.all([
			fetch(`${API_BASE}/api/blog/posts?page=1&per_page=${PAGE_SIZE}`),
			fetch(`${API_BASE}/api/blog/categories`),
			fetch(`${API_BASE}/api/blog/tags`)
		]);

		if (postsRes.ok) {
			const postsPayload: PaginatedResponse<BlogPostListItem> = await postsRes.json();
			blogPages = postsPayload.data.map((post) => ({
				path: `/blog/${post.slug}`,
				priority: '0.7',
				changefreq: 'weekly',
				lastmod: (post.updated_at || post.published_at || post.created_at).split('T')[0]
			}));
		}

		if (categoriesRes.ok) {
			const categories: BlogCategory[] = await categoriesRes.json();
			categoryPages = categories.map((category) => ({
				path: `/blog/category/${category.slug}`,
				priority: '0.5',
				changefreq: 'weekly',
				lastmod: today
			}));
		}

		if (tagsRes.ok) {
			const tags: BlogTag[] = await tagsRes.json();
			tagPages = tags.map((tag) => ({
				path: `/blog/tag/${tag.slug}`,
				priority: '0.5',
				changefreq: 'weekly',
				lastmod: today
			}));
		}
	} catch (error) {
		console.warn('Sitemap dynamic URL generation failed, serving static-only sitemap:', error);
	}

	const allPages = [...staticEntries, ...coursePages, ...blogPages, ...categoryPages, ...tagPages];

	const xml = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${allPages
	.map(
		(p) => `  <url>
    <loc>${SITE.url}${p.path}</loc>
    <lastmod>${p.lastmod}</lastmod>
    <changefreq>${p.changefreq}</changefreq>
    <priority>${p.priority}</priority>
  </url>`
	)
	.join('\n')}
</urlset>`;

	return new Response(xml, {
		headers: {
			'Content-Type': 'application/xml',
			'Cache-Control': 'max-age=3600'
		}
	});
};

interface SitemapEntry {
	path: string;
	priority: string;
	changefreq: string;
	lastmod: string;
}
```

### 1.10 API client and related files (`src/lib/api/`)

#### `src/lib/api/client.ts`

```typescript
// src/lib/api/client.ts
import { auth } from '$lib/stores/auth.svelte';
import { getPublicApiBase } from '$lib/api/publicApiBase';

const API_BASE = getPublicApiBase();

interface FetchOptions extends RequestInit {
	skipAuth?: boolean;
}

class ApiClient {
	private baseUrl: string;
	/** Ensures parallel 401s share one refresh (avoids races invalidating the refresh token). */
	private refreshInFlight: Promise<boolean> | null = null;

	constructor(baseUrl: string) {
		this.baseUrl = baseUrl;
	}

	private async request<T>(endpoint: string, options: FetchOptions = {}): Promise<T> {
		const { skipAuth, ...fetchOptions } = options;
		const headers = new Headers(fetchOptions.headers);

		if (
			!headers.has('Content-Type') &&
			fetchOptions.body &&
			!(fetchOptions.body instanceof FormData)
		) {
			headers.set('Content-Type', 'application/json');
		}

		if (!skipAuth && auth.accessToken) {
			headers.set('Authorization', `Bearer ${auth.accessToken}`);
		}

		const response = await fetch(`${this.baseUrl}${endpoint}`, {
			...fetchOptions,
			headers
		});

		if (response.status === 401 && !skipAuth && auth.refreshToken) {
			const refreshed = await this.refreshTokens();
			if (refreshed) {
				headers.set('Authorization', `Bearer ${auth.accessToken}`);
				const retry = await fetch(`${this.baseUrl}${endpoint}`, {
					...fetchOptions,
					headers
				});
				if (!retry.ok) {
					const err = await retry.json().catch(() => ({ error: 'Request failed' }));
					throw new ApiError(retry.status, err.error || 'Request failed');
				}
				return retry.json();
			} else {
				auth.logout();
				throw new ApiError(401, 'Session expired');
			}
		}

		if (!response.ok) {
			const err = await response.json().catch(() => ({ error: 'Request failed' }));
			throw new ApiError(response.status, err.error || 'Request failed');
		}

		return response.json();
	}

	private async refreshTokens(): Promise<boolean> {
		if (this.refreshInFlight) {
			return this.refreshInFlight;
		}
		this.refreshInFlight = this.performRefresh();
		try {
			return await this.refreshInFlight;
		} finally {
			this.refreshInFlight = null;
		}
	}

	private async performRefresh(): Promise<boolean> {
		try {
			const res = await fetch(`${this.baseUrl}/api/auth/refresh`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ refresh_token: auth.refreshToken })
			});

			if (!res.ok) return false;

			const data: { access_token: string; refresh_token: string } = await res.json();
			auth.setTokens(data.access_token, data.refresh_token);
			return true;
		} catch {
			return false;
		}
	}

	async get<T>(endpoint: string, options?: FetchOptions): Promise<T> {
		return this.request<T>(endpoint, { ...options, method: 'GET' });
	}

	async post<T>(endpoint: string, body?: unknown, options?: FetchOptions): Promise<T> {
		return this.request<T>(endpoint, {
			...options,
			method: 'POST',
			body: body ? JSON.stringify(body) : undefined
		});
	}

	async put<T>(endpoint: string, body?: unknown, options?: FetchOptions): Promise<T> {
		return this.request<T>(endpoint, {
			...options,
			method: 'PUT',
			body: body ? JSON.stringify(body) : undefined
		});
	}

	async del<T>(endpoint: string, options?: FetchOptions): Promise<T> {
		return this.request<T>(endpoint, { ...options, method: 'DELETE' });
	}

	async delete<T>(endpoint: string, options?: FetchOptions): Promise<T> {
		return this.del<T>(endpoint, options);
	}

	async upload<T>(endpoint: string, formData: FormData, options?: FetchOptions): Promise<T> {
		return this.request<T>(endpoint, {
			...options,
			method: 'POST',
			body: formData
		});
	}
}

export class ApiError extends Error {
	status: number;

	constructor(status: number, message: string) {
		super(message);
		this.status = status;
	}
}

export const api = new ApiClient(API_BASE);
```

#### `src/lib/api/publicApiBase.ts`

```typescript
// src/lib/api/publicApiBase.ts
import { browser } from '$app/environment';
import { resolvePublicApiBase } from '$lib/api/resolvePublicApiBase';

/**
 * Base URL for the Rust HTTP API (no trailing slash).
 *
 * - **Dev (browser):** `""` — Vite proxies `/api` → `vite.config.ts` target.
 * - **Dev (SSR):** `http://127.0.0.1:3001` — Node cannot use the browser proxy.
 * - **Production browser:** same-origin `/api` (Vercel rewrites to the Rust API). No CORS for page navigations.
 * - **Production SSR / build:** set `VITE_API_URL` so server-side loaders can reach the API host directly.
 */
export function getPublicApiBase(): string {
	return resolvePublicApiBase({
		viteApiUrl: import.meta.env.VITE_API_URL,
		dev: import.meta.env.DEV,
		browser
	});
}
```

#### `src/lib/api/resolvePublicApiBase.test.ts`

```typescript
// src/lib/api/resolvePublicApiBase.test.ts
import { describe, it, expect } from 'vitest';
import { resolvePublicApiBase } from './resolvePublicApiBase';

describe('resolvePublicApiBase', () => {
	it('server-side: trims VITE value and strips trailing slash', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: ' https://api.example.com/ ',
				dev: false,
				browser: false
			})
		).toBe('https://api.example.com');
	});

	it('dev + browser: empty string for Vite proxy', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: undefined,
				dev: true,
				browser: true
			})
		).toBe('');
	});

	it('dev + server: loopback for direct Rust access', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: undefined,
				dev: true,
				browser: false
			})
		).toBe('http://127.0.0.1:3001');
	});

	it('production + missing env: never localhost (Vercel split deploy)', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: undefined,
				dev: false,
				browser: true
			})
		).toBe('');
		expect(
			resolvePublicApiBase({
				viteApiUrl: undefined,
				dev: false,
				browser: false
			})
		).toBe('');
	});

	it('production + env set in browser: still uses same-origin path', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: 'https://swings-api.onrender.com',
				dev: false,
				browser: true
			})
		).toBe('');
	});

	it('production + env set on server: uses API host', () => {
		expect(
			resolvePublicApiBase({
				viteApiUrl: 'https://swings-api.onrender.com',
				dev: false,
				browser: false
			})
		).toBe('https://swings-api.onrender.com');
	});
});
```

#### `src/lib/api/resolvePublicApiBase.ts`

```typescript
// src/lib/api/resolvePublicApiBase.ts
/**
 * Pure resolver for tests and build-time behavior (no Svelte imports).
 *
 * Architecture (repo evidence):
 * - Frontend: `@sveltejs/adapter-vercel` (svelte.config.js)
 * - API: separate host (e.g. Render `backend/render.yaml`, `FRONTEND_URL` → Vercel)
 *
 * Production browser bundles must not fall back to `localhost` — that breaks Vercel users.
 * Set `VITE_API_URL` to the public API origin in the Vercel project (Production + Preview as needed).
 */
export function resolvePublicApiBase(params: {
	viteApiUrl: string | undefined;
	dev: boolean;
	browser: boolean;
}): string {
	// In production browsers, force same-origin requests so platform rewrites/proxies
	// can handle cross-origin API routing without CORS fragility.
	if (params.browser && !params.dev) {
		return '';
	}

	const raw = params.viteApiUrl;
	if (raw != null && String(raw).trim() !== '') {
		return String(raw).trim().replace(/\/$/, '');
	}
	if (params.dev) {
		return params.browser ? '' : 'http://127.0.0.1:3001';
	}
	// Production / preview build without VITE_API_URL: same-origin `/api/...` (only valid if you add edge rewrites).
	return '';
}
```

#### `src/lib/api/types.ts`

```typescript
// src/lib/api/types.ts
export interface AuthResponse {
	user: UserResponse;
	access_token: string;
	refresh_token: string;
}

export interface UserResponse {
	id: string;
	email: string;
	name: string;
	role: 'member' | 'admin';
	avatar_url: string | null;
	bio: string | null;
	position: string | null;
	website_url: string | null;
	twitter_url: string | null;
	linkedin_url: string | null;
	youtube_url: string | null;
	instagram_url: string | null;
	created_at: string;
}

export interface Subscription {
	id: string;
	user_id: string;
	stripe_customer_id: string;
	stripe_subscription_id: string;
	plan: 'monthly' | 'annual';
	status: 'active' | 'canceled' | 'past_due' | 'trialing' | 'unpaid';
	current_period_start: string;
	current_period_end: string;
	created_at: string;
	updated_at: string;
}

export interface SubscriptionStatusResponse {
	subscription: Subscription | null;
	is_active: boolean;
}

/** @deprecated use SubscriptionStatusResponse */
export type SubscriptionResponse = SubscriptionStatusResponse;

export interface BillingPortalResponse {
	url: string;
}

export interface Watchlist {
	id: string;
	title: string;
	week_of: string;
	video_url: string | null;
	notes: string | null;
	published: boolean;
	published_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface WatchlistAlert {
	id: string;
	watchlist_id: string;
	ticker: string;
	direction: 'bullish' | 'bearish';
	entry_zone: string;
	invalidation: string;
	profit_zones: string[];
	notes: string | null;
	chart_url: string | null;
	created_at: string;
}

export interface WatchlistWithAlerts extends Watchlist {
	alerts: WatchlistAlert[];
}

export interface CourseEnrollment {
	id: string;
	user_id: string;
	course_id: string;
	progress: number;
	enrolled_at: string;
	completed_at: string | null;
}

export interface AdminStats {
	total_members: number;
	active_subscriptions: number;
	monthly_subscriptions: number;
	annual_subscriptions: number;
	total_watchlists: number;
	total_enrollments: number;
	recent_members: UserResponse[];
}

export interface AnalyticsTimeBucket {
	date: string;
	page_views: number;
	unique_sessions: number;
	impressions: number;
}

export interface AnalyticsTopPage {
	path: string;
	views: number;
}

export interface AnalyticsCtrPoint {
	date: string;
	cta_id: string;
	impressions: number;
	clicks: number;
	ctr: number;
}

export interface AnalyticsSummary {
	from: string;
	to: string;
	total_page_views: number;
	total_sessions: number;
	total_impressions: number;
	time_series: AnalyticsTimeBucket[];
	top_pages: AnalyticsTopPage[];
	ctr_series: AnalyticsCtrPoint[];
}

export interface PaginatedResponse<T> {
	data: T[];
	total: number;
	page: number;
	per_page: number;
	total_pages: number;
}

// ── Blog ───────────────────────────────────────────────────────────────

export interface PostMeta {
	id: string;
	post_id: string;
	meta_key: string;
	meta_value: string;
	created_at: string;
	updated_at: string;
}

export type PostStatus =
	| 'draft'
	| 'pending_review'
	| 'published'
	| 'private'
	| 'scheduled'
	| 'trash';

export interface BlogPostResponse {
	id: string;
	author_id: string;
	author_name: string;
	author_avatar: string | null;
	author_position: string | null;
	author_bio: string | null;
	author_website: string | null;
	author_twitter: string | null;
	author_linkedin: string | null;
	author_youtube: string | null;
	title: string;
	slug: string;
	content: string;
	content_json: Record<string, unknown> | null;
	excerpt: string | null;
	featured_image_url: string | null;
	status: PostStatus;
	/** Status before the post was moved to trash (used when restoring). */
	pre_trash_status?: PostStatus | null;
	trashed_at?: string | null;
	visibility: string;
	is_password_protected: boolean;
	format: string;
	is_sticky: boolean;
	allow_comments: boolean;
	meta_title: string | null;
	meta_description: string | null;
	canonical_url: string | null;
	og_image_url: string | null;
	reading_time_minutes: number;
	word_count: number;
	categories: BlogCategory[];
	tags: BlogTag[];
	meta: PostMeta[];
	scheduled_at: string | null;
	published_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface BlogPostListItem {
	id: string;
	author_id: string;
	author_name: string;
	title: string;
	slug: string;
	excerpt: string | null;
	featured_image_url: string | null;
	status: PostStatus;
	is_sticky: boolean;
	reading_time_minutes: number;
	word_count: number;
	published_at: string | null;
	created_at: string;
	updated_at: string;
	categories: BlogCategory[];
	tags: BlogTag[];
}

export interface CreatePostPayload {
	title: string;
	slug?: string;
	content?: string;
	content_json?: Record<string, unknown>;
	excerpt?: string;
	featured_image_id?: string;
	status?: PostStatus;
	visibility?: string;
	is_sticky?: boolean;
	allow_comments?: boolean;
	meta_title?: string;
	meta_description?: string;
	canonical_url?: string;
	og_image_url?: string;
	category_ids?: string[];
	tag_ids?: string[];
	scheduled_at?: string;
	post_password?: string;
	author_id?: string;
	format?: string;
}

export interface UpdatePostPayload {
	title?: string;
	slug?: string;
	content?: string;
	content_json?: Record<string, unknown>;
	excerpt?: string;
	featured_image_id?: string;
	status?: PostStatus;
	visibility?: string;
	is_sticky?: boolean;
	allow_comments?: boolean;
	meta_title?: string;
	meta_description?: string;
	canonical_url?: string;
	og_image_url?: string;
	category_ids?: string[];
	tag_ids?: string[];
	scheduled_at?: string;
	post_password?: string;
	author_id?: string;
	format?: string;
}

export interface AutosavePayload {
	title?: string;
	content?: string;
	content_json?: Record<string, unknown>;
}

export interface BlogCategory {
	id: string;
	name: string;
	slug: string;
	description: string | null;
	parent_id: string | null;
	sort_order: number;
	created_at: string;
}

export interface BlogTag {
	id: string;
	name: string;
	slug: string;
	created_at: string;
}

export interface BlogRevision {
	id: string;
	post_id: string;
	author_id: string;
	author_name: string;
	title: string;
	revision_number: number;
	created_at: string;
}

export interface MediaItem {
	id: string;
	uploader_id: string;
	filename: string;
	original_filename: string;
	title: string | null;
	mime_type: string;
	file_size: number;
	width: number | null;
	height: number | null;
	alt_text: string | null;
	caption: string | null;
	storage_path: string;
	url: string;
	focal_x: number;
	focal_y: number;
	created_at: string;
}

export interface PostListFilters {
	page?: number;
	per_page?: number;
	status?: PostStatus;
	author_id?: string;
	category_slug?: string;
	tag_slug?: string;
	search?: string;
}

// ── Courses ───────────────────────────────────────────────────────────

export interface Course {
	id: string;
	title: string;
	slug: string;
	description: string;
	short_description: string | null;
	thumbnail_url: string | null;
	trailer_video_url: string | null;
	difficulty: 'beginner' | 'intermediate' | 'advanced';
	instructor_id: string;
	price_cents: number;
	currency: string;
	is_free: boolean;
	is_included_in_subscription: boolean;
	sort_order: number;
	published: boolean;
	published_at: string | null;
	estimated_duration_minutes: number;
	created_at: string;
	updated_at: string;
}

export interface CourseModule {
	id: string;
	course_id: string;
	title: string;
	description: string | null;
	sort_order: number;
	created_at: string;
	updated_at: string;
}

export interface CourseLesson {
	id: string;
	module_id: string;
	title: string;
	slug: string;
	description: string | null;
	content: string;
	content_json: Record<string, unknown> | null;
	video_url: string | null;
	video_duration_seconds: number | null;
	sort_order: number;
	is_preview: boolean;
	created_at: string;
	updated_at: string;
}

export interface LessonProgress {
	id: string;
	user_id: string;
	lesson_id: string;
	completed: boolean;
	progress_seconds: number;
	completed_at: string | null;
	last_accessed_at: string;
}

export interface CourseWithModules extends Course {
	modules: ModuleWithLessons[];
	total_lessons: number;
	total_duration_seconds: number;
}

export interface ModuleWithLessons extends CourseModule {
	lessons: CourseLesson[];
}

export interface CourseListItem {
	id: string;
	title: string;
	slug: string;
	short_description: string | null;
	thumbnail_url: string | null;
	difficulty: string;
	instructor_name: string;
	price_cents: number;
	is_free: boolean;
	is_included_in_subscription: boolean;
	published: boolean;
	estimated_duration_minutes: number;
	total_lessons: number;
	created_at: string;
}

export interface CreateCoursePayload {
	title: string;
	slug?: string;
	description?: string;
	short_description?: string;
	thumbnail_url?: string;
	trailer_video_url?: string;
	difficulty?: string;
	price_cents?: number;
	currency?: string;
	is_free?: boolean;
	is_included_in_subscription?: boolean;
	sort_order?: number;
	published?: boolean;
	estimated_duration_minutes?: number;
}

export interface UpdateCoursePayload {
	title?: string;
	slug?: string;
	description?: string;
	short_description?: string;
	thumbnail_url?: string;
	trailer_video_url?: string;
	difficulty?: string;
	price_cents?: number;
	currency?: string;
	is_free?: boolean;
	is_included_in_subscription?: boolean;
	sort_order?: number;
	published?: boolean;
	estimated_duration_minutes?: number;
}

export interface CreateModulePayload {
	title: string;
	description?: string;
	sort_order?: number;
}

export interface CreateLessonPayload {
	title: string;
	slug?: string;
	description?: string;
	content?: string;
	content_json?: Record<string, unknown>;
	video_url?: string;
	video_duration_seconds?: number;
	sort_order?: number;
	is_preview?: boolean;
}

// ── Pricing Plans ─────────────────────────────────────────────────────

export interface PricingPlan {
	id: string;
	name: string;
	slug: string;
	description: string | null;
	stripe_price_id: string | null;
	stripe_product_id: string | null;
	amount_cents: number;
	currency: string;
	interval: 'month' | 'year' | 'one_time';
	interval_count: number;
	trial_days: number;
	features: string[];
	highlight_text: string | null;
	is_popular: boolean;
	is_active: boolean;
	sort_order: number;
	created_at: string;
	updated_at: string;
}

export interface CreatePricingPlanPayload {
	name: string;
	slug?: string;
	description?: string;
	stripe_price_id?: string;
	stripe_product_id?: string;
	amount_cents: number;
	currency?: string;
	interval?: string;
	interval_count?: number;
	trial_days?: number;
	features?: string[];
	highlight_text?: string;
	is_popular?: boolean;
	is_active?: boolean;
	sort_order?: number;
}

export interface UpdatePricingPlanPayload {
	name?: string;
	slug?: string;
	description?: string;
	stripe_price_id?: string;
	stripe_product_id?: string;
	amount_cents?: number;
	currency?: string;
	interval?: string;
	interval_count?: number;
	trial_days?: number;
	features?: string[];
	highlight_text?: string;
	is_popular?: boolean;
	is_active?: boolean;
	sort_order?: number;
}

export interface PricingChangeLog {
	id: string;
	plan_id: string;
	field_changed: string;
	old_value: string | null;
	new_value: string | null;
	changed_by: string;
	changed_at: string;
}

export interface PricingPlanPriceLogEntry {
	id: string;
	plan_name: string;
	old_amount_cents: number;
	new_amount_cents: number;
	changed_at: string;
	changed_by: string;
}

// ── Coupons ───────────────────────────────────────────────────────────

export type DiscountType = 'percentage' | 'fixed_amount' | 'free_trial';

export interface Coupon {
	id: string;
	code: string;
	description: string | null;
	discount_type: DiscountType;
	discount_value: number;
	min_purchase_cents: number | null;
	max_discount_cents: number | null;
	applies_to: 'all' | 'monthly' | 'annual' | 'course' | 'specific_plans';
	applicable_plan_ids: string[];
	applicable_course_ids: string[];
	usage_limit: number | null;
	usage_count: number;
	per_user_limit: number;
	starts_at: string | null;
	expires_at: string | null;
	is_active: boolean;
	stackable: boolean;
	first_purchase_only: boolean;
	stripe_coupon_id: string | null;
	stripe_promotion_code_id: string | null;
	created_by: string;
	created_at: string;
	updated_at: string;
}

export interface CreateCouponPayload {
	code?: string;
	description?: string;
	discount_type: DiscountType;
	discount_value: number;
	min_purchase_cents?: number;
	max_discount_cents?: number;
	applies_to?: string;
	applicable_plan_ids?: string[];
	applicable_course_ids?: string[];
	usage_limit?: number;
	per_user_limit?: number;
	starts_at?: string;
	expires_at?: string;
	is_active?: boolean;
	stackable?: boolean;
	first_purchase_only?: boolean;
}

export interface UpdateCouponPayload {
	description?: string;
	discount_type?: DiscountType;
	discount_value?: number;
	min_purchase_cents?: number;
	max_discount_cents?: number;
	applies_to?: string;
	applicable_plan_ids?: string[];
	applicable_course_ids?: string[];
	usage_limit?: number;
	per_user_limit?: number;
	starts_at?: string;
	expires_at?: string;
	is_active?: boolean;
	stackable?: boolean;
	first_purchase_only?: boolean;
}

export interface CouponValidationResponse {
	valid: boolean;
	coupon: Coupon | null;
	discount_amount_cents: number | null;
	message: string;
}

export interface CouponUsage {
	id: string;
	coupon_id: string;
	user_id: string;
	subscription_id: string | null;
	discount_applied_cents: number;
	used_at: string;
}

export interface BulkCouponPayload {
	count: number;
	prefix?: string;
	discount_type: DiscountType;
	discount_value: number;
	usage_limit?: number;
	expires_at?: string;
}

// ── Popups ────────────────────────────────────────────────────────────

export type PopupType = 'modal' | 'slide_in' | 'banner' | 'fullscreen' | 'floating_bar' | 'inline';
export type PopupTrigger =
	| 'on_load'
	| 'exit_intent'
	| 'scroll_percentage'
	| 'time_delay'
	| 'click'
	| 'manual'
	| 'inactivity';
export type PopupFrequency = 'every_time' | 'once_per_session' | 'once_ever' | 'custom';

export interface PopupElement {
	id: string;
	type:
		| 'heading'
		| 'text'
		| 'image'
		| 'input'
		| 'email'
		| 'textarea'
		| 'select'
		| 'checkbox'
		| 'radio'
		| 'button'
		| 'divider'
		| 'spacer';
	props: Record<string, unknown>;
	style?: Record<string, string>;
}

export interface PopupStyle {
	background: string;
	textColor: string;
	accentColor: string;
	borderRadius: string;
	maxWidth: string;
	animation: 'fade' | 'slide_up' | 'slide_down' | 'slide_left' | 'slide_right' | 'scale' | 'none';
	backdrop: boolean;
	backdropColor: string;
	padding?: string;
	shadow?: string;
}

export interface PopupTargetingRules {
	pages: string[];
	devices: ('desktop' | 'mobile' | 'tablet')[];
	userStatus: ('all' | 'logged_in' | 'logged_out' | 'member' | 'non_member')[];
}

export interface Popup {
	id: string;
	name: string;
	popup_type: PopupType;
	trigger_type: PopupTrigger;
	trigger_config: Record<string, unknown>;
	content_json: { elements: PopupElement[] };
	style_json: PopupStyle;
	targeting_rules: PopupTargetingRules;
	display_frequency: PopupFrequency;
	frequency_config: Record<string, unknown>;
	success_message: string | null;
	redirect_url: string | null;
	is_active: boolean;
	starts_at: string | null;
	expires_at: string | null;
	priority: number;
	created_by: string;
	created_at: string;
	updated_at: string;
}

export interface CreatePopupPayload {
	name: string;
	popup_type?: PopupType;
	trigger_type?: PopupTrigger;
	trigger_config?: Record<string, unknown>;
	content_json?: { elements: PopupElement[] };
	style_json?: Partial<PopupStyle>;
	targeting_rules?: Partial<PopupTargetingRules>;
	display_frequency?: PopupFrequency;
	frequency_config?: Record<string, unknown>;
	success_message?: string;
	redirect_url?: string;
	is_active?: boolean;
	starts_at?: string;
	expires_at?: string;
	priority?: number;
}

export type UpdatePopupPayload = Partial<CreatePopupPayload>;

export interface PopupSubmission {
	id: string;
	popup_id: string;
	user_id: string | null;
	session_id: string | null;
	form_data: Record<string, unknown>;
	ip_address: string | null;
	user_agent: string | null;
	page_url: string | null;
	submitted_at: string;
}

export interface PopupAnalytics {
	popup_id: string;
	popup_name: string;
	total_impressions: number;
	total_closes: number;
	total_submissions: number;
	conversion_rate: number;
}

// ── Revenue Analytics ─────────────────────────────────────────────────

export interface SalesEvent {
	id: string;
	user_id: string;
	event_type:
		| 'new_subscription'
		| 'renewal'
		| 'upgrade'
		| 'downgrade'
		| 'cancellation'
		| 'refund'
		| 'course_purchase';
	amount_cents: number;
	currency: string;
	plan_id: string | null;
	coupon_id: string | null;
	stripe_payment_intent_id: string | null;
	stripe_invoice_id: string | null;
	metadata: Record<string, unknown>;
	created_at: string;
}

export interface MonthlyRevenueSummary {
	year: number;
	month: number;
	revenue_cents: number;
	new_subscribers: number;
	churned: number;
}

export interface PlanRevenueSummary {
	plan_name: string;
	subscriber_count: number;
	revenue_cents: number;
}

export interface RevenueAnalytics {
	total_revenue_cents: number;
	mrr_cents: number;
	arr_cents: number;
	total_subscribers: number;
	churn_rate: number;
	avg_revenue_per_user_cents: number;
	revenue_by_month: MonthlyRevenueSummary[];
	revenue_by_plan: PlanRevenueSummary[];
	recent_sales: SalesEvent[];
}
```

### 1.11 Auth-related frontend files

#### `src/lib/stores/auth.svelte.ts`

```typescript
// src/lib/stores/auth.svelte.ts
// Svelte 5 reactive auth state class
import { browser } from '$app/environment';

export interface AuthUser {
	id: string;
	email: string;
	name: string;
	role: 'member' | 'admin';
	avatar_url: string | null;
	created_at: string;
}

const TOKEN_KEY = 'swings_access_token';
const REFRESH_KEY = 'swings_refresh_token';
const USER_KEY = 'swings_user';

class AuthState {
	user = $state<AuthUser | null>(null);
	accessToken = $state<string | null>(null);
	refreshToken = $state<string | null>(null);
	loading = $state(true);

	isAuthenticated = $derived(!!this.user && !!this.accessToken);
	isAdmin = $derived(this.user?.role === 'admin');
	isMember = $derived(this.user?.role === 'member');

	constructor() {
		if (browser) {
			this.accessToken = localStorage.getItem(TOKEN_KEY);
			this.refreshToken = localStorage.getItem(REFRESH_KEY);
			const stored = localStorage.getItem(USER_KEY);
			if (stored) {
				try {
					this.user = JSON.parse(stored);
				} catch {
					this.user = null;
				}
			}
		}
		// Resolve loading on both server and client so SSR-rendered UI doesn't get
		// stuck in a "loading" state forever.
		this.loading = false;
	}

	setAuth = (user: AuthUser, accessToken: string, refreshToken: string) => {
		this.user = user;
		this.accessToken = accessToken;
		this.refreshToken = refreshToken;

		if (browser) {
			localStorage.setItem(TOKEN_KEY, accessToken);
			localStorage.setItem(REFRESH_KEY, refreshToken);
			localStorage.setItem(USER_KEY, JSON.stringify(user));
		}
	};

	setTokens = (accessToken: string, refreshToken: string) => {
		this.accessToken = accessToken;
		this.refreshToken = refreshToken;

		if (browser) {
			localStorage.setItem(TOKEN_KEY, accessToken);
			localStorage.setItem(REFRESH_KEY, refreshToken);
		}
	};

	setUser = (user: AuthUser) => {
		this.user = user;
		if (browser) {
			localStorage.setItem(USER_KEY, JSON.stringify(user));
		}
	};

	logout = () => {
		this.user = null;
		this.accessToken = null;
		this.refreshToken = null;

		if (browser) {
			localStorage.removeItem(TOKEN_KEY);
			localStorage.removeItem(REFRESH_KEY);
			localStorage.removeItem(USER_KEY);
		}
	};
}

export const auth = new AuthState();
```

### 1.12 `src/hooks.server.ts` and `src/hooks.client.ts`

#### `src/hooks.server.ts`

```typescript
// src/hooks.server.ts
import type { Handle, HandleServerError } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
	const response = await resolve(event, {
		// `js` is preloaded as modulepreload by SvelteKit by default; we only need
		// to opt fonts and CSS in explicitly.
		preload: ({ type }) => type === 'font' || type === 'css'
	});

	// Security headers
	response.headers.set('X-Frame-Options', 'SAMEORIGIN');
	response.headers.set('X-Content-Type-Options', 'nosniff');
	response.headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
	response.headers.set(
		'Permissions-Policy',
		'camera=(), microphone=(), geolocation=(), interest-cohort=()'
	);

	// Cache immutable assets (Vite hashed files)
	const { pathname } = event.url;
	if (pathname.startsWith('/_app/immutable/')) {
		response.headers.set('Cache-Control', 'public, max-age=31536000, immutable');
	}

	return response;
};

export const handleError: HandleServerError = ({ error, event, status, message }) => {
	const errorId = crypto.randomUUID();
	console.error(`[server-error ${errorId}]`, status, message, event.url.pathname, error);
	return {
		message: status >= 500 ? 'An unexpected error occurred. Please try again.' : message,
		id: errorId
	};
};
```

#### `src/hooks.client.ts`

```typescript
// src/hooks.client.ts
import { dev } from '$app/environment';
import type { HandleClientError } from '@sveltejs/kit';
import { applyServiceWorkerDevPolicy } from '$lib/client/service-worker-dev-policy';

/**
 * Client bootstrap (runs once per app load in the browser).
 * @see https://svelte.dev/docs/kit/hooks
 */
applyServiceWorkerDevPolicy({
	dev,
	optedIntoServiceWorkerInDev:
		import.meta.env.PUBLIC_SERVICE_WORKER_IN_DEV === '1' ||
		import.meta.env.PUBLIC_SERVICE_WORKER_IN_DEV === 'true'
});

export const handleError: HandleClientError = ({ error, event, status, message }) => {
	const errorId = crypto.randomUUID();
	console.error(`[client-error ${errorId}]`, status, message, event.url.pathname, error);
	return {
		message: status >= 500 ? 'An unexpected error occurred. Please try again.' : message,
		id: errorId
	};
};
```

### 1.13 Shared / API TypeScript definitions

#### `src/lib/api/types.ts`

```typescript
// src/lib/api/types.ts
export interface AuthResponse {
	user: UserResponse;
	access_token: string;
	refresh_token: string;
}

export interface UserResponse {
	id: string;
	email: string;
	name: string;
	role: 'member' | 'admin';
	avatar_url: string | null;
	bio: string | null;
	position: string | null;
	website_url: string | null;
	twitter_url: string | null;
	linkedin_url: string | null;
	youtube_url: string | null;
	instagram_url: string | null;
	created_at: string;
}

export interface Subscription {
	id: string;
	user_id: string;
	stripe_customer_id: string;
	stripe_subscription_id: string;
	plan: 'monthly' | 'annual';
	status: 'active' | 'canceled' | 'past_due' | 'trialing' | 'unpaid';
	current_period_start: string;
	current_period_end: string;
	created_at: string;
	updated_at: string;
}

export interface SubscriptionStatusResponse {
	subscription: Subscription | null;
	is_active: boolean;
}

/** @deprecated use SubscriptionStatusResponse */
export type SubscriptionResponse = SubscriptionStatusResponse;

export interface BillingPortalResponse {
	url: string;
}

export interface Watchlist {
	id: string;
	title: string;
	week_of: string;
	video_url: string | null;
	notes: string | null;
	published: boolean;
	published_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface WatchlistAlert {
	id: string;
	watchlist_id: string;
	ticker: string;
	direction: 'bullish' | 'bearish';
	entry_zone: string;
	invalidation: string;
	profit_zones: string[];
	notes: string | null;
	chart_url: string | null;
	created_at: string;
}

export interface WatchlistWithAlerts extends Watchlist {
	alerts: WatchlistAlert[];
}

export interface CourseEnrollment {
	id: string;
	user_id: string;
	course_id: string;
	progress: number;
	enrolled_at: string;
	completed_at: string | null;
}

export interface AdminStats {
	total_members: number;
	active_subscriptions: number;
	monthly_subscriptions: number;
	annual_subscriptions: number;
	total_watchlists: number;
	total_enrollments: number;
	recent_members: UserResponse[];
}

export interface AnalyticsTimeBucket {
	date: string;
	page_views: number;
	unique_sessions: number;
	impressions: number;
}

export interface AnalyticsTopPage {
	path: string;
	views: number;
}

export interface AnalyticsCtrPoint {
	date: string;
	cta_id: string;
	impressions: number;
	clicks: number;
	ctr: number;
}

export interface AnalyticsSummary {
	from: string;
	to: string;
	total_page_views: number;
	total_sessions: number;
	total_impressions: number;
	time_series: AnalyticsTimeBucket[];
	top_pages: AnalyticsTopPage[];
	ctr_series: AnalyticsCtrPoint[];
}

export interface PaginatedResponse<T> {
	data: T[];
	total: number;
	page: number;
	per_page: number;
	total_pages: number;
}

// ── Blog ───────────────────────────────────────────────────────────────

export interface PostMeta {
	id: string;
	post_id: string;
	meta_key: string;
	meta_value: string;
	created_at: string;
	updated_at: string;
}

export type PostStatus =
	| 'draft'
	| 'pending_review'
	| 'published'
	| 'private'
	| 'scheduled'
	| 'trash';

export interface BlogPostResponse {
	id: string;
	author_id: string;
	author_name: string;
	author_avatar: string | null;
	author_position: string | null;
	author_bio: string | null;
	author_website: string | null;
	author_twitter: string | null;
	author_linkedin: string | null;
	author_youtube: string | null;
	title: string;
	slug: string;
	content: string;
	content_json: Record<string, unknown> | null;
	excerpt: string | null;
	featured_image_url: string | null;
	status: PostStatus;
	/** Status before the post was moved to trash (used when restoring). */
	pre_trash_status?: PostStatus | null;
	trashed_at?: string | null;
	visibility: string;
	is_password_protected: boolean;
	format: string;
	is_sticky: boolean;
	allow_comments: boolean;
	meta_title: string | null;
	meta_description: string | null;
	canonical_url: string | null;
	og_image_url: string | null;
	reading_time_minutes: number;
	word_count: number;
	categories: BlogCategory[];
	tags: BlogTag[];
	meta: PostMeta[];
	scheduled_at: string | null;
	published_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface BlogPostListItem {
	id: string;
	author_id: string;
	author_name: string;
	title: string;
	slug: string;
	excerpt: string | null;
	featured_image_url: string | null;
	status: PostStatus;
	is_sticky: boolean;
	reading_time_minutes: number;
	word_count: number;
	published_at: string | null;
	created_at: string;
	updated_at: string;
	categories: BlogCategory[];
	tags: BlogTag[];
}

export interface CreatePostPayload {
	title: string;
	slug?: string;
	content?: string;
	content_json?: Record<string, unknown>;
	excerpt?: string;
	featured_image_id?: string;
	status?: PostStatus;
	visibility?: string;
	is_sticky?: boolean;
	allow_comments?: boolean;
	meta_title?: string;
	meta_description?: string;
	canonical_url?: string;
	og_image_url?: string;
	category_ids?: string[];
	tag_ids?: string[];
	scheduled_at?: string;
	post_password?: string;
	author_id?: string;
	format?: string;
}

export interface UpdatePostPayload {
	title?: string;
	slug?: string;
	content?: string;
	content_json?: Record<string, unknown>;
	excerpt?: string;
	featured_image_id?: string;
	status?: PostStatus;
	visibility?: string;
	is_sticky?: boolean;
	allow_comments?: boolean;
	meta_title?: string;
	meta_description?: string;
	canonical_url?: string;
	og_image_url?: string;
	category_ids?: string[];
	tag_ids?: string[];
	scheduled_at?: string;
	post_password?: string;
	author_id?: string;
	format?: string;
}

export interface AutosavePayload {
	title?: string;
	content?: string;
	content_json?: Record<string, unknown>;
}

export interface BlogCategory {
	id: string;
	name: string;
	slug: string;
	description: string | null;
	parent_id: string | null;
	sort_order: number;
	created_at: string;
}

export interface BlogTag {
	id: string;
	name: string;
	slug: string;
	created_at: string;
}

export interface BlogRevision {
	id: string;
	post_id: string;
	author_id: string;
	author_name: string;
	title: string;
	revision_number: number;
	created_at: string;
}

export interface MediaItem {
	id: string;
	uploader_id: string;
	filename: string;
	original_filename: string;
	title: string | null;
	mime_type: string;
	file_size: number;
	width: number | null;
	height: number | null;
	alt_text: string | null;
	caption: string | null;
	storage_path: string;
	url: string;
	focal_x: number;
	focal_y: number;
	created_at: string;
}

export interface PostListFilters {
	page?: number;
	per_page?: number;
	status?: PostStatus;
	author_id?: string;
	category_slug?: string;
	tag_slug?: string;
	search?: string;
}

// ── Courses ───────────────────────────────────────────────────────────

export interface Course {
	id: string;
	title: string;
	slug: string;
	description: string;
	short_description: string | null;
	thumbnail_url: string | null;
	trailer_video_url: string | null;
	difficulty: 'beginner' | 'intermediate' | 'advanced';
	instructor_id: string;
	price_cents: number;
	currency: string;
	is_free: boolean;
	is_included_in_subscription: boolean;
	sort_order: number;
	published: boolean;
	published_at: string | null;
	estimated_duration_minutes: number;
	created_at: string;
	updated_at: string;
}

export interface CourseModule {
	id: string;
	course_id: string;
	title: string;
	description: string | null;
	sort_order: number;
	created_at: string;
	updated_at: string;
}

export interface CourseLesson {
	id: string;
	module_id: string;
	title: string;
	slug: string;
	description: string | null;
	content: string;
	content_json: Record<string, unknown> | null;
	video_url: string | null;
	video_duration_seconds: number | null;
	sort_order: number;
	is_preview: boolean;
	created_at: string;
	updated_at: string;
}

export interface LessonProgress {
	id: string;
	user_id: string;
	lesson_id: string;
	completed: boolean;
	progress_seconds: number;
	completed_at: string | null;
	last_accessed_at: string;
}

export interface CourseWithModules extends Course {
	modules: ModuleWithLessons[];
	total_lessons: number;
	total_duration_seconds: number;
}

export interface ModuleWithLessons extends CourseModule {
	lessons: CourseLesson[];
}

export interface CourseListItem {
	id: string;
	title: string;
	slug: string;
	short_description: string | null;
	thumbnail_url: string | null;
	difficulty: string;
	instructor_name: string;
	price_cents: number;
	is_free: boolean;
	is_included_in_subscription: boolean;
	published: boolean;
	estimated_duration_minutes: number;
	total_lessons: number;
	created_at: string;
}

export interface CreateCoursePayload {
	title: string;
	slug?: string;
	description?: string;
	short_description?: string;
	thumbnail_url?: string;
	trailer_video_url?: string;
	difficulty?: string;
	price_cents?: number;
	currency?: string;
	is_free?: boolean;
	is_included_in_subscription?: boolean;
	sort_order?: number;
	published?: boolean;
	estimated_duration_minutes?: number;
}

export interface UpdateCoursePayload {
	title?: string;
	slug?: string;
	description?: string;
	short_description?: string;
	thumbnail_url?: string;
	trailer_video_url?: string;
	difficulty?: string;
	price_cents?: number;
	currency?: string;
	is_free?: boolean;
	is_included_in_subscription?: boolean;
	sort_order?: number;
	published?: boolean;
	estimated_duration_minutes?: number;
}

export interface CreateModulePayload {
	title: string;
	description?: string;
	sort_order?: number;
}

export interface CreateLessonPayload {
	title: string;
	slug?: string;
	description?: string;
	content?: string;
	content_json?: Record<string, unknown>;
	video_url?: string;
	video_duration_seconds?: number;
	sort_order?: number;
	is_preview?: boolean;
}

// ── Pricing Plans ─────────────────────────────────────────────────────

export interface PricingPlan {
	id: string;
	name: string;
	slug: string;
	description: string | null;
	stripe_price_id: string | null;
	stripe_product_id: string | null;
	amount_cents: number;
	currency: string;
	interval: 'month' | 'year' | 'one_time';
	interval_count: number;
	trial_days: number;
	features: string[];
	highlight_text: string | null;
	is_popular: boolean;
	is_active: boolean;
	sort_order: number;
	created_at: string;
	updated_at: string;
}

export interface CreatePricingPlanPayload {
	name: string;
	slug?: string;
	description?: string;
	stripe_price_id?: string;
	stripe_product_id?: string;
	amount_cents: number;
	currency?: string;
	interval?: string;
	interval_count?: number;
	trial_days?: number;
	features?: string[];
	highlight_text?: string;
	is_popular?: boolean;
	is_active?: boolean;
	sort_order?: number;
}

export interface UpdatePricingPlanPayload {
	name?: string;
	slug?: string;
	description?: string;
	stripe_price_id?: string;
	stripe_product_id?: string;
	amount_cents?: number;
	currency?: string;
	interval?: string;
	interval_count?: number;
	trial_days?: number;
	features?: string[];
	highlight_text?: string;
	is_popular?: boolean;
	is_active?: boolean;
	sort_order?: number;
}

export interface PricingChangeLog {
	id: string;
	plan_id: string;
	field_changed: string;
	old_value: string | null;
	new_value: string | null;
	changed_by: string;
	changed_at: string;
}

export interface PricingPlanPriceLogEntry {
	id: string;
	plan_name: string;
	old_amount_cents: number;
	new_amount_cents: number;
	changed_at: string;
	changed_by: string;
}

// ── Coupons ───────────────────────────────────────────────────────────

export type DiscountType = 'percentage' | 'fixed_amount' | 'free_trial';

export interface Coupon {
	id: string;
	code: string;
	description: string | null;
	discount_type: DiscountType;
	discount_value: number;
	min_purchase_cents: number | null;
	max_discount_cents: number | null;
	applies_to: 'all' | 'monthly' | 'annual' | 'course' | 'specific_plans';
	applicable_plan_ids: string[];
	applicable_course_ids: string[];
	usage_limit: number | null;
	usage_count: number;
	per_user_limit: number;
	starts_at: string | null;
	expires_at: string | null;
	is_active: boolean;
	stackable: boolean;
	first_purchase_only: boolean;
	stripe_coupon_id: string | null;
	stripe_promotion_code_id: string | null;
	created_by: string;
	created_at: string;
	updated_at: string;
}

export interface CreateCouponPayload {
	code?: string;
	description?: string;
	discount_type: DiscountType;
	discount_value: number;
	min_purchase_cents?: number;
	max_discount_cents?: number;
	applies_to?: string;
	applicable_plan_ids?: string[];
	applicable_course_ids?: string[];
	usage_limit?: number;
	per_user_limit?: number;
	starts_at?: string;
	expires_at?: string;
	is_active?: boolean;
	stackable?: boolean;
	first_purchase_only?: boolean;
}

export interface UpdateCouponPayload {
	description?: string;
	discount_type?: DiscountType;
	discount_value?: number;
	min_purchase_cents?: number;
	max_discount_cents?: number;
	applies_to?: string;
	applicable_plan_ids?: string[];
	applicable_course_ids?: string[];
	usage_limit?: number;
	per_user_limit?: number;
	starts_at?: string;
	expires_at?: string;
	is_active?: boolean;
	stackable?: boolean;
	first_purchase_only?: boolean;
}

export interface CouponValidationResponse {
	valid: boolean;
	coupon: Coupon | null;
	discount_amount_cents: number | null;
	message: string;
}

export interface CouponUsage {
	id: string;
	coupon_id: string;
	user_id: string;
	subscription_id: string | null;
	discount_applied_cents: number;
	used_at: string;
}

export interface BulkCouponPayload {
	count: number;
	prefix?: string;
	discount_type: DiscountType;
	discount_value: number;
	usage_limit?: number;
	expires_at?: string;
}

// ── Popups ────────────────────────────────────────────────────────────

export type PopupType = 'modal' | 'slide_in' | 'banner' | 'fullscreen' | 'floating_bar' | 'inline';
export type PopupTrigger =
	| 'on_load'
	| 'exit_intent'
	| 'scroll_percentage'
	| 'time_delay'
	| 'click'
	| 'manual'
	| 'inactivity';
export type PopupFrequency = 'every_time' | 'once_per_session' | 'once_ever' | 'custom';

export interface PopupElement {
	id: string;
	type:
		| 'heading'
		| 'text'
		| 'image'
		| 'input'
		| 'email'
		| 'textarea'
		| 'select'
		| 'checkbox'
		| 'radio'
		| 'button'
		| 'divider'
		| 'spacer';
	props: Record<string, unknown>;
	style?: Record<string, string>;
}

export interface PopupStyle {
	background: string;
	textColor: string;
	accentColor: string;
	borderRadius: string;
	maxWidth: string;
	animation: 'fade' | 'slide_up' | 'slide_down' | 'slide_left' | 'slide_right' | 'scale' | 'none';
	backdrop: boolean;
	backdropColor: string;
	padding?: string;
	shadow?: string;
}

export interface PopupTargetingRules {
	pages: string[];
	devices: ('desktop' | 'mobile' | 'tablet')[];
	userStatus: ('all' | 'logged_in' | 'logged_out' | 'member' | 'non_member')[];
}

export interface Popup {
	id: string;
	name: string;
	popup_type: PopupType;
	trigger_type: PopupTrigger;
	trigger_config: Record<string, unknown>;
	content_json: { elements: PopupElement[] };
	style_json: PopupStyle;
	targeting_rules: PopupTargetingRules;
	display_frequency: PopupFrequency;
	frequency_config: Record<string, unknown>;
	success_message: string | null;
	redirect_url: string | null;
	is_active: boolean;
	starts_at: string | null;
	expires_at: string | null;
	priority: number;
	created_by: string;
	created_at: string;
	updated_at: string;
}

export interface CreatePopupPayload {
	name: string;
	popup_type?: PopupType;
	trigger_type?: PopupTrigger;
	trigger_config?: Record<string, unknown>;
	content_json?: { elements: PopupElement[] };
	style_json?: Partial<PopupStyle>;
	targeting_rules?: Partial<PopupTargetingRules>;
	display_frequency?: PopupFrequency;
	frequency_config?: Record<string, unknown>;
	success_message?: string;
	redirect_url?: string;
	is_active?: boolean;
	starts_at?: string;
	expires_at?: string;
	priority?: number;
}

export type UpdatePopupPayload = Partial<CreatePopupPayload>;

export interface PopupSubmission {
	id: string;
	popup_id: string;
	user_id: string | null;
	session_id: string | null;
	form_data: Record<string, unknown>;
	ip_address: string | null;
	user_agent: string | null;
	page_url: string | null;
	submitted_at: string;
}

export interface PopupAnalytics {
	popup_id: string;
	popup_name: string;
	total_impressions: number;
	total_closes: number;
	total_submissions: number;
	conversion_rate: number;
}

// ── Revenue Analytics ─────────────────────────────────────────────────

export interface SalesEvent {
	id: string;
	user_id: string;
	event_type:
		| 'new_subscription'
		| 'renewal'
		| 'upgrade'
		| 'downgrade'
		| 'cancellation'
		| 'refund'
		| 'course_purchase';
	amount_cents: number;
	currency: string;
	plan_id: string | null;
	coupon_id: string | null;
	stripe_payment_intent_id: string | null;
	stripe_invoice_id: string | null;
	metadata: Record<string, unknown>;
	created_at: string;
}

export interface MonthlyRevenueSummary {
	year: number;
	month: number;
	revenue_cents: number;
	new_subscribers: number;
	churned: number;
}

export interface PlanRevenueSummary {
	plan_name: string;
	subscriber_count: number;
	revenue_cents: number;
}

export interface RevenueAnalytics {
	total_revenue_cents: number;
	mrr_cents: number;
	arr_cents: number;
	total_subscribers: number;
	churn_rate: number;
	avg_revenue_per_user_cents: number;
	revenue_by_month: MonthlyRevenueSummary[];
	revenue_by_plan: PlanRevenueSummary[];
	recent_sales: SalesEvent[];
}
```

**Note:** No `src/lib/types/` or `src/lib/models/` directories exist in this repo.

## Section 2: Backend (Rust)

### 2.1 backend/Cargo.toml

```toml
// backend/Cargo.toml
[package]
name = "swings-api"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web framework
axum = { version = "0.8", features = ["macros", "multipart"] }
axum-extra = { version = "0.12", features = ["typed-header", "cookie"] }
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "fs"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono", "migrate", "json", "rust_decimal"] }

# Auth
jsonwebtoken = "10"
argon2 = "0.5"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
dotenvy = "0.15"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "2"
anyhow = "1"
sha2 = "0.10"
hmac = "0.12"
validator = { version = "0.20", features = ["derive"] }

# Stripe
stripe-rust = { package = "async-stripe", version = "0.39", features = ["runtime-tokio-hyper"] }

# Blog / Media
axum_typed_multipart = "0.16"
tempfile = "3"
sanitize-filename = "0.6"

# Email
lettre = { version = "0.11", default-features = false, features = ["tokio1-rustls-tls", "builder", "smtp-transport", "tokio1"] }
tera = "1"
rand = "0.8"

# Decimal
rust_decimal = { version = "1", features = ["serde-with-str"] }

# R2 (S3 API)
aws-sdk-s3 = { version = "1", default-features = false, features = ["rt-tokio", "rustls"] }
bytes = "1"

# Rate limiting
governor = "0.10"
tower_governor = "0.8"
```

### 2.2 Directory tree of `backend/src/`

```
backend/src/
├── handlers
│   ├── admin.rs
│   ├── analytics.rs
│   ├── auth.rs
│   ├── blog.rs
│   ├── coupons.rs
│   ├── courses.rs
│   ├── member.rs
│   ├── mod.rs
│   ├── popups.rs
│   ├── pricing.rs
│   └── webhooks.rs
├── middleware
│   └── rate_limit.rs
├── services
│   ├── mod.rs
│   └── storage.rs
├── config.rs
├── db.rs
├── email.rs
├── error.rs
├── extractors.rs
├── main.rs
├── middleware.rs
├── models.rs
└── stripe_api.rs
```

### 2.3 backend/src/main.rs

```rust
// backend/src/main.rs
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::http::HeaderValue;
use axum::http::Method;
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;
mod email;
mod error;
mod extractors;
mod handlers;
mod middleware;
mod models;
mod services;
mod stripe_api;

use config::Config;

/// `dotenvy::dotenv()` only reads `./.env` from the process CWD. When invoked as
/// `cargo run --manifest-path backend/Cargo.toml` from the repo root, CWD is the root and env
/// vars in `backend/.env` are missed — try that path as a fallback.
fn load_dotenv() {
    dotenvy::dotenv().ok();
    if std::env::var("DATABASE_URL").is_err() {
        let _ = dotenvy::from_filename("backend/.env");
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Arc<Config>,
    pub email_service: Option<Arc<email::EmailService>>,
    pub media_backend: services::MediaBackend,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "swings_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    load_dotenv();

    let config = Config::from_env();
    config.assert_production_ready();
    let port = config.port;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(0)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let admin_email = std::env::var("ADMIN_EMAIL").ok();
    let admin_password = std::env::var("ADMIN_PASSWORD").ok();
    let admin_name = std::env::var("ADMIN_NAME").unwrap_or_else(|_| "Admin".to_string());
    match (admin_email, admin_password) {
        (Some(email), Some(password)) => {
            db::seed_admin(&pool, &email, &password, &admin_name)
                .await
                .expect("Failed to seed admin user");
        }
        _ if config.is_production() => {
            panic!("ADMIN_EMAIL and ADMIN_PASSWORD must be set in production");
        }
        _ => {
            tracing::warn!(
                "ADMIN_EMAIL/ADMIN_PASSWORD not set; skipping admin seed in non-production mode"
            );
        }
    }

    // Ensure uploads directory exists
    let upload_dir = config.upload_dir.clone();

    let email_service = if config.smtp_user.is_empty() {
        tracing::warn!("SMTP_USER not configured — email sending is disabled");
        None
    } else {
        match email::EmailService::new(&config) {
            Ok(svc) => {
                tracing::info!("Email service initialized (SMTP: {})", config.smtp_host);
                Some(Arc::new(svc))
            }
            Err(e) => {
                tracing::error!("Failed to initialize email service: {e}");
                None
            }
        }
    };

    let media_backend = services::MediaBackend::resolve(config.upload_dir.clone());

    let state = AppState {
        db: pool,
        config: Arc::new(config),
        email_service,
        media_backend,
    };

    let allowed_origins = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();
    if allowed_origins.is_empty() {
        panic!("CORS_ALLOWED_ORIGINS (or FRONTEND_URL) must contain at least one valid origin");
    }

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        // Avoid preflight failures when browsers/extensions send non-standard request headers.
        .allow_headers(Any);
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .expect("Failed to create uploads directory");

    let mount_local_uploads =
        !(state.config.is_production() && state.media_backend.is_r2());

    let mut app = Router::new()
        // Auth & analytics
        .nest("/api/auth", handlers::auth::router())
        .nest("/api/analytics", handlers::analytics::router())
        // Admin routes
        .nest("/api/admin", handlers::admin::router())
        .nest("/api/admin/blog", handlers::blog::admin_router())
        .nest("/api/admin/courses", handlers::courses::admin_router())
        .nest("/api/admin/pricing", handlers::pricing::admin_router())
        .nest("/api/admin/coupons", handlers::coupons::admin_router())
        .nest("/api/admin/popups", handlers::popups::admin_router())
        // Public routes
        .nest("/api/blog", handlers::blog::public_router())
        .nest("/api/courses", handlers::courses::public_router())
        .nest("/api/pricing", handlers::pricing::public_router())
        .nest("/api/coupons", handlers::coupons::public_router())
        .nest("/api/popups", handlers::popups::public_router())
        // Member routes
        .nest("/api/member", handlers::member::router())
        .nest("/api/member", handlers::courses::member_router())
        // Webhooks
        .nest("/api/webhooks", handlers::webhooks::router());

    if mount_local_uploads {
        app = app.nest_service("/uploads", ServeDir::new(&upload_dir));
    }

    let app = app
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();

    tracing::info!("Swings API listening on port {port}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
```

### 2.4 Application state (`AppState` in `backend/src/main.rs`)

The `AppState` struct and server setup live in `backend/src/main.rs`. Full file contents (verbatim duplicate of §2.3):

```rust
// backend/src/main.rs
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::http::HeaderValue;
use axum::http::Method;
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;
mod email;
mod error;
mod extractors;
mod handlers;
mod middleware;
mod models;
mod services;
mod stripe_api;

use config::Config;

/// `dotenvy::dotenv()` only reads `./.env` from the process CWD. When invoked as
/// `cargo run --manifest-path backend/Cargo.toml` from the repo root, CWD is the root and env
/// vars in `backend/.env` are missed — try that path as a fallback.
fn load_dotenv() {
    dotenvy::dotenv().ok();
    if std::env::var("DATABASE_URL").is_err() {
        let _ = dotenvy::from_filename("backend/.env");
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Arc<Config>,
    pub email_service: Option<Arc<email::EmailService>>,
    pub media_backend: services::MediaBackend,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "swings_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    load_dotenv();

    let config = Config::from_env();
    config.assert_production_ready();
    let port = config.port;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(0)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let admin_email = std::env::var("ADMIN_EMAIL").ok();
    let admin_password = std::env::var("ADMIN_PASSWORD").ok();
    let admin_name = std::env::var("ADMIN_NAME").unwrap_or_else(|_| "Admin".to_string());
    match (admin_email, admin_password) {
        (Some(email), Some(password)) => {
            db::seed_admin(&pool, &email, &password, &admin_name)
                .await
                .expect("Failed to seed admin user");
        }
        _ if config.is_production() => {
            panic!("ADMIN_EMAIL and ADMIN_PASSWORD must be set in production");
        }
        _ => {
            tracing::warn!(
                "ADMIN_EMAIL/ADMIN_PASSWORD not set; skipping admin seed in non-production mode"
            );
        }
    }

    // Ensure uploads directory exists
    let upload_dir = config.upload_dir.clone();

    let email_service = if config.smtp_user.is_empty() {
        tracing::warn!("SMTP_USER not configured — email sending is disabled");
        None
    } else {
        match email::EmailService::new(&config) {
            Ok(svc) => {
                tracing::info!("Email service initialized (SMTP: {})", config.smtp_host);
                Some(Arc::new(svc))
            }
            Err(e) => {
                tracing::error!("Failed to initialize email service: {e}");
                None
            }
        }
    };

    let media_backend = services::MediaBackend::resolve(config.upload_dir.clone());

    let state = AppState {
        db: pool,
        config: Arc::new(config),
        email_service,
        media_backend,
    };

    let allowed_origins = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();
    if allowed_origins.is_empty() {
        panic!("CORS_ALLOWED_ORIGINS (or FRONTEND_URL) must contain at least one valid origin");
    }

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        // Avoid preflight failures when browsers/extensions send non-standard request headers.
        .allow_headers(Any);
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .expect("Failed to create uploads directory");

    let mount_local_uploads =
        !(state.config.is_production() && state.media_backend.is_r2());

    let mut app = Router::new()
        // Auth & analytics
        .nest("/api/auth", handlers::auth::router())
        .nest("/api/analytics", handlers::analytics::router())
        // Admin routes
        .nest("/api/admin", handlers::admin::router())
        .nest("/api/admin/blog", handlers::blog::admin_router())
        .nest("/api/admin/courses", handlers::courses::admin_router())
        .nest("/api/admin/pricing", handlers::pricing::admin_router())
        .nest("/api/admin/coupons", handlers::coupons::admin_router())
        .nest("/api/admin/popups", handlers::popups::admin_router())
        // Public routes
        .nest("/api/blog", handlers::blog::public_router())
        .nest("/api/courses", handlers::courses::public_router())
        .nest("/api/pricing", handlers::pricing::public_router())
        .nest("/api/coupons", handlers::coupons::public_router())
        .nest("/api/popups", handlers::popups::public_router())
        // Member routes
        .nest("/api/member", handlers::member::router())
        .nest("/api/member", handlers::courses::member_router())
        // Webhooks
        .nest("/api/webhooks", handlers::webhooks::router());

    if mount_local_uploads {
        app = app.nest_service("/uploads", ServeDir::new(&upload_dir));
    }

    let app = app
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();

    tracing::info!("Swings API listening on port {port}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
```

### 2.5 Route handler files (`backend/src/handlers/`)

#### `backend/src/handlers/admin.rs`

```rust
// backend/src/handlers/admin.rs
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration, NaiveDate};
use uuid::Uuid;
use validator::Validate;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::AdminUser,
    models::*,
    stripe_api, AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/stats", get(dashboard_stats))
        .route("/analytics/summary", get(analytics_summary))
        // Members
        .route("/members", get(list_members))
        .route("/members/{id}", get(get_member))
        .route(
            "/members/{id}/subscription",
            get(get_member_subscription_admin),
        )
        .route(
            "/members/{id}/billing-portal",
            post(admin_member_billing_portal),
        )
        .route(
            "/members/{id}/subscription/cancel",
            post(admin_member_subscription_cancel),
        )
        .route(
            "/members/{id}/subscription/resume",
            post(admin_member_subscription_resume),
        )
        .route("/members/{id}/role", put(update_member_role))
        .route("/members/{id}", delete(delete_member))
        // Watchlists
        .route("/watchlists", get(list_watchlists))
        .route("/watchlists", post(create_watchlist))
        .route("/watchlists/{id}", get(get_watchlist))
        .route("/watchlists/{id}", put(update_watchlist))
        .route("/watchlists/{id}", delete(delete_watchlist))
        // Watchlist Alerts
        .route("/watchlists/{id}/alerts", get(get_watchlist_alerts))
        .route("/watchlists/{id}/alerts", post(create_alert))
        .route("/alerts/{id}", put(update_alert))
        .route("/alerts/{id}", delete(delete_alert))
}

// ── Dashboard ───────────────────────────────────────────────────────────

async fn dashboard_stats(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<AdminStats>> {
    let (users, total_members) = db::list_users(&state.db, 0, 1).await?;
    let _ = users;
    let active_subscriptions = db::count_active_subscriptions(&state.db).await?;
    let monthly = db::count_subscriptions_by_plan(&state.db, &SubscriptionPlan::Monthly).await?;
    let annual = db::count_subscriptions_by_plan(&state.db, &SubscriptionPlan::Annual).await?;
    let total_watchlists = db::count_watchlists(&state.db).await?;
    let total_enrollments = db::count_enrollments(&state.db).await?;
    let recent = db::recent_members(&state.db, 5).await?;

    Ok(Json(AdminStats {
        total_members,
        active_subscriptions,
        monthly_subscriptions: monthly,
        annual_subscriptions: annual,
        total_watchlists,
        total_enrollments,
        recent_members: recent.into_iter().map(UserResponse::from).collect(),
    }))
}

async fn analytics_summary(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<AnalyticsSummaryQuery>,
) -> AppResult<Json<AnalyticsSummary>> {
    let from_date = NaiveDate::parse_from_str(&q.from, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("invalid from date (use YYYY-MM-DD)".to_string()))?;
    let to_date = NaiveDate::parse_from_str(&q.to, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("invalid to date (use YYYY-MM-DD)".to_string()))?;
    if to_date < from_date {
        return Err(AppError::BadRequest(
            "to must be on or after from".to_string(),
        ));
    }

    let start = from_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end_exclusive = (to_date + Duration::days(1))
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    let (total_page_views, total_sessions, total_impressions) =
        db::analytics_totals(&state.db, start, end_exclusive).await?;

    let days = db::analytics_time_series(&state.db, start, end_exclusive).await?;
    let tops = db::analytics_top_pages(&state.db, start, end_exclusive, 25).await?;
    let ctr_rows = db::analytics_ctr_breakdown(&state.db, start, end_exclusive).await?;

    let time_series = days
        .into_iter()
        .map(|d| AnalyticsTimeBucket {
            date: d.day.format("%Y-%m-%d").to_string(),
            page_views: d.page_views,
            unique_sessions: d.unique_sessions,
            impressions: d.impressions,
        })
        .collect();

    let top_pages = tops
        .into_iter()
        .map(|t| AnalyticsTopPage {
            path: t.path,
            views: t.views,
        })
        .collect();

    let ctr_series = ctr_rows
        .into_iter()
        .map(|r| {
            let ctr = if r.impressions > 0 {
                r.clicks as f64 / r.impressions as f64
            } else {
                0.0
            };
            AnalyticsCtrPoint {
                date: r.day.format("%Y-%m-%d").to_string(),
                cta_id: r.cta_id,
                impressions: r.impressions,
                clicks: r.clicks,
                ctr,
            }
        })
        .collect();

    Ok(Json(AnalyticsSummary {
        from: q.from.clone(),
        to: q.to.clone(),
        total_page_views,
        total_sessions,
        total_impressions,
        time_series,
        top_pages,
        ctr_series,
    }))
}

// ── Members ─────────────────────────────────────────────────────────────

async fn list_members(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<UserResponse>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let (users, total) = db::list_users(&state.db, offset, per_page).await?;
    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: users.into_iter().map(UserResponse::from).collect(),
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn get_member(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<UserResponse>> {
    let user = db::find_user_by_id(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;
    Ok(Json(user.into()))
}

async fn get_member_subscription_admin(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<SubscriptionStatusResponse>> {
    db::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;

    let sub = db::find_subscription_by_user(&state.db, user_id).await?;
    let is_active = sub
        .as_ref()
        .map(|s| s.status == SubscriptionStatus::Active || s.status == SubscriptionStatus::Trialing)
        .unwrap_or(false);

    Ok(Json(SubscriptionStatusResponse {
        subscription: sub,
        is_active,
    }))
}

async fn admin_member_billing_portal(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
    Json(req): Json<BillingPortalRequest>,
) -> AppResult<Json<BillingPortalResponse>> {
    db::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;

    let sub = db::find_subscription_by_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Member has no subscription".to_string()))?;

    let base = state.config.frontend_url.trim_end_matches('/');
    let return_url = req
        .return_url
        .unwrap_or_else(|| format!("{base}/dashboard/account"));

    let url =
        stripe_api::create_billing_portal_session(&state, &sub.stripe_customer_id, &return_url)
            .await?;

    Ok(Json(BillingPortalResponse { url }))
}

async fn admin_member_subscription_cancel(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;

    let sub = db::find_subscription_by_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Member has no subscription".to_string()))?;

    stripe_api::set_subscription_cancel_at_period_end(&state, &sub.stripe_subscription_id, true)
        .await?;

    Ok(Json(
        serde_json::json!({ "ok": true, "cancel_at_period_end": true }),
    ))
}

async fn admin_member_subscription_resume(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;

    let sub = db::find_subscription_by_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Member has no subscription".to_string()))?;

    stripe_api::set_subscription_cancel_at_period_end(&state, &sub.stripe_subscription_id, false)
        .await?;

    Ok(Json(
        serde_json::json!({ "ok": true, "cancel_at_period_end": false }),
    ))
}

#[derive(serde::Deserialize)]
struct RoleUpdate {
    role: UserRole,
}

async fn update_member_role(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RoleUpdate>,
) -> AppResult<Json<UserResponse>> {
    let user = db::update_user_role(&state.db, id, &req.role).await?;
    Ok(Json(user.into()))
}

async fn delete_member(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_user(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "message": "Member deleted" })))
}

// ── Watchlists ──────────────────────────────────────────────────────────

async fn list_watchlists(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<Watchlist>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let (watchlists, total) = db::list_watchlists(&state.db, offset, per_page, false).await?;
    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: watchlists,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn create_watchlist(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreateWatchlistRequest>,
) -> AppResult<Json<Watchlist>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let watchlist = db::create_watchlist(
        &state.db,
        &req.title,
        req.week_of,
        req.video_url.as_deref(),
        req.notes.as_deref(),
        req.published.unwrap_or(false),
    )
    .await?;

    Ok(Json(watchlist))
}

async fn get_watchlist(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<WatchlistWithAlerts>> {
    let watchlist = db::get_watchlist(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Watchlist not found".to_string()))?;

    let alerts = db::get_alerts_for_watchlist(&state.db, id).await?;

    Ok(Json(WatchlistWithAlerts { watchlist, alerts }))
}

async fn update_watchlist(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWatchlistRequest>,
) -> AppResult<Json<Watchlist>> {
    let watchlist = db::update_watchlist(&state.db, id, &req).await?;
    Ok(Json(watchlist))
}

async fn delete_watchlist(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_watchlist(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "message": "Watchlist deleted" })))
}

// ── Alerts ──────────────────────────────────────────────────────────────

async fn get_watchlist_alerts(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<WatchlistAlert>>> {
    let alerts = db::get_alerts_for_watchlist(&state.db, id).await?;
    Ok(Json(alerts))
}

async fn create_alert(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(watchlist_id): Path<Uuid>,
    Json(req): Json<CreateAlertRequest>,
) -> AppResult<Json<WatchlistAlert>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let alert = db::create_alert(&state.db, watchlist_id, &req).await?;
    Ok(Json(alert))
}

async fn update_alert(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAlertRequest>,
) -> AppResult<Json<WatchlistAlert>> {
    let alert = db::update_alert(&state.db, id, &req).await?;
    Ok(Json(alert))
}

async fn delete_alert(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_alert(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "message": "Alert deleted" })))
}
```

#### `backend/src/handlers/analytics.rs`

```rust
// backend/src/handlers/analytics.rs
use axum::{extract::State, routing::post, Json, Router};

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::OptionalAuthUser,
    models::AnalyticsIngestRequest,
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/events", post(ingest_events))
}

const MAX_EVENTS_PER_REQUEST: usize = 64;
const MAX_PATH_LEN: usize = 2048;

/// Public ingest for SPA analytics (optional Bearer links session to logged-in user).
async fn ingest_events(
    State(state): State<AppState>,
    opt: OptionalAuthUser,
    Json(req): Json<AnalyticsIngestRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.events.is_empty() {
        return Err(AppError::BadRequest("events cannot be empty".to_string()));
    }
    if req.events.len() > MAX_EVENTS_PER_REQUEST {
        return Err(AppError::BadRequest(format!(
            "at most {MAX_EVENTS_PER_REQUEST} events per request"
        )));
    }

    let mut batch: Vec<(String, String, Option<String>, serde_json::Value)> =
        Vec::with_capacity(req.events.len());

    for ev in req.events {
        let et = ev.event_type.to_lowercase();
        if et != "page_view" && et != "impression" && et != "click" {
            return Err(AppError::BadRequest(
                "event_type must be page_view, impression, or click".to_string(),
            ));
        }
        let path = ev.path.trim().to_string();
        if path.is_empty() {
            return Err(AppError::BadRequest("path is required".to_string()));
        }
        if path.len() > MAX_PATH_LEN {
            return Err(AppError::BadRequest("path too long".to_string()));
        }
        let referrer = ev
            .referrer
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty());

        let metadata = if ev.metadata.is_null() {
            serde_json::json!({})
        } else {
            ev.metadata
        };

        batch.push((et, path, referrer, metadata));
    }

    db::ingest_analytics_events(&state.db, req.session_id, opt.user_id, batch).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
```

#### `backend/src/handlers/auth.rs`

```rust
// backend/src/handlers/auth.rs
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{extract::State, routing::post, Json, Router};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use validator::Validate;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::{AuthUser, Claims},
    models::*,
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(
            Router::new()
                .route("/register", post(register))
                .layer(crate::middleware::rate_limit::register_layer()),
        )
        .merge(
            Router::new()
                .route("/login", post(login))
                .layer(crate::middleware::rate_limit::login_layer()),
        )
        .merge(
            Router::new()
                .route("/forgot-password", post(forgot_password))
                .layer(crate::middleware::rate_limit::forgot_password_layer()),
        )
        .route("/refresh", post(refresh))
        .route("/me", axum::routing::get(me))
        .route("/logout", post(logout))
        .route("/reset-password", post(reset_password))
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if db::find_user_by_email(&state.db, &req.email)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("Email already registered".to_string()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| AppError::BadRequest(format!("Password hash error: {e}")))?
        .to_string();

    let user = db::create_user(&state.db, &req.email, &password_hash, &req.name).await?;

    let (access_token, refresh_token) = generate_tokens(&state, &user).await?;

    Ok(Json(AuthResponse {
        user: user.into(),
        access_token,
        refresh_token,
    }))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user = db::find_user_by_email(&state.db, &req.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let parsed_hash = PasswordHash::new(&user.password_hash).map_err(|_| AppError::Unauthorized)?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized)?;

    let (access_token, refresh_token) = generate_tokens(&state, &user).await?;

    Ok(Json(AuthResponse {
        user: user.into(),
        access_token,
        refresh_token,
    }))
}

async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<TokenResponse>> {
    let token_hash = hash_token(&req.refresh_token);

    let stored = db::find_refresh_token(&state.db, &token_hash)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if stored.used {
        tracing::warn!(
            user_id = %stored.user_id,
            family_id = %stored.family_id,
            "Refresh token reuse detected — revoking token family"
        );
        db::delete_refresh_tokens_by_family(&state.db, stored.family_id).await?;
        return Err(AppError::TokenReuseDetected(
            "Session invalidated due to token reuse".to_string(),
        ));
    }

    db::mark_refresh_token_used(&state.db, stored.id).await?;

    let user = db::find_user_by_id(&state.db, stored.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let now = Utc::now();
    let claims = Claims {
        sub: user.id,
        role: format!("{:?}", user.role).to_lowercase(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(state.config.jwt_expiration_hours)).timestamp() as usize,
    };

    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::BadRequest(format!("Token generation failed: {e}")))?;

    let new_refresh = Uuid::new_v4().to_string();
    let new_hash = hash_token(&new_refresh);
    let expires_at = now + Duration::days(state.config.refresh_token_expiration_days);

    db::store_refresh_token(
        &state.db,
        stored.user_id,
        &new_hash,
        expires_at,
        stored.family_id,
        false,
    )
    .await?;

    Ok(Json(TokenResponse {
        access_token,
        refresh_token: new_refresh,
    }))
}

async fn me(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<UserResponse>> {
    let user = db::find_user_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}

async fn logout(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_user_refresh_tokens(&state.db, auth.user_id).await?;
    Ok(Json(serde_json::json!({ "message": "Logged out" })))
}

// ── Forgot / Reset Password ─────────────────────────────────────────────

async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Always return success to prevent email enumeration
    let user = db::find_user_by_email(&state.db, &req.email).await?;

    if let Some(user) = user {
        let raw_token = Uuid::new_v4().to_string();
        let token_hash = hash_token(&raw_token);
        let expires_at = Utc::now() + Duration::hours(1);

        db::create_password_reset_token(&state.db, user.id, &token_hash, expires_at).await?;

        // Build reset URL
        let reset_url = format!(
            "{}/admin/reset-password?token={}",
            state.config.frontend_url, raw_token
        );

        // TODO: Send email with reset_url in production
        // For now, log the reset link for development
        tracing::info!(
            "Password reset requested for {}. Reset URL: {}",
            req.email,
            reset_url
        );
    }

    Ok(Json(serde_json::json!({
        "message": "If an account with that email exists, a password reset link has been sent."
    })))
}

async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let token_hash = hash_token(&req.token);

    let reset_token = db::find_password_reset_token(&state.db, &token_hash)
        .await?
        .ok_or(AppError::BadRequest(
            "Invalid or expired reset token".to_string(),
        ))?;

    // Hash new password
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(req.new_password.as_bytes(), &salt)
        .map_err(|e| AppError::BadRequest(format!("Password hash error: {e}")))?
        .to_string();

    // Update password and mark token as used
    db::update_user_password(&state.db, reset_token.user_id, &password_hash).await?;
    db::mark_reset_token_used(&state.db, reset_token.id).await?;

    // Invalidate all refresh tokens for security
    db::delete_user_refresh_tokens(&state.db, reset_token.user_id).await?;

    tracing::info!("Password reset completed for user {}", reset_token.user_id);

    Ok(Json(serde_json::json!({
        "message": "Password has been reset successfully. Please log in with your new password."
    })))
}

// ── Helpers ─────────────────────────────────────────────────────────────

async fn generate_tokens(state: &AppState, user: &User) -> AppResult<(String, String)> {
    let now = Utc::now();

    let claims = Claims {
        sub: user.id,
        role: format!("{:?}", user.role).to_lowercase(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(state.config.jwt_expiration_hours)).timestamp() as usize,
    };

    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::BadRequest(format!("Token generation failed: {e}")))?;

    let refresh_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&refresh_token);
    let expires_at = now + Duration::days(state.config.refresh_token_expiration_days);
    let family_id = Uuid::new_v4();

    db::store_refresh_token(
        &state.db,
        user.id,
        &token_hash,
        expires_at,
        family_id,
        false,
    )
    .await?;

    Ok((access_token, refresh_token))
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
```

#### `backend/src/handlers/blog.rs`

```rust
// backend/src/handlers/blog.rs
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Multipart, Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use bytes::Bytes;
use uuid::Uuid;
use validator::Validate;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::AdminUser,
    models::*,
    services::{MediaBackend, R2Storage},
    AppState,
};

// ── Admin Blog Router ──────────────────────────────────────────────────

pub fn admin_router() -> Router<AppState> {
    Router::new()
        // Posts
        .route("/posts", get(admin_list_posts))
        .route("/posts", post(admin_create_post))
        .route("/posts/{id}", get(admin_get_post))
        .route("/posts/{id}", put(admin_update_post))
        .route("/posts/{id}", delete(admin_delete_post))
        .route("/posts/{id}/restore", post(admin_restore_post_from_trash))
        .route("/posts/{id}/status", put(admin_update_post_status))
        .route("/posts/{id}/autosave", post(admin_autosave_post))
        .route("/posts/{id}/revisions", get(admin_list_revisions))
        .route(
            "/posts/{id}/revisions/{rev_id}/restore",
            post(admin_restore_revision),
        )
        .route("/posts/{id}/meta", get(admin_list_post_meta))
        .route("/posts/{id}/meta", post(admin_upsert_post_meta))
        .route("/posts/{id}/meta/{key}", delete(admin_delete_post_meta))
        // Categories
        .route("/categories", get(admin_list_categories))
        .route("/categories", post(admin_create_category))
        .route("/categories/{id}", put(admin_update_category))
        .route("/categories/{id}", delete(admin_delete_category))
        // Tags
        .route("/tags", get(admin_list_tags))
        .route("/tags", post(admin_create_tag))
        .route("/tags/{id}", delete(admin_delete_tag))
        // Media
        .route("/media", get(admin_list_media))
        .route("/media/upload", post(admin_upload_media))
        .route("/media/{id}", put(admin_update_media))
        .route("/media/{id}", delete(admin_delete_media))
}

// ── Public Blog Router ─────────────────────────────────────────────────

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/posts", get(public_list_posts))
        .route("/posts/{slug}", get(public_get_post))
        .route("/posts/{slug}/unlock", post(public_unlock_post))
        .route("/categories", get(public_list_categories))
        .route("/tags", get(public_list_tags))
        .route("/posts/category/{slug}", get(public_posts_by_category))
        .route("/posts/tag/{slug}", get(public_posts_by_tag))
        .route("/slugs", get(public_all_slugs))
}

// ── Helper: Build full post response ───────────────────────────────────

async fn build_post_response(pool: &sqlx::PgPool, post: BlogPost) -> AppResult<BlogPostResponse> {
    let author = db::find_user_by_id(pool, post.author_id)
        .await?
        .ok_or(AppError::NotFound("Author not found".to_string()))?;
    let categories = db::get_categories_for_post(pool, post.id).await?;
    let tags = db::get_tags_for_post(pool, post.id).await?;
    let meta = db::list_post_meta(pool, post.id).await?;
    let featured_image_url = if let Some(img_id) = post.featured_image_id {
        db::get_media(pool, img_id).await?.map(|m| m.url)
    } else {
        None
    };

    Ok(BlogPostResponse {
        id: post.id,
        author_id: post.author_id,
        author_name: author.name.clone(),
        author_avatar: author.avatar_url.clone(),
        author_position: author.position.clone(),
        author_bio: author.bio.clone(),
        author_website: author.website_url.clone(),
        author_twitter: author.twitter_url.clone(),
        author_linkedin: author.linkedin_url.clone(),
        author_youtube: author.youtube_url.clone(),
        title: post.title,
        slug: post.slug,
        content: post.content,
        content_json: post.content_json,
        excerpt: post.excerpt,
        featured_image_url,
        status: post.status,
        pre_trash_status: post.pre_trash_status,
        trashed_at: post.trashed_at,
        visibility: post.visibility.clone(),
        is_password_protected: post.password_hash.is_some(),
        format: post.format.clone(),
        is_sticky: post.is_sticky,
        allow_comments: post.allow_comments,
        meta_title: post.meta_title,
        meta_description: post.meta_description,
        canonical_url: post.canonical_url,
        og_image_url: post.og_image_url,
        reading_time_minutes: post.reading_time_minutes,
        word_count: post.word_count,
        categories,
        tags,
        meta,
        scheduled_at: post.scheduled_at,
        published_at: post.published_at,
        created_at: post.created_at,
        updated_at: post.updated_at,
    })
}

async fn build_post_list_item(pool: &sqlx::PgPool, post: BlogPost) -> AppResult<BlogPostListItem> {
    let author = db::find_user_by_id(pool, post.author_id).await?;
    let author_name = author
        .map(|a| a.name)
        .unwrap_or_else(|| "Unknown".to_string());
    let categories = db::get_categories_for_post(pool, post.id).await?;
    let tags = db::get_tags_for_post(pool, post.id).await?;
    let featured_image_url = if let Some(img_id) = post.featured_image_id {
        db::get_media(pool, img_id).await?.map(|m| m.url)
    } else {
        None
    };

    Ok(BlogPostListItem {
        id: post.id,
        author_id: post.author_id,
        author_name,
        title: post.title,
        slug: post.slug,
        excerpt: post.excerpt,
        featured_image_url,
        status: post.status,
        format: post.format,
        is_sticky: post.is_sticky,
        reading_time_minutes: post.reading_time_minutes,
        word_count: post.word_count,
        published_at: post.published_at,
        created_at: post.created_at,
        updated_at: post.updated_at,
        categories,
        tags,
    })
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN POST HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_posts(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PostListParams>,
) -> AppResult<Json<PaginatedResponse<BlogPostListItem>>> {
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let (posts, total) = db::list_blog_posts_admin(
        &state.db,
        offset,
        per_page,
        params.status.as_ref(),
        params.author_id,
        params.search.as_deref(),
    )
    .await?;

    let mut items = Vec::with_capacity(posts.len());
    for p in posts {
        items.push(build_post_list_item(&state.db, p).await?);
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: items,
        total,
        page,
        per_page,
        total_pages,
    }))
}

fn hash_post_password(plain: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::BadRequest(format!("Password hash error: {e}")))
}

async fn admin_create_post(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<CreatePostRequest>,
) -> AppResult<Json<BlogPostResponse>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let password_hash = if req.visibility.as_deref() == Some("password") {
        if let Some(ref pw) = req.post_password {
            if !pw.is_empty() {
                Some(hash_post_password(pw)?)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let effective_author = req.author_id.unwrap_or(admin.user_id);
    let post =
        db::create_blog_post(&state.db, effective_author, &req, password_hash.as_deref()).await?;

    // Set categories/tags if provided
    if let Some(ref cat_ids) = req.category_ids {
        db::set_post_categories(&state.db, post.id, cat_ids).await?;
    }
    if let Some(ref tag_ids) = req.tag_ids {
        db::set_post_tags(&state.db, post.id, tag_ids).await?;
    }

    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

async fn admin_get_post(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BlogPostResponse>> {
    let post = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

async fn admin_update_post(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePostRequest>,
) -> AppResult<Json<BlogPostResponse>> {
    let existing = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

    if existing.status == PostStatus::Trash {
        if let Some(ref s) = req.status {
            if s != &PostStatus::Trash {
                return Err(AppError::BadRequest(
                    "This post is in the trash. Restore it first (POST .../restore), or keep status as trash."
                        .to_string(),
                ));
            }
        }
    }

    // Create revision before updating
    db::create_blog_revision(
        &state.db,
        id,
        admin.user_id,
        &existing.title,
        &existing.content,
        existing.content_json.as_ref(),
    )
    .await?;

    let password_hash_update: Option<Option<String>> = if let Some(vis) = req.visibility.as_deref()
    {
        if vis == "password" {
            if let Some(ref pw) = req.post_password {
                if !pw.is_empty() {
                    Some(Some(hash_post_password(pw)?))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            Some(None)
        }
    } else if let Some(ref pw) = req.post_password {
        if !pw.is_empty() {
            Some(Some(hash_post_password(pw)?))
        } else {
            None
        }
    } else {
        None
    };

    let post =
        db::update_blog_post(&state.db, id, &req, password_hash_update, req.author_id).await?;

    // Update categories/tags if provided
    if let Some(ref cat_ids) = req.category_ids {
        db::set_post_categories(&state.db, post.id, cat_ids).await?;
    }
    if let Some(ref tag_ids) = req.tag_ids {
        db::set_post_tags(&state.db, post.id, tag_ids).await?;
    }

    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

async fn admin_delete_post(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let existing = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;
    if existing.status != PostStatus::Trash {
        return Err(AppError::BadRequest(
            "Only posts in the trash can be permanently deleted. Move the post to trash first."
                .to_string(),
        ));
    }
    db::delete_blog_post(&state.db, id).await?;
    Ok(Json(
        serde_json::json!({ "message": "Post permanently deleted" }),
    ))
}

async fn admin_restore_post_from_trash(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BlogPostResponse>> {
    let post = db::restore_post_from_trash(&state.db, id).await?;
    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

async fn admin_update_post_status(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePostStatusRequest>,
) -> AppResult<Json<BlogPostResponse>> {
    let existing = db::get_blog_post(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

    let post = if req.status == PostStatus::Trash {
        db::move_post_to_trash(&state.db, id).await?
    } else if existing.status == PostStatus::Trash {
        return Err(AppError::BadRequest(
            "This post is in the trash. Use POST /api/admin/blog/posts/{id}/restore to restore it."
                .to_string(),
        ));
    } else {
        db::update_post_status(&state.db, id, &req.status).await?
    };
    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

async fn admin_autosave_post(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AutosaveRequest>,
) -> AppResult<Json<serde_json::Value>> {
    db::autosave_blog_post(&state.db, id, &req).await?;
    Ok(Json(serde_json::json!({ "message": "Autosaved" })))
}

async fn admin_list_revisions(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<RevisionResponse>>> {
    let revisions = db::list_blog_revisions(&state.db, id).await?;
    let mut items = Vec::with_capacity(revisions.len());
    for rev in revisions {
        let author = db::find_user_by_id(&state.db, rev.author_id).await?;
        let author_name = author
            .map(|a| a.name)
            .unwrap_or_else(|| "Unknown".to_string());
        items.push(RevisionResponse {
            id: rev.id,
            post_id: rev.post_id,
            author_id: rev.author_id,
            author_name,
            title: rev.title,
            revision_number: rev.revision_number,
            created_at: rev.created_at,
        });
    }
    Ok(Json(items))
}

#[derive(serde::Deserialize)]
struct RevisionRestorePath {
    id: Uuid,
    rev_id: Uuid,
}

async fn admin_restore_revision(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(path): Path<RevisionRestorePath>,
) -> AppResult<Json<BlogPostResponse>> {
    let existing = db::get_blog_post(&state.db, path.id)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

    let revision = db::get_blog_revision(&state.db, path.rev_id)
        .await?
        .ok_or(AppError::NotFound("Revision not found".to_string()))?;

    // Create revision of current state before restoring
    db::create_blog_revision(
        &state.db,
        path.id,
        admin.user_id,
        &existing.title,
        &existing.content,
        existing.content_json.as_ref(),
    )
    .await?;

    // Restore from revision
    let req = UpdatePostRequest {
        title: Some(revision.title),
        content: Some(revision.content),
        content_json: revision.content_json,
        slug: None,
        excerpt: None,
        featured_image_id: None,
        status: None,
        visibility: None,
        post_password: None,
        is_sticky: None,
        allow_comments: None,
        meta_title: None,
        meta_description: None,
        canonical_url: None,
        og_image_url: None,
        category_ids: None,
        tag_ids: None,
        scheduled_at: None,
        author_id: None,
        format: None,
    };

    let post = db::update_blog_post(&state.db, path.id, &req, None, None).await?;
    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN CATEGORY HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_categories(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<BlogCategory>>> {
    let cats = db::list_blog_categories(&state.db).await?;
    Ok(Json(cats))
}

async fn admin_create_category(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreateCategoryRequest>,
) -> AppResult<Json<BlogCategory>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let cat = db::create_blog_category(&state.db, &req).await?;
    Ok(Json(cat))
}

async fn admin_update_category(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCategoryRequest>,
) -> AppResult<Json<BlogCategory>> {
    let cat = db::update_blog_category(&state.db, id, &req).await?;
    Ok(Json(cat))
}

async fn admin_delete_category(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_blog_category(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "message": "Category deleted" })))
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN TAG HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_tags(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<BlogTag>>> {
    let tags = db::list_blog_tags(&state.db).await?;
    Ok(Json(tags))
}

async fn admin_create_tag(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreateTagRequest>,
) -> AppResult<Json<BlogTag>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let tag = db::create_blog_tag(&state.db, &req).await?;
    Ok(Json(tag))
}

async fn admin_delete_tag(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_blog_tag(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "message": "Tag deleted" })))
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN MEDIA HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_media(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<Media>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let (items, total) = db::list_media(&state.db, offset, per_page).await?;
    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: items,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn admin_upload_media(
    State(state): State<AppState>,
    admin: AdminUser,
    mut multipart: Multipart,
) -> AppResult<Json<Media>> {
    let api_url = &state.config.api_url;

    let mut file_data: Option<Vec<u8>> = None;
    let mut original_filename = String::new();
    let mut content_type = String::new();
    let mut title: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            original_filename = field.file_name().unwrap_or("unknown").to_string();
            content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read file: {}", e)))?;
            file_data = Some(data.to_vec());
        } else if name == "title" {
            let text = field
                .text()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read title: {}", e)))?;
            if !text.trim().is_empty() {
                title = Some(text.trim().to_string());
            }
        }
    }

    let data = file_data.ok_or(AppError::BadRequest("No file provided".to_string()))?;

    // Validate MIME type
    let allowed = [
        "image/jpeg",
        "image/png",
        "image/gif",
        "image/webp",
        "image/avif",
        "image/svg+xml",
        "application/pdf",
    ];
    if !allowed.contains(&content_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "File type '{}' not allowed",
            content_type
        )));
    }

    let file_size = data.len() as i64;

    let (storage_path, url, stored_filename) = match &state.media_backend {
        MediaBackend::R2(r2) => {
            let key = R2Storage::generate_key(&original_filename);
            let url = r2
                .upload(&key, Bytes::from(data), &content_type)
                .await?;
            let stored_filename = key
                .rsplit('/')
                .next()
                .unwrap_or(key.as_str())
                .to_string();
            (key, url, stored_filename)
        }
        MediaBackend::Local { upload_dir } => {
            tokio::fs::create_dir_all(upload_dir)
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to create upload dir: {}", e)))?;

            let safe_name = sanitize_filename::sanitize(&original_filename);
            let ext = std::path::Path::new(&safe_name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin");
            let unique_name = format!("{}.{}", Uuid::new_v4(), ext);
            let storage_path = format!("{}/{}", upload_dir, unique_name);
            let url = format!("{}/uploads/{}", api_url, unique_name);

            tokio::fs::write(&storage_path, &data)
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to write file: {}", e)))?;

            (storage_path, url, unique_name)
        }
    };

    // Get image dimensions if applicable
    let (width, height) = if content_type.starts_with("image/") && content_type != "image/svg+xml" {
        // Simple dimension detection: read first bytes
        // For now, store None; could add `image` crate later
        (None, None)
    } else {
        (None, None)
    };

    let media = db::create_media(
        &state.db,
        admin.user_id,
        &stored_filename,
        &original_filename,
        title.as_deref(),
        &content_type,
        file_size,
        width,
        height,
        &storage_path,
        &url,
    )
    .await?;

    Ok(Json(media))
}

async fn admin_update_media(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMediaRequest>,
) -> AppResult<Json<Media>> {
    let media = db::update_media(&state.db, id, &req).await?;
    Ok(Json(media))
}

async fn admin_delete_media(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let media = db::delete_media(&state.db, id).await?;

    if let Some(m) = media {
        match &state.media_backend {
            MediaBackend::R2(r2) if m.storage_path.starts_with("media/") => {
                if let Err(e) = r2.delete_object(&m.storage_path).await {
                    tracing::warn!(
                        key = %m.storage_path,
                        error = %e,
                        "Failed to delete object from R2 (metadata already removed)"
                    );
                }
            }
            _ => {
                let _ = tokio::fs::remove_file(&m.storage_path).await;
            }
        }
    }

    Ok(Json(serde_json::json!({ "message": "Media deleted" })))
}

// ══════════════════════════════════════════════════════════════════════
// PUBLIC BLOG HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn public_list_posts(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<BlogPostListItem>>> {
    let per_page = params.per_page();
    let page = params.page.unwrap_or(1).max(1);
    let offset = params.offset();

    let (posts, total) = db::list_published_posts(&state.db, offset, per_page).await?;

    let mut items = Vec::with_capacity(posts.len());
    for p in posts {
        items.push(build_post_list_item(&state.db, p).await?);
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: items,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn public_get_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<BlogPostResponse>> {
    let post = db::get_blog_post_by_slug(&state.db, &slug)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

    let mut response = build_post_response(&state.db, post).await?;
    if response.is_password_protected {
        response.content = String::new();
        response.content_json = None;
    }
    Ok(Json(response))
}

async fn public_unlock_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<VerifyPostPasswordRequest>,
) -> AppResult<Json<BlogPostResponse>> {
    let post = db::get_blog_post_by_slug(&state.db, &slug)
        .await?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

    let hash_str = post
        .password_hash
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("Post is not password protected".to_string()))?;

    let parsed = PasswordHash::new(hash_str)
        .map_err(|_| AppError::BadRequest("Invalid password hash".to_string()))?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed)
        .map_err(|_| AppError::Unauthorized)?;

    let response = build_post_response(&state.db, post).await?;
    Ok(Json(response))
}

async fn public_list_categories(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<BlogCategory>>> {
    let cats = db::list_blog_categories(&state.db).await?;
    Ok(Json(cats))
}

async fn public_list_tags(State(state): State<AppState>) -> AppResult<Json<Vec<BlogTag>>> {
    let tags = db::list_blog_tags(&state.db).await?;
    Ok(Json(tags))
}

async fn public_posts_by_category(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<BlogPostListItem>>> {
    let per_page = params.per_page();
    let page = params.page.unwrap_or(1).max(1);
    let offset = params.offset();

    let (posts, total) =
        db::list_published_posts_by_category(&state.db, &slug, offset, per_page).await?;

    let mut items = Vec::with_capacity(posts.len());
    for p in posts {
        items.push(build_post_list_item(&state.db, p).await?);
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: items,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn public_posts_by_tag(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<BlogPostListItem>>> {
    let per_page = params.per_page();
    let page = params.page.unwrap_or(1).max(1);
    let offset = params.offset();

    let (posts, total) =
        db::list_published_posts_by_tag(&state.db, &slug, offset, per_page).await?;

    let mut items = Vec::with_capacity(posts.len());
    for p in posts {
        items.push(build_post_list_item(&state.db, p).await?);
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: items,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn public_all_slugs(State(state): State<AppState>) -> AppResult<Json<Vec<String>>> {
    let slugs = db::list_all_published_slugs(&state.db).await?;
    Ok(Json(slugs))
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN POST META HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_post_meta(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<PostMeta>>> {
    let items = db::list_post_meta(&state.db, id).await?;
    Ok(Json(items))
}

async fn admin_upsert_post_meta(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpsertPostMetaRequest>,
) -> AppResult<Json<PostMeta>> {
    let item = db::upsert_post_meta(&state.db, id, &req.meta_key, &req.meta_value).await?;
    Ok(Json(item))
}

async fn admin_delete_post_meta(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path((id, key)): Path<(Uuid, String)>,
) -> AppResult<axum::http::StatusCode> {
    db::delete_post_meta(&state.db, id, &key).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
```

#### `backend/src/handlers/coupons.rs`

```rust
// backend/src/handlers/coupons.rs
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use rand::Rng;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    extractors::{AdminUser, AuthUser},
    models::*,
    AppState,
};

// ── Admin Coupon Router ────────────────────────────────────────────────

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/coupons", get(admin_list_coupons))
        .route("/coupons", post(admin_create_coupon))
        .route("/coupons/bulk", post(admin_bulk_create_coupons))
        .route("/coupons/{id}", get(admin_get_coupon))
        .route("/coupons/{id}", put(admin_update_coupon))
        .route("/coupons/{id}", delete(admin_delete_coupon))
        .route("/coupons/{id}/toggle", post(admin_toggle_coupon))
        .route("/coupons/{id}/usages", get(admin_list_coupon_usages))
}

// ── Public Coupon Router ───────────────────────────────────────────────

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/coupons/validate", post(public_validate_coupon))
        .route("/coupons/apply", post(public_apply_coupon))
}

// ── Query / Request / Response types ───────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CouponListParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub filter: Option<String>, // "active", "expired", "depleted"
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CouponWithStats {
    #[serde(flatten)]
    pub coupon: Coupon,
    pub total_usages: i64,
    pub total_discount_cents: i64,
}

#[derive(Debug, Serialize)]
pub struct CouponUsageWithUser {
    pub id: Uuid,
    pub coupon_id: Uuid,
    pub user_id: Uuid,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub subscription_id: Option<Uuid>,
    pub discount_applied_cents: i32,
    pub used_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyCouponRequest {
    pub code: String,
    pub plan_id: Option<Uuid>,
    pub course_id: Option<Uuid>,
    pub amount_cents: i32,
    pub subscription_id: Option<Uuid>,
}

// ── Helpers ────────────────────────────────────────────────────────────

fn generate_random_code(len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn generate_coupon_code(prefix: Option<&str>) -> String {
    let random_part = generate_random_code(8);
    match prefix {
        Some(p) if !p.is_empty() => format!("{}-{}", p.to_uppercase(), random_part),
        _ => random_part,
    }
}

/// Calculate the discount amount in cents for a given coupon and purchase amount.
fn calculate_discount(coupon: &Coupon, amount_cents: i32) -> Option<i32> {
    match coupon.discount_type {
        DiscountType::Percentage => {
            let value = coupon.discount_value.to_f64().unwrap_or(0.0);
            let raw = (amount_cents as f64 * value / 100.0).round() as i32;
            let capped = match coupon.max_discount_cents {
                Some(max) => raw.min(max),
                None => raw,
            };
            Some(capped.min(amount_cents).max(0))
        }
        DiscountType::FixedAmount => {
            let raw = coupon.discount_value.to_i32().unwrap_or(0);
            let capped = match coupon.max_discount_cents {
                Some(max) => raw.min(max),
                None => raw,
            };
            Some(capped.min(amount_cents).max(0))
        }
        DiscountType::FreeTrial => None,
    }
}

/// Shared validation logic. Returns the coupon if valid, or an error message.
async fn validate_coupon_inner(
    pool: &sqlx::PgPool,
    code: &str,
    plan_id: Option<Uuid>,
    course_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<Coupon, String> {
    // 1. Code exists and is_active
    let coupon: Option<Coupon> =
        sqlx::query_as("SELECT * FROM coupons WHERE UPPER(code) = UPPER($1)")
            .bind(code)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Database error: {e}"))?;

    let coupon = coupon.ok_or_else(|| "Coupon code not found".to_string())?;

    if !coupon.is_active {
        return Err("Coupon is not active".to_string());
    }

    // 2. Check time window
    let now = Utc::now();
    if let Some(starts_at) = coupon.starts_at {
        if now < starts_at {
            return Err("Coupon is not yet valid".to_string());
        }
    }
    if let Some(expires_at) = coupon.expires_at {
        if now > expires_at {
            return Err("Coupon has expired".to_string());
        }
    }

    // 3. Check global usage limit
    if let Some(limit) = coupon.usage_limit {
        if coupon.usage_count >= limit {
            return Err("Coupon usage limit has been reached".to_string());
        }
    }

    // 4. Check per-user limit
    if let Some(uid) = user_id {
        let user_usage_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM coupon_usages WHERE coupon_id = $1 AND user_id = $2",
        )
        .bind(coupon.id)
        .bind(uid)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Database error: {e}"))?;

        if user_usage_count >= coupon.per_user_limit as i64 {
            return Err(
                "You have already used this coupon the maximum number of times".to_string(),
            );
        }
    }

    // 5. Check applies_to scope
    match coupon.applies_to.as_str() {
        "all" => { /* applies to everything */ }
        "plan" => {
            if let Some(pid) = plan_id {
                if !coupon.applicable_plan_ids.is_empty()
                    && !coupon.applicable_plan_ids.contains(&pid)
                {
                    return Err("Coupon does not apply to this plan".to_string());
                }
            }
        }
        "course" => {
            if let Some(cid) = course_id {
                if !coupon.applicable_course_ids.is_empty()
                    && !coupon.applicable_course_ids.contains(&cid)
                {
                    return Err("Coupon does not apply to this course".to_string());
                }
            }
        }
        _ => { /* unknown scope, allow */ }
    }

    Ok(coupon)
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_coupons(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<CouponListParams>,
) -> AppResult<Json<PaginatedResponse<Coupon>>> {
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let filter = params.filter.as_deref().unwrap_or("all");
    let search_pattern = params.search.as_deref().map(|s| format!("%{}%", s));

    let (coupons, total): (Vec<Coupon>, i64) = match filter {
        "active" => {
            let rows: Vec<Coupon> = sqlx::query_as(
                r#"
                SELECT * FROM coupons
                WHERE is_active = true
                  AND (expires_at IS NULL OR expires_at > NOW())
                  AND (usage_limit IS NULL OR usage_count < usage_limit)
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&search_pattern)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM coupons
                WHERE is_active = true
                  AND (expires_at IS NULL OR expires_at > NOW())
                  AND (usage_limit IS NULL OR usage_count < usage_limit)
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                "#,
            )
            .bind(&search_pattern)
            .fetch_one(&state.db)
            .await?;

            (rows, count)
        }
        "expired" => {
            let rows: Vec<Coupon> = sqlx::query_as(
                r#"
                SELECT * FROM coupons
                WHERE expires_at IS NOT NULL AND expires_at <= NOW()
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&search_pattern)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM coupons
                WHERE expires_at IS NOT NULL AND expires_at <= NOW()
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                "#,
            )
            .bind(&search_pattern)
            .fetch_one(&state.db)
            .await?;

            (rows, count)
        }
        "depleted" => {
            let rows: Vec<Coupon> = sqlx::query_as(
                r#"
                SELECT * FROM coupons
                WHERE usage_limit IS NOT NULL AND usage_count >= usage_limit
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&search_pattern)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM coupons
                WHERE usage_limit IS NOT NULL AND usage_count >= usage_limit
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                "#,
            )
            .bind(&search_pattern)
            .fetch_one(&state.db)
            .await?;

            (rows, count)
        }
        _ => {
            // "all" or unrecognized
            let rows: Vec<Coupon> = sqlx::query_as(
                r#"
                SELECT * FROM coupons
                WHERE ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&search_pattern)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM coupons
                WHERE ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                "#,
            )
            .bind(&search_pattern)
            .fetch_one(&state.db)
            .await?;

            (rows, count)
        }
    };

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: coupons,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn admin_create_coupon(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<CreateCouponRequest>,
) -> AppResult<Json<Coupon>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let code = match req.code {
        Some(ref c) if !c.is_empty() => c.to_uppercase(),
        _ => generate_coupon_code(None),
    };

    // Check for duplicate code
    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM coupons WHERE UPPER(code) = UPPER($1)")
            .bind(&code)
            .fetch_optional(&state.db)
            .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(format!(
            "Coupon code '{}' already exists",
            code
        )));
    }

    let coupon: Coupon = sqlx::query_as(
        r#"
        INSERT INTO coupons (
            id, code, description, discount_type, discount_value,
            min_purchase_cents, max_discount_cents, applies_to,
            applicable_plan_ids, applicable_course_ids,
            usage_limit, usage_count, per_user_limit,
            starts_at, expires_at, is_active, stackable, first_purchase_only,
            created_by, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5,
            $6, $7, $8,
            $9, $10,
            $11, 0, $12,
            $13, $14, $15, $16, $17,
            $18, NOW(), NOW()
        )
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&code)
    .bind(&req.description)
    .bind(&req.discount_type)
    .bind(rust_decimal::Decimal::from_f64_retain(req.discount_value).unwrap_or_default())
    .bind(req.min_purchase_cents)
    .bind(req.max_discount_cents)
    .bind(req.applies_to.as_deref().unwrap_or("all"))
    .bind(req.applicable_plan_ids.as_deref().unwrap_or(&[]))
    .bind(req.applicable_course_ids.as_deref().unwrap_or(&[]))
    .bind(req.usage_limit)
    .bind(req.per_user_limit.unwrap_or(1))
    .bind(req.starts_at)
    .bind(req.expires_at)
    .bind(req.is_active.unwrap_or(true))
    .bind(req.stackable.unwrap_or(false))
    .bind(req.first_purchase_only.unwrap_or(false))
    .bind(admin.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(coupon))
}

async fn admin_get_coupon(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<CouponWithStats>> {
    let coupon: Coupon = sqlx::query_as("SELECT * FROM coupons WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Coupon not found".to_string()))?;

    let stats: (i64, Option<i64>) = sqlx::query_as(
        r#"
        SELECT COUNT(*), COALESCE(SUM(discount_applied_cents::bigint), 0)
        FROM coupon_usages
        WHERE coupon_id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(CouponWithStats {
        coupon,
        total_usages: stats.0,
        total_discount_cents: stats.1.unwrap_or(0),
    }))
}

async fn admin_update_coupon(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCouponRequest>,
) -> AppResult<Json<Coupon>> {
    let existing: Coupon = sqlx::query_as("SELECT * FROM coupons WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Coupon not found".to_string()))?;

    let discount_type = req.discount_type.unwrap_or(existing.discount_type);
    let discount_value = req
        .discount_value
        .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default())
        .unwrap_or(existing.discount_value);
    let description = req.description.or(existing.description);
    let min_purchase_cents = req.min_purchase_cents.or(existing.min_purchase_cents);
    let max_discount_cents = req.max_discount_cents.or(existing.max_discount_cents);
    let applies_to = req.applies_to.unwrap_or(existing.applies_to);
    let applicable_plan_ids = req
        .applicable_plan_ids
        .unwrap_or(existing.applicable_plan_ids);
    let applicable_course_ids = req
        .applicable_course_ids
        .unwrap_or(existing.applicable_course_ids);
    let usage_limit = req.usage_limit.or(existing.usage_limit);
    let per_user_limit = req.per_user_limit.unwrap_or(existing.per_user_limit);
    let starts_at = req.starts_at.or(existing.starts_at);
    let expires_at = req.expires_at.or(existing.expires_at);
    let is_active = req.is_active.unwrap_or(existing.is_active);
    let stackable = req.stackable.unwrap_or(existing.stackable);
    let first_purchase_only = req
        .first_purchase_only
        .unwrap_or(existing.first_purchase_only);

    let coupon: Coupon = sqlx::query_as(
        r#"
        UPDATE coupons SET
            description = $1,
            discount_type = $2,
            discount_value = $3,
            min_purchase_cents = $4,
            max_discount_cents = $5,
            applies_to = $6,
            applicable_plan_ids = $7,
            applicable_course_ids = $8,
            usage_limit = $9,
            per_user_limit = $10,
            starts_at = $11,
            expires_at = $12,
            is_active = $13,
            stackable = $14,
            first_purchase_only = $15,
            updated_at = NOW()
        WHERE id = $16
        RETURNING *
        "#,
    )
    .bind(&description)
    .bind(&discount_type)
    .bind(discount_value)
    .bind(min_purchase_cents)
    .bind(max_discount_cents)
    .bind(&applies_to)
    .bind(&applicable_plan_ids)
    .bind(&applicable_course_ids)
    .bind(usage_limit)
    .bind(per_user_limit)
    .bind(starts_at)
    .bind(expires_at)
    .bind(is_active)
    .bind(stackable)
    .bind(first_purchase_only)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(coupon))
}

async fn admin_delete_coupon(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM coupons WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Coupon not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn admin_toggle_coupon(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Coupon>> {
    let coupon: Coupon = sqlx::query_as(
        r#"
        UPDATE coupons
        SET is_active = NOT is_active, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Coupon not found".to_string()))?;

    Ok(Json(coupon))
}

async fn admin_bulk_create_coupons(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<BulkCouponRequest>,
) -> AppResult<Json<Vec<Coupon>>> {
    if req.count < 1 || req.count > 1000 {
        return Err(AppError::BadRequest(
            "Count must be between 1 and 1000".to_string(),
        ));
    }

    let discount_value =
        rust_decimal::Decimal::from_f64_retain(req.discount_value).unwrap_or_default();

    let mut created: Vec<Coupon> = Vec::with_capacity(req.count as usize);

    for _ in 0..req.count {
        let code = generate_coupon_code(req.prefix.as_deref());

        let coupon: Coupon = sqlx::query_as(
            r#"
            INSERT INTO coupons (
                id, code, description, discount_type, discount_value,
                min_purchase_cents, max_discount_cents, applies_to,
                applicable_plan_ids, applicable_course_ids,
                usage_limit, usage_count, per_user_limit,
                starts_at, expires_at, is_active, stackable, first_purchase_only,
                created_by, created_at, updated_at
            ) VALUES (
                $1, $2, NULL, $3, $4,
                NULL, NULL, 'all',
                '{}', '{}',
                $5, 0, 1,
                NULL, $6, true, false, false,
                $7, NOW(), NOW()
            )
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&code)
        .bind(&req.discount_type)
        .bind(discount_value)
        .bind(req.usage_limit)
        .bind(req.expires_at)
        .bind(admin.user_id)
        .fetch_one(&state.db)
        .await?;

        created.push(coupon);
    }

    Ok(Json(created))
}

async fn admin_list_coupon_usages(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<CouponUsageWithUser>>> {
    // Verify coupon exists
    let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM coupons WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("Coupon not found".to_string()));
    }

    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Uuid,
            Option<String>,
            Option<String>,
            Option<Uuid>,
            i32,
            chrono::DateTime<Utc>,
        ),
    >(
        r#"
        SELECT
            cu.id,
            cu.coupon_id,
            cu.user_id,
            u.name AS user_name,
            u.email AS user_email,
            cu.subscription_id,
            cu.discount_applied_cents,
            cu.used_at
        FROM coupon_usages cu
        LEFT JOIN users u ON u.id = cu.user_id
        WHERE cu.coupon_id = $1
        ORDER BY cu.used_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let usages: Vec<CouponUsageWithUser> = rows
        .into_iter()
        .map(|r| CouponUsageWithUser {
            id: r.0,
            coupon_id: r.1,
            user_id: r.2,
            user_name: r.3,
            user_email: r.4,
            subscription_id: r.5,
            discount_applied_cents: r.6,
            used_at: r.7,
        })
        .collect();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM coupon_usages WHERE coupon_id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: usages,
        total,
        page,
        per_page,
        total_pages,
    }))
}

// ══════════════════════════════════════════════════════════════════════
// PUBLIC HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn public_validate_coupon(
    State(state): State<AppState>,
    Json(req): Json<ValidateCouponRequest>,
) -> AppResult<Json<CouponValidationResponse>> {
    match validate_coupon_inner(&state.db, &req.code, req.plan_id, req.course_id, None).await {
        Ok(coupon) => {
            let message = match coupon.discount_type {
                DiscountType::Percentage => {
                    let val = coupon.discount_value.to_f64().unwrap_or(0.0);
                    format!("{}% discount", val)
                }
                DiscountType::FixedAmount => {
                    let val = coupon.discount_value.to_i32().unwrap_or(0);
                    format!("${:.2} discount", val as f64 / 100.0)
                }
                DiscountType::FreeTrial => "Free trial period".to_string(),
            };

            Ok(Json(CouponValidationResponse {
                valid: true,
                coupon: Some(coupon),
                discount_amount_cents: None,
                message,
            }))
        }
        Err(msg) => Ok(Json(CouponValidationResponse {
            valid: false,
            coupon: None,
            discount_amount_cents: None,
            message: msg,
        })),
    }
}

async fn public_apply_coupon(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ApplyCouponRequest>,
) -> AppResult<Json<CouponUsage>> {
    // Validate the coupon with user context
    let coupon = validate_coupon_inner(
        &state.db,
        &req.code,
        req.plan_id,
        req.course_id,
        Some(auth.user_id),
    )
    .await
    .map_err(AppError::BadRequest)?;

    // Check min purchase
    if let Some(min) = coupon.min_purchase_cents {
        if req.amount_cents < min {
            return Err(AppError::BadRequest(format!(
                "Minimum purchase of ${:.2} required",
                min as f64 / 100.0
            )));
        }
    }

    // Check first_purchase_only
    if coupon.first_purchase_only {
        let has_prior: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM coupon_usages WHERE user_id = $1")
                .bind(auth.user_id)
                .fetch_one(&state.db)
                .await?;

        if has_prior > 0 {
            return Err(AppError::BadRequest(
                "This coupon is only valid for first-time purchases".to_string(),
            ));
        }
    }

    // Calculate discount
    let discount_applied_cents = calculate_discount(&coupon, req.amount_cents).unwrap_or(0);

    // Insert usage record and increment usage_count in a transaction
    let mut tx = state.db.begin().await?;

    let usage: CouponUsage = sqlx::query_as(
        r#"
        INSERT INTO coupon_usages (id, coupon_id, user_id, subscription_id, discount_applied_cents, used_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(coupon.id)
    .bind(auth.user_id)
    .bind(req.subscription_id)
    .bind(discount_applied_cents)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE coupons SET usage_count = usage_count + 1, updated_at = NOW() WHERE id = $1",
    )
    .bind(coupon.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(usage))
}
```

#### `backend/src/handlers/courses.rs`

```rust
// backend/src/handlers/courses.rs
use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    extractors::{AdminUser, AuthUser},
    models::*,
    AppState,
};

// ── Routers ──────────────────────────────────────────────────────────────

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/courses", get(admin_list_courses).post(create_course))
        .route(
            "/courses/{id}",
            get(admin_get_course)
                .put(update_course)
                .delete(delete_course),
        )
        .route("/courses/{id}/publish", post(toggle_publish))
        .route("/courses/{id}/modules", post(create_module))
        .route(
            "/courses/{course_id}/modules/{module_id}",
            put(update_module).delete(delete_module),
        )
        .route(
            "/courses/{course_id}/modules/{module_id}/lessons",
            post(create_lesson),
        )
        .route(
            "/lessons/{lesson_id}",
            put(update_lesson).delete(delete_lesson),
        )
}

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/courses", get(public_list_courses))
        .route("/courses/{slug}", get(public_get_course))
}

pub fn member_router() -> Router<AppState> {
    Router::new()
        .route("/courses/{course_id}/enroll", post(enroll_course))
        .route("/courses/{course_id}/progress", get(get_course_progress))
        .route("/lessons/{lesson_id}/progress", put(update_lesson_progress))
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ── Admin Handlers ───────────────────────────────────────────────────────

async fn admin_list_courses(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<CourseListItem>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM courses")
        .fetch_one(&state.db)
        .await?;

    let courses = sqlx::query_as::<_, CourseListItem>(
        r#"
        SELECT c.id, c.title, c.slug, c.short_description, c.thumbnail_url,
               c.difficulty, u.name AS instructor_name, c.price_cents,
               c.is_free, c.is_included_in_subscription, c.published,
               c.estimated_duration_minutes,
               COUNT(cl.id)::bigint AS total_lessons,
               c.created_at
        FROM courses c
        JOIN users u ON u.id = c.instructor_id
        LEFT JOIN course_modules cm ON cm.course_id = c.id
        LEFT JOIN course_lessons cl ON cl.module_id = cm.id
        GROUP BY c.id, u.name
        ORDER BY c.sort_order ASC, c.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total_pages = (total + per_page - 1) / per_page;

    Ok(Json(PaginatedResponse {
        data: courses,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn create_course(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<CreateCourseRequest>,
) -> AppResult<Json<Course>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.title));
    let description = req.description.as_deref().unwrap_or("");
    let difficulty = req.difficulty.as_deref().unwrap_or("beginner");
    let price_cents = req.price_cents.unwrap_or(0);
    let currency = req.currency.as_deref().unwrap_or("usd");
    let is_free = req.is_free.unwrap_or(false);
    let is_included_in_subscription = req.is_included_in_subscription.unwrap_or(false);
    let sort_order = req.sort_order.unwrap_or(0);
    let published = req.published.unwrap_or(false);
    let estimated_duration_minutes = req.estimated_duration_minutes.unwrap_or(0);
    let published_at: Option<chrono::DateTime<Utc>> =
        if published { Some(Utc::now()) } else { None };

    let course = sqlx::query_as::<_, Course>(
        r#"
        INSERT INTO courses
            (title, slug, description, short_description, thumbnail_url,
             trailer_video_url, difficulty, instructor_id, price_cents, currency,
             is_free, is_included_in_subscription, sort_order, published,
             published_at, estimated_duration_minutes)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        RETURNING id, title, slug, description, short_description, thumbnail_url,
                  trailer_video_url, difficulty, instructor_id, price_cents, currency,
                  is_free, is_included_in_subscription, sort_order, published,
                  published_at, estimated_duration_minutes, created_at, updated_at
        "#,
    )
    .bind(&req.title)
    .bind(&slug)
    .bind(description)
    .bind(&req.short_description)
    .bind(&req.thumbnail_url)
    .bind(&req.trailer_video_url)
    .bind(difficulty)
    .bind(admin.user_id)
    .bind(price_cents)
    .bind(currency)
    .bind(is_free)
    .bind(is_included_in_subscription)
    .bind(sort_order)
    .bind(published)
    .bind(published_at)
    .bind(estimated_duration_minutes)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(course))
}

async fn admin_get_course(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<CourseWithModules>> {
    let course = sqlx::query_as::<_, Course>(
        r#"
        SELECT id, title, slug, description, short_description, thumbnail_url,
               trailer_video_url, difficulty, instructor_id, price_cents, currency,
               is_free, is_included_in_subscription, sort_order, published,
               published_at, estimated_duration_minutes, created_at, updated_at
        FROM courses
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Course not found".to_string()))?;

    let modules = sqlx::query_as::<_, CourseModule>(
        r#"
        SELECT id, course_id, title, description, sort_order, created_at, updated_at
        FROM course_modules
        WHERE course_id = $1
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let lessons = sqlx::query_as::<_, CourseLesson>(
        r#"
        SELECT cl.id, cl.module_id, cl.title, cl.slug, cl.description,
               cl.content, cl.content_json, cl.video_url, cl.video_duration_seconds,
               cl.sort_order, cl.is_preview, cl.created_at, cl.updated_at
        FROM course_lessons cl
        JOIN course_modules cm ON cm.id = cl.module_id
        WHERE cm.course_id = $1
        ORDER BY cl.sort_order ASC, cl.created_at ASC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let total_lessons = lessons.len() as i64;
    let total_duration_seconds: i64 = lessons
        .iter()
        .filter_map(|l| l.video_duration_seconds)
        .map(|d| d as i64)
        .sum();

    let mut modules_with_lessons: Vec<ModuleWithLessons> = modules
        .into_iter()
        .map(|m| ModuleWithLessons {
            module: m,
            lessons: Vec::new(),
        })
        .collect();

    for lesson in lessons {
        if let Some(mwl) = modules_with_lessons
            .iter_mut()
            .find(|mwl| mwl.module.id == lesson.module_id)
        {
            mwl.lessons.push(lesson);
        }
    }

    Ok(Json(CourseWithModules {
        course,
        modules: modules_with_lessons,
        total_lessons,
        total_duration_seconds,
    }))
}

async fn update_course(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCourseRequest>,
) -> AppResult<Json<Course>> {
    let _existing = sqlx::query_scalar::<_, Uuid>("SELECT id FROM courses WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Course not found".to_string()))?;

    let slug = req.slug.as_deref().map(slugify);

    let course = sqlx::query_as::<_, Course>(
        r#"
        UPDATE courses SET
            title = COALESCE($2, title),
            slug = COALESCE($3, slug),
            description = COALESCE($4, description),
            short_description = COALESCE($5, short_description),
            thumbnail_url = COALESCE($6, thumbnail_url),
            trailer_video_url = COALESCE($7, trailer_video_url),
            difficulty = COALESCE($8, difficulty),
            price_cents = COALESCE($9, price_cents),
            currency = COALESCE($10, currency),
            is_free = COALESCE($11, is_free),
            is_included_in_subscription = COALESCE($12, is_included_in_subscription),
            sort_order = COALESCE($13, sort_order),
            published = COALESCE($14, published),
            estimated_duration_minutes = COALESCE($15, estimated_duration_minutes),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, title, slug, description, short_description, thumbnail_url,
                  trailer_video_url, difficulty, instructor_id, price_cents, currency,
                  is_free, is_included_in_subscription, sort_order, published,
                  published_at, estimated_duration_minutes, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&req.title)
    .bind(&slug)
    .bind(&req.description)
    .bind(&req.short_description)
    .bind(&req.thumbnail_url)
    .bind(&req.trailer_video_url)
    .bind(&req.difficulty)
    .bind(req.price_cents)
    .bind(&req.currency)
    .bind(req.is_free)
    .bind(req.is_included_in_subscription)
    .bind(req.sort_order)
    .bind(req.published)
    .bind(req.estimated_duration_minutes)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(course))
}

async fn delete_course(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = sqlx::query("DELETE FROM courses WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Course not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn toggle_publish(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Course>> {
    let course = sqlx::query_as::<_, Course>(
        r#"
        UPDATE courses SET
            published = NOT published,
            published_at = CASE
                WHEN NOT published THEN NOW()
                ELSE published_at
            END,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, title, slug, description, short_description, thumbnail_url,
                  trailer_video_url, difficulty, instructor_id, price_cents, currency,
                  is_free, is_included_in_subscription, sort_order, published,
                  published_at, estimated_duration_minutes, created_at, updated_at
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Course not found".to_string()))?;

    Ok(Json(course))
}

// ── Module Handlers ──────────────────────────────────────────────────────

async fn create_module(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(course_id): Path<Uuid>,
    Json(req): Json<CreateModuleRequest>,
) -> AppResult<Json<CourseModule>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Verify course exists
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM courses WHERE id = $1")
        .bind(course_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Course not found".to_string()))?;

    let sort_order = req.sort_order.unwrap_or(0);

    let module = sqlx::query_as::<_, CourseModule>(
        r#"
        INSERT INTO course_modules (course_id, title, description, sort_order)
        VALUES ($1, $2, $3, $4)
        RETURNING id, course_id, title, description, sort_order, created_at, updated_at
        "#,
    )
    .bind(course_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(sort_order)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(module))
}

async fn update_module(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path((course_id, module_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateModuleRequest>,
) -> AppResult<Json<CourseModule>> {
    let module = sqlx::query_as::<_, CourseModule>(
        r#"
        UPDATE course_modules SET
            title = COALESCE($3, title),
            description = COALESCE($4, description),
            sort_order = COALESCE($5, sort_order),
            updated_at = NOW()
        WHERE id = $2 AND course_id = $1
        RETURNING id, course_id, title, description, sort_order, created_at, updated_at
        "#,
    )
    .bind(course_id)
    .bind(module_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.sort_order)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Module not found".to_string()))?;

    Ok(Json(module))
}

async fn delete_module(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path((course_id, module_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = sqlx::query("DELETE FROM course_modules WHERE id = $1 AND course_id = $2")
        .bind(module_id)
        .bind(course_id)
        .execute(&state.db)
        .await?
        .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Module not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ── Lesson Handlers ──────────────────────────────────────────────────────

async fn create_lesson(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path((course_id, module_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateLessonRequest>,
) -> AppResult<Json<CourseLesson>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Verify module belongs to course
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM course_modules WHERE id = $1 AND course_id = $2")
        .bind(module_id)
        .bind(course_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Module not found".to_string()))?;

    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.title));
    let content = req.content.as_deref().unwrap_or("");
    let sort_order = req.sort_order.unwrap_or(0);
    let is_preview = req.is_preview.unwrap_or(false);

    let lesson = sqlx::query_as::<_, CourseLesson>(
        r#"
        INSERT INTO course_lessons
            (module_id, title, slug, description, content, content_json,
             video_url, video_duration_seconds, sort_order, is_preview)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, module_id, title, slug, description, content, content_json,
                  video_url, video_duration_seconds, sort_order, is_preview,
                  created_at, updated_at
        "#,
    )
    .bind(module_id)
    .bind(&req.title)
    .bind(&slug)
    .bind(&req.description)
    .bind(content)
    .bind(&req.content_json)
    .bind(&req.video_url)
    .bind(req.video_duration_seconds)
    .bind(sort_order)
    .bind(is_preview)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(lesson))
}

async fn update_lesson(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(lesson_id): Path<Uuid>,
    Json(req): Json<UpdateLessonRequest>,
) -> AppResult<Json<CourseLesson>> {
    let slug = req.slug.as_deref().map(slugify);

    let lesson = sqlx::query_as::<_, CourseLesson>(
        r#"
        UPDATE course_lessons SET
            title = COALESCE($2, title),
            slug = COALESCE($3, slug),
            description = COALESCE($4, description),
            content = COALESCE($5, content),
            content_json = COALESCE($6, content_json),
            video_url = COALESCE($7, video_url),
            video_duration_seconds = COALESCE($8, video_duration_seconds),
            sort_order = COALESCE($9, sort_order),
            is_preview = COALESCE($10, is_preview),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, module_id, title, slug, description, content, content_json,
                  video_url, video_duration_seconds, sort_order, is_preview,
                  created_at, updated_at
        "#,
    )
    .bind(lesson_id)
    .bind(&req.title)
    .bind(&slug)
    .bind(&req.description)
    .bind(&req.content)
    .bind(&req.content_json)
    .bind(&req.video_url)
    .bind(req.video_duration_seconds)
    .bind(req.sort_order)
    .bind(req.is_preview)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Lesson not found".to_string()))?;

    Ok(Json(lesson))
}

async fn delete_lesson(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(lesson_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = sqlx::query("DELETE FROM course_lessons WHERE id = $1")
        .bind(lesson_id)
        .execute(&state.db)
        .await?
        .rows_affected();

    if rows == 0 {
        return Err(AppError::NotFound("Lesson not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ── Public Handlers ──────────────────────────────────────────────────────

async fn public_list_courses(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<CourseListItem>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM courses WHERE published = true")
        .fetch_one(&state.db)
        .await?;

    let courses = sqlx::query_as::<_, CourseListItem>(
        r#"
        SELECT c.id, c.title, c.slug, c.short_description, c.thumbnail_url,
               c.difficulty, u.name AS instructor_name, c.price_cents,
               c.is_free, c.is_included_in_subscription, c.published,
               c.estimated_duration_minutes,
               COUNT(cl.id)::bigint AS total_lessons,
               c.created_at
        FROM courses c
        JOIN users u ON u.id = c.instructor_id
        LEFT JOIN course_modules cm ON cm.course_id = c.id
        LEFT JOIN course_lessons cl ON cl.module_id = cm.id
        WHERE c.published = true
        GROUP BY c.id, u.name
        ORDER BY c.sort_order ASC, c.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total_pages = (total + per_page - 1) / per_page;

    Ok(Json(PaginatedResponse {
        data: courses,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn public_get_course(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<CourseWithModules>> {
    let course = sqlx::query_as::<_, Course>(
        r#"
        SELECT id, title, slug, description, short_description, thumbnail_url,
               trailer_video_url, difficulty, instructor_id, price_cents, currency,
               is_free, is_included_in_subscription, sort_order, published,
               published_at, estimated_duration_minutes, created_at, updated_at
        FROM courses
        WHERE slug = $1 AND published = true
        "#,
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Course not found".to_string()))?;

    let modules = sqlx::query_as::<_, CourseModule>(
        r#"
        SELECT id, course_id, title, description, sort_order, created_at, updated_at
        FROM course_modules
        WHERE course_id = $1
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .bind(course.id)
    .fetch_all(&state.db)
    .await?;

    let lessons = sqlx::query_as::<_, CourseLesson>(
        r#"
        SELECT cl.id, cl.module_id, cl.title, cl.slug, cl.description,
               cl.content, cl.content_json, cl.video_url, cl.video_duration_seconds,
               cl.sort_order, cl.is_preview, cl.created_at, cl.updated_at
        FROM course_lessons cl
        JOIN course_modules cm ON cm.id = cl.module_id
        WHERE cm.course_id = $1
        ORDER BY cl.sort_order ASC, cl.created_at ASC
        "#,
    )
    .bind(course.id)
    .fetch_all(&state.db)
    .await?;

    let total_lessons = lessons.len() as i64;
    let total_duration_seconds: i64 = lessons
        .iter()
        .filter_map(|l| l.video_duration_seconds)
        .map(|d| d as i64)
        .sum();

    let mut modules_with_lessons: Vec<ModuleWithLessons> = modules
        .into_iter()
        .map(|m| ModuleWithLessons {
            module: m,
            lessons: Vec::new(),
        })
        .collect();

    for lesson in lessons {
        if let Some(mwl) = modules_with_lessons
            .iter_mut()
            .find(|mwl| mwl.module.id == lesson.module_id)
        {
            mwl.lessons.push(lesson);
        }
    }

    Ok(Json(CourseWithModules {
        course,
        modules: modules_with_lessons,
        total_lessons,
        total_duration_seconds,
    }))
}

// ── Member Handlers ──────────────────────────────────────────────────────

async fn enroll_course(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(course_id): Path<Uuid>,
) -> AppResult<Json<CourseEnrollment>> {
    // Verify course exists and is published
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM courses WHERE id = $1 AND published = true")
        .bind(course_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Course not found".to_string()))?;

    let enrollment = sqlx::query_as::<_, CourseEnrollment>(
        r#"
        INSERT INTO course_enrollments (user_id, course_id, progress)
        VALUES ($1, $2, 0)
        ON CONFLICT (user_id, course_id) DO UPDATE SET user_id = EXCLUDED.user_id
        RETURNING id, user_id, course_id, progress, enrolled_at, completed_at
        "#,
    )
    .bind(auth.user_id)
    .bind(course_id.to_string())
    .fetch_one(&state.db)
    .await?;

    Ok(Json(enrollment))
}

#[derive(serde::Serialize)]
struct CourseProgressResponse {
    enrollment: CourseEnrollment,
    lesson_progress: Vec<LessonProgress>,
}

async fn get_course_progress(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(course_id): Path<Uuid>,
) -> AppResult<Json<CourseProgressResponse>> {
    let enrollment = sqlx::query_as::<_, CourseEnrollment>(
        r#"
        SELECT id, user_id, course_id, progress, enrolled_at, completed_at
        FROM course_enrollments
        WHERE user_id = $1 AND course_id = $2
        "#,
    )
    .bind(auth.user_id)
    .bind(course_id.to_string())
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Enrollment not found".to_string()))?;

    let lesson_progress = sqlx::query_as::<_, LessonProgress>(
        r#"
        SELECT lp.id, lp.user_id, lp.lesson_id, lp.completed, lp.progress_seconds,
               lp.completed_at, lp.last_accessed_at
        FROM lesson_progress lp
        JOIN course_lessons cl ON cl.id = lp.lesson_id
        JOIN course_modules cm ON cm.id = cl.module_id
        WHERE lp.user_id = $1 AND cm.course_id = $2
        "#,
    )
    .bind(auth.user_id)
    .bind(course_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(CourseProgressResponse {
        enrollment,
        lesson_progress,
    }))
}

async fn update_lesson_progress(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(lesson_id): Path<Uuid>,
    Json(req): Json<UpdateLessonProgressRequest>,
) -> AppResult<Json<LessonProgress>> {
    // Verify lesson exists
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM course_lessons WHERE id = $1")
        .bind(lesson_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Lesson not found".to_string()))?;

    let completed = req.completed.unwrap_or(false);
    let progress_seconds = req.progress_seconds.unwrap_or(0);
    let completed_at: Option<chrono::DateTime<Utc>> =
        if completed { Some(Utc::now()) } else { None };

    let progress = sqlx::query_as::<_, LessonProgress>(
        r#"
        INSERT INTO lesson_progress (user_id, lesson_id, completed, progress_seconds, completed_at, last_accessed_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        ON CONFLICT (user_id, lesson_id) DO UPDATE SET
            completed = COALESCE($3, lesson_progress.completed),
            progress_seconds = COALESCE($4, lesson_progress.progress_seconds),
            completed_at = CASE
                WHEN $3 = true AND lesson_progress.completed_at IS NULL THEN NOW()
                ELSE lesson_progress.completed_at
            END,
            last_accessed_at = NOW()
        RETURNING id, user_id, lesson_id, completed, progress_seconds, completed_at, last_accessed_at
        "#,
    )
    .bind(auth.user_id)
    .bind(lesson_id)
    .bind(completed)
    .bind(progress_seconds)
    .bind(completed_at)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(progress))
}
```

#### `backend/src/handlers/member.rs`

```rust
// backend/src/handlers/member.rs
use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::AuthUser,
    models::*,
    stripe_api, AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // Profile
        .route("/profile", get(get_profile))
        .route("/profile", put(update_profile))
        // Subscription
        .route("/subscription", get(get_subscription))
        .route("/billing-portal", post(post_billing_portal))
        .route("/subscription/cancel", post(post_subscription_cancel))
        .route("/subscription/resume", post(post_subscription_resume))
        // Watchlists
        .route("/watchlists", get(list_watchlists))
        .route("/watchlists/{id}", get(get_watchlist))
        // Courses
        .route("/courses", get(get_enrollments))
        .route("/courses/{course_id}/progress", put(update_progress))
}

// ── Profile ─────────────────────────────────────────────────────────────

async fn get_profile(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<UserResponse>> {
    let user = db::find_user_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;
    Ok(Json(user.into()))
}

#[derive(serde::Deserialize)]
struct UpdateProfileRequest {
    name: Option<String>,
    avatar_url: Option<String>,
    bio: Option<String>,
    position: Option<String>,
    website_url: Option<String>,
    twitter_url: Option<String>,
    linkedin_url: Option<String>,
    youtube_url: Option<String>,
    instagram_url: Option<String>,
}

async fn update_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> AppResult<Json<UserResponse>> {
    let user = db::find_user_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    let name = req.name.as_deref().unwrap_or(&user.name);
    let avatar_url = req.avatar_url.as_deref().or(user.avatar_url.as_deref());
    let bio = req.bio.as_deref().or(user.bio.as_deref());
    let position = req.position.as_deref().or(user.position.as_deref());
    let website_url = req.website_url.as_deref().or(user.website_url.as_deref());
    let twitter_url = req.twitter_url.as_deref().or(user.twitter_url.as_deref());
    let linkedin_url = req.linkedin_url.as_deref().or(user.linkedin_url.as_deref());
    let youtube_url = req.youtube_url.as_deref().or(user.youtube_url.as_deref());
    let instagram_url = req
        .instagram_url
        .as_deref()
        .or(user.instagram_url.as_deref());

    let updated = sqlx::query_as::<_, crate::models::User>(
        r#"UPDATE users SET
            name = $1, avatar_url = $2, bio = $3, position = $4,
            website_url = $5, twitter_url = $6, linkedin_url = $7,
            youtube_url = $8, instagram_url = $9, updated_at = NOW()
           WHERE id = $10 RETURNING *"#,
    )
    .bind(name)
    .bind(avatar_url)
    .bind(bio)
    .bind(position)
    .bind(website_url)
    .bind(twitter_url)
    .bind(linkedin_url)
    .bind(youtube_url)
    .bind(instagram_url)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(updated.into()))
}

// ── Subscription ────────────────────────────────────────────────────────

async fn get_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<SubscriptionStatusResponse>> {
    let sub = db::find_subscription_by_user(&state.db, auth.user_id).await?;
    let is_active = sub
        .as_ref()
        .map(|s| s.status == SubscriptionStatus::Active || s.status == SubscriptionStatus::Trialing)
        .unwrap_or(false);

    Ok(Json(SubscriptionStatusResponse {
        subscription: sub,
        is_active,
    }))
}

async fn post_billing_portal(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BillingPortalRequest>,
) -> AppResult<Json<BillingPortalResponse>> {
    let sub = db::find_subscription_by_user(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No subscription on file".to_string()))?;

    let base = state.config.frontend_url.trim_end_matches('/');
    let return_url = req
        .return_url
        .unwrap_or_else(|| format!("{base}/dashboard/account"));

    let url =
        stripe_api::create_billing_portal_session(&state, &sub.stripe_customer_id, &return_url)
            .await?;

    Ok(Json(BillingPortalResponse { url }))
}

async fn post_subscription_cancel(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let sub = db::find_subscription_by_user(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No subscription on file".to_string()))?;

    stripe_api::set_subscription_cancel_at_period_end(&state, &sub.stripe_subscription_id, true)
        .await?;

    Ok(Json(
        serde_json::json!({ "ok": true, "cancel_at_period_end": true }),
    ))
}

async fn post_subscription_resume(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let sub = db::find_subscription_by_user(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No subscription on file".to_string()))?;

    stripe_api::set_subscription_cancel_at_period_end(&state, &sub.stripe_subscription_id, false)
        .await?;

    Ok(Json(
        serde_json::json!({ "ok": true, "cancel_at_period_end": false }),
    ))
}

// ── Watchlists ──────────────────────────────────────────────────────────

async fn list_watchlists(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<Watchlist>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let (watchlists, total) = db::list_watchlists(&state.db, offset, per_page, true).await?;
    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: watchlists,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn get_watchlist(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<WatchlistWithAlerts>> {
    let watchlist = db::get_watchlist(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Watchlist not found".to_string()))?;

    if !watchlist.published {
        return Err(AppError::NotFound("Watchlist not found".to_string()));
    }

    let alerts = db::get_alerts_for_watchlist(&state.db, id).await?;
    Ok(Json(WatchlistWithAlerts { watchlist, alerts }))
}

// ── Courses ─────────────────────────────────────────────────────────────

async fn get_enrollments(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Vec<CourseEnrollment>>> {
    let enrollments = db::get_user_enrollments(&state.db, auth.user_id).await?;
    Ok(Json(enrollments))
}

#[derive(serde::Deserialize)]
struct ProgressUpdate {
    progress: i32,
}

async fn update_progress(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(course_id): Path<String>,
    Json(req): Json<ProgressUpdate>,
) -> AppResult<Json<CourseEnrollment>> {
    let enrollment =
        db::update_course_progress(&state.db, auth.user_id, &course_id, req.progress).await?;
    Ok(Json(enrollment))
}
```

#### `backend/src/handlers/mod.rs`

```rust
// backend/src/handlers/mod.rs
pub mod admin;
pub mod analytics;
pub mod auth;
pub mod blog;
pub mod coupons;
pub mod courses;
pub mod member;
pub mod popups;
pub mod pricing;
pub mod webhooks;
```

#### `backend/src/handlers/popups.rs`

```rust
// backend/src/handlers/popups.rs
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    extractors::{AdminUser, OptionalAuthUser},
    models::*,
    AppState,
};

// ══════════════════════════════════════════════════════════════════════
// ROUTERS
// ══════════════════════════════════════════════════════════════════════

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/popups", get(admin_list_popups))
        .route("/popups", post(admin_create_popup))
        .route("/popups/{id}", get(admin_get_popup))
        .route("/popups/{id}", put(admin_update_popup))
        .route("/popups/{id}", delete(admin_delete_popup))
        .route("/popups/{id}/toggle", post(admin_toggle_popup))
        .route("/popups/{id}/duplicate", post(admin_duplicate_popup))
        .route("/popups/{id}/submissions", get(admin_list_submissions))
        .route("/popups/{id}/analytics", get(admin_get_analytics))
}

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/popups/active", get(public_active_popups))
        .route("/popups/event", post(public_track_event))
        .route("/popups/submit", post(public_submit_form))
}

// ══════════════════════════════════════════════════════════════════════
// REQUEST / RESPONSE TYPES
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ActivePopupsQuery {
    pub page: Option<String>,
    pub device: Option<String>,
    pub user_status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TrackEventRequest {
    pub popup_id: Uuid,
    pub event_type: String,
    pub session_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct PopupDetailResponse {
    #[serde(flatten)]
    pub popup: Popup,
    pub analytics: PopupAnalytics,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AnalyticsTimeBucket {
    pub bucket: DateTime<Utc>,
    pub impressions: i64,
    pub closes: i64,
    pub submits: i64,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub granularity: Option<String>,
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_popups(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<Popup>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let popups = sqlx::query_as::<_, Popup>(
        r#"
        SELECT id, name, popup_type, trigger_type, trigger_config, content_json,
               style_json, targeting_rules, display_frequency, frequency_config,
               success_message, redirect_url, is_active, starts_at, expires_at,
               priority, created_by, created_at, updated_at
        FROM popups
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM popups")
        .fetch_one(&state.db)
        .await?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: popups,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn admin_create_popup(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<CreatePopupRequest>,
) -> AppResult<Json<Popup>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let popup = sqlx::query_as::<_, Popup>(
        r#"
        INSERT INTO popups (
            name, popup_type, trigger_type, trigger_config, content_json,
            style_json, targeting_rules, display_frequency, frequency_config,
            success_message, redirect_url, is_active, starts_at, expires_at,
            priority, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        RETURNING id, name, popup_type, trigger_type, trigger_config, content_json,
                  style_json, targeting_rules, display_frequency, frequency_config,
                  success_message, redirect_url, is_active, starts_at, expires_at,
                  priority, created_by, created_at, updated_at
        "#,
    )
    .bind(&req.name)
    .bind(req.popup_type.as_deref().unwrap_or("modal"))
    .bind(req.trigger_type.as_deref().unwrap_or("time_delay"))
    .bind(
        req.trigger_config
            .as_ref()
            .unwrap_or(&serde_json::json!({"delay_ms": 3000})),
    )
    .bind(
        req.content_json
            .as_ref()
            .unwrap_or(&serde_json::json!({"elements": []})),
    )
    .bind(req.style_json.as_ref().unwrap_or(&serde_json::json!({
        "background": "#1a1a2e",
        "textColor": "#ffffff",
        "accentColor": "#0fa4af",
        "borderRadius": "16px",
        "maxWidth": "480px",
        "animation": "fade",
        "backdrop": true,
        "backdropColor": "rgba(0,0,0,0.6)"
    })))
    .bind(req.targeting_rules.as_ref().unwrap_or(&serde_json::json!({
        "pages": ["*"],
        "devices": ["desktop", "mobile", "tablet"],
        "userStatus": ["all"]
    })))
    .bind(
        req.display_frequency
            .as_deref()
            .unwrap_or("once_per_session"),
    )
    .bind(
        req.frequency_config
            .as_ref()
            .unwrap_or(&serde_json::json!({})),
    )
    .bind(&req.success_message)
    .bind(&req.redirect_url)
    .bind(req.is_active.unwrap_or(false))
    .bind(req.starts_at)
    .bind(req.expires_at)
    .bind(req.priority.unwrap_or(0))
    .bind(admin.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(popup))
}

async fn admin_get_popup(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PopupDetailResponse>> {
    let popup = sqlx::query_as::<_, Popup>(
        r#"
        SELECT id, name, popup_type, trigger_type, trigger_config, content_json,
               style_json, targeting_rules, display_frequency, frequency_config,
               success_message, redirect_url, is_active, starts_at, expires_at,
               priority, created_by, created_at, updated_at
        FROM popups
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Popup not found".to_string()))?;

    let analytics = build_popup_analytics(&state.db, &popup).await?;

    Ok(Json(PopupDetailResponse { popup, analytics }))
}

async fn admin_update_popup(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePopupRequest>,
) -> AppResult<Json<Popup>> {
    // Ensure popup exists
    let existing = sqlx::query_as::<_, Popup>("SELECT * FROM popups WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Popup not found".to_string()))?;

    let popup = sqlx::query_as::<_, Popup>(
        r#"
        UPDATE popups SET
            name = COALESCE($1, name),
            popup_type = COALESCE($2, popup_type),
            trigger_type = COALESCE($3, trigger_type),
            trigger_config = COALESCE($4, trigger_config),
            content_json = COALESCE($5, content_json),
            style_json = COALESCE($6, style_json),
            targeting_rules = COALESCE($7, targeting_rules),
            display_frequency = COALESCE($8, display_frequency),
            frequency_config = COALESCE($9, frequency_config),
            success_message = COALESCE($10, success_message),
            redirect_url = COALESCE($11, redirect_url),
            is_active = COALESCE($12, is_active),
            starts_at = COALESCE($13, starts_at),
            expires_at = COALESCE($14, expires_at),
            priority = COALESCE($15, priority),
            updated_at = NOW()
        WHERE id = $16
        RETURNING id, name, popup_type, trigger_type, trigger_config, content_json,
                  style_json, targeting_rules, display_frequency, frequency_config,
                  success_message, redirect_url, is_active, starts_at, expires_at,
                  priority, created_by, created_at, updated_at
        "#,
    )
    .bind(&req.name)
    .bind(&req.popup_type)
    .bind(&req.trigger_type)
    .bind(&req.trigger_config)
    .bind(&req.content_json)
    .bind(&req.style_json)
    .bind(&req.targeting_rules)
    .bind(&req.display_frequency)
    .bind(&req.frequency_config)
    .bind(&req.success_message)
    .bind(&req.redirect_url)
    .bind(req.is_active)
    .bind(req.starts_at)
    .bind(req.expires_at)
    .bind(req.priority)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    // Suppress unused variable warning — we read existing to confirm it exists
    let _ = existing;

    Ok(Json(popup))
}

async fn admin_delete_popup(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM popups WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Popup not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "message": "Popup deleted" })))
}

async fn admin_toggle_popup(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Popup>> {
    let popup = sqlx::query_as::<_, Popup>(
        r#"
        UPDATE popups SET
            is_active = NOT is_active,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, popup_type, trigger_type, trigger_config, content_json,
                  style_json, targeting_rules, display_frequency, frequency_config,
                  success_message, redirect_url, is_active, starts_at, expires_at,
                  priority, created_by, created_at, updated_at
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Popup not found".to_string()))?;

    Ok(Json(popup))
}

async fn admin_duplicate_popup(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Popup>> {
    let source = sqlx::query_as::<_, Popup>("SELECT * FROM popups WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Popup not found".to_string()))?;

    let new_name = format!("{} (Copy)", source.name);

    let popup = sqlx::query_as::<_, Popup>(
        r#"
        INSERT INTO popups (
            name, popup_type, trigger_type, trigger_config, content_json,
            style_json, targeting_rules, display_frequency, frequency_config,
            success_message, redirect_url, is_active, starts_at, expires_at,
            priority, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, FALSE, $12, $13, $14, $15)
        RETURNING id, name, popup_type, trigger_type, trigger_config, content_json,
                  style_json, targeting_rules, display_frequency, frequency_config,
                  success_message, redirect_url, is_active, starts_at, expires_at,
                  priority, created_by, created_at, updated_at
        "#,
    )
    .bind(&new_name)
    .bind(&source.popup_type)
    .bind(&source.trigger_type)
    .bind(&source.trigger_config)
    .bind(&source.content_json)
    .bind(&source.style_json)
    .bind(&source.targeting_rules)
    .bind(&source.display_frequency)
    .bind(&source.frequency_config)
    .bind(&source.success_message)
    .bind(&source.redirect_url)
    .bind(source.starts_at)
    .bind(source.expires_at)
    .bind(source.priority)
    .bind(admin.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(popup))
}

async fn admin_list_submissions(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<PopupSubmission>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    // Verify popup exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM popups WHERE id = $1)")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    if !exists {
        return Err(AppError::NotFound("Popup not found".to_string()));
    }

    let submissions = sqlx::query_as::<_, PopupSubmission>(
        r#"
        SELECT id, popup_id, user_id, session_id, form_data, ip_address,
               user_agent, page_url, submitted_at
        FROM popup_submissions
        WHERE popup_id = $1
        ORDER BY submitted_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM popup_submissions WHERE popup_id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: submissions,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn admin_get_analytics(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Query(query): Query<AnalyticsQuery>,
) -> AppResult<Json<serde_json::Value>> {
    // Verify popup exists
    let popup = sqlx::query_as::<_, Popup>("SELECT * FROM popups WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Popup not found".to_string()))?;

    let summary = build_popup_analytics(&state.db, &popup).await?;

    let from = query
        .from
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
    let to = query.to.unwrap_or_else(Utc::now);
    let granularity = query.granularity.as_deref().unwrap_or("day");

    let time_series = sqlx::query_as::<_, AnalyticsTimeBucket>(
        r#"
        SELECT
            time_bucket AS bucket,
            COALESCE(SUM(CASE WHEN event_type = 'impression' THEN 1 ELSE 0 END), 0) AS impressions,
            COALESCE(SUM(CASE WHEN event_type = 'close' THEN 1 ELSE 0 END), 0) AS closes,
            COALESCE(SUM(CASE WHEN event_type = 'submit' THEN 1 ELSE 0 END), 0) AS submits
        FROM (
            SELECT
                date_trunc($1, created_at) AS time_bucket,
                event_type
            FROM popup_events
            WHERE popup_id = $2
              AND created_at >= $3
              AND created_at <= $4
        ) sub
        GROUP BY time_bucket
        ORDER BY time_bucket ASC
        "#,
    )
    .bind(granularity_to_trunc(granularity))
    .bind(id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "summary": summary,
        "time_series": time_series,
        "from": from,
        "to": to,
        "granularity": granularity,
    })))
}

// ══════════════════════════════════════════════════════════════════════
// PUBLIC HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn public_active_popups(
    State(state): State<AppState>,
    Query(query): Query<ActivePopupsQuery>,
) -> AppResult<Json<Vec<Popup>>> {
    // Fetch all active popups within their date window
    let popups = sqlx::query_as::<_, Popup>(
        r#"
        SELECT id, name, popup_type, trigger_type, trigger_config, content_json,
               style_json, targeting_rules, display_frequency, frequency_config,
               success_message, redirect_url, is_active, starts_at, expires_at,
               priority, created_by, created_at, updated_at
        FROM popups
        WHERE is_active = TRUE
          AND (starts_at IS NULL OR starts_at <= NOW())
          AND (expires_at IS NULL OR expires_at >= NOW())
        ORDER BY priority DESC, created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let page_path = query.page.as_deref().unwrap_or("*");
    let device = query.device.as_deref().unwrap_or("desktop");
    let user_status = query.user_status.as_deref().unwrap_or("all");

    let filtered: Vec<Popup> = popups
        .into_iter()
        .filter(|popup| {
            matches_targeting_rules(&popup.targeting_rules, page_path, device, user_status)
        })
        .collect();

    Ok(Json(filtered))
}

async fn public_track_event(
    State(state): State<AppState>,
    Json(req): Json<TrackEventRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let event_type = req.event_type.to_lowercase();
    if !["impression", "close", "submit", "click"].contains(&event_type.as_str()) {
        return Err(AppError::BadRequest(
            "event_type must be one of: impression, close, submit, click".to_string(),
        ));
    }

    // Verify popup exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM popups WHERE id = $1)")
        .bind(req.popup_id)
        .fetch_one(&state.db)
        .await?;

    if !exists {
        return Err(AppError::NotFound("Popup not found".to_string()));
    }

    sqlx::query(
        r#"
        INSERT INTO popup_events (popup_id, event_type, session_id)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(req.popup_id)
    .bind(&event_type)
    .bind(req.session_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn public_submit_form(
    State(state): State<AppState>,
    opt: OptionalAuthUser,
    Json(req): Json<PopupSubmitRequest>,
) -> AppResult<Json<PopupSubmission>> {
    // Verify popup exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM popups WHERE id = $1)")
        .bind(req.popup_id)
        .fetch_one(&state.db)
        .await?;

    if !exists {
        return Err(AppError::NotFound("Popup not found".to_string()));
    }

    let submission = sqlx::query_as::<_, PopupSubmission>(
        r#"
        INSERT INTO popup_submissions (popup_id, user_id, session_id, form_data, page_url)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, popup_id, user_id, session_id, form_data, ip_address,
                  user_agent, page_url, submitted_at
        "#,
    )
    .bind(req.popup_id)
    .bind(opt.user_id)
    .bind(req.session_id)
    .bind(&req.form_data)
    .bind(&req.page_url)
    .fetch_one(&state.db)
    .await?;

    // Also record a submit event
    sqlx::query(
        r#"
        INSERT INTO popup_events (popup_id, event_type, session_id)
        VALUES ($1, 'submit', $2)
        "#,
    )
    .bind(req.popup_id)
    .bind(req.session_id)
    .execute(&state.db)
    .await?;

    Ok(Json(submission))
}

// ══════════════════════════════════════════════════════════════════════
// HELPERS
// ══════════════════════════════════════════════════════════════════════

async fn build_popup_analytics(pool: &sqlx::PgPool, popup: &Popup) -> AppResult<PopupAnalytics> {
    let impressions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM popup_events WHERE popup_id = $1 AND event_type = 'impression'",
    )
    .bind(popup.id)
    .fetch_one(pool)
    .await?;

    let closes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM popup_events WHERE popup_id = $1 AND event_type = 'close'",
    )
    .bind(popup.id)
    .fetch_one(pool)
    .await?;

    let submissions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM popup_events WHERE popup_id = $1 AND event_type = 'submit'",
    )
    .bind(popup.id)
    .fetch_one(pool)
    .await?;

    let conversion_rate = if impressions > 0 {
        (submissions as f64 / impressions as f64) * 100.0
    } else {
        0.0
    };

    Ok(PopupAnalytics {
        popup_id: popup.id,
        popup_name: popup.name.clone(),
        total_impressions: impressions,
        total_closes: closes,
        total_submissions: submissions,
        conversion_rate,
    })
}

/// Match a popup's targeting_rules JSON against the requested page, device, and user status.
fn matches_targeting_rules(
    rules: &serde_json::Value,
    page_path: &str,
    device: &str,
    user_status: &str,
) -> bool {
    // Check page patterns
    if let Some(pages) = rules.get("pages").and_then(|v| v.as_array()) {
        let page_match = pages.iter().any(|p| {
            if let Some(pattern) = p.as_str() {
                matches_page_pattern(pattern, page_path)
            } else {
                false
            }
        });
        if !page_match {
            return false;
        }
    }

    // Check device targeting
    if let Some(devices) = rules.get("devices").and_then(|v| v.as_array()) {
        let device_match = devices.iter().any(|d| {
            d.as_str()
                .map(|s| s.eq_ignore_ascii_case(device))
                .unwrap_or(false)
        });
        if !device_match {
            return false;
        }
    }

    // Check user status targeting
    if let Some(statuses) = rules.get("userStatus").and_then(|v| v.as_array()) {
        let status_match = statuses.iter().any(|s| {
            if let Some(st) = s.as_str() {
                st == "all" || st.eq_ignore_ascii_case(user_status)
            } else {
                false
            }
        });
        if !status_match {
            return false;
        }
    }

    true
}

/// Simple glob-like page pattern matching.
/// Supports:
///   "*"         -> matches everything
///   "/blog/*"   -> matches /blog/ and anything under it
///   "/pricing"  -> exact match
fn matches_page_pattern(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix);
    }

    pattern == path
}

/// Convert user-facing granularity to a Postgres date_trunc argument.
fn granularity_to_trunc(granularity: &str) -> &str {
    match granularity {
        "hour" => "hour",
        "day" => "day",
        "week" => "week",
        "month" => "month",
        _ => "day",
    }
}
```

#### `backend/src/handlers/pricing.rs`

```rust
// backend/src/handlers/pricing.rs
use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    extractors::AdminUser,
    models::*,
    AppState,
};

// ── Admin Pricing Router ──────────────────────────────────────────────

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/plans", get(admin_list_plans))
        .route("/plans", post(admin_create_plan))
        .route("/plans/{id}", get(admin_get_plan))
        .route("/plans/{id}", put(admin_update_plan))
        .route("/plans/{id}", axum::routing::delete(admin_delete_plan))
        .route("/plans/{id}/toggle", post(admin_toggle_plan))
        .route("/plans/{id}/history", get(admin_plan_history))
}

// ── Public Pricing Router ─────────────────────────────────────────────

pub fn public_router() -> Router<AppState> {
    Router::new().route("/plans", get(public_list_plans))
}

// ── Helpers ───────────────────────────────────────────────────────────

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[derive(serde::Serialize)]
struct PlanWithHistory {
    #[serde(flatten)]
    plan: PricingPlan,
    change_history: Vec<PricingChangeLog>,
}

// ── Admin Handlers ────────────────────────────────────────────────────

async fn admin_list_plans(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<PricingPlan>>> {
    let plans = sqlx::query_as::<_, PricingPlan>(
        r#"
        SELECT id, name, slug, description, stripe_price_id, stripe_product_id,
               amount_cents, currency, interval, interval_count, trial_days,
               features, highlight_text, is_popular, is_active, sort_order,
               created_at, updated_at
        FROM pricing_plans
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(plans))
}

async fn admin_create_plan(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreatePricingPlanRequest>,
) -> AppResult<Json<PricingPlan>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.name));
    let currency = req.currency.as_deref().unwrap_or("usd");
    let interval = req.interval.as_deref().unwrap_or("month");
    let interval_count = req.interval_count.unwrap_or(1);
    let trial_days = req.trial_days.unwrap_or(0);
    let features = req.features.clone().unwrap_or(serde_json::json!([]));
    let is_popular = req.is_popular.unwrap_or(false);
    let is_active = req.is_active.unwrap_or(true);
    let sort_order = req.sort_order.unwrap_or(0);

    let plan = sqlx::query_as::<_, PricingPlan>(
        r#"
        INSERT INTO pricing_plans
            (name, slug, description, stripe_price_id, stripe_product_id,
             amount_cents, currency, interval, interval_count, trial_days,
             features, highlight_text, is_popular, is_active, sort_order)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        RETURNING id, name, slug, description, stripe_price_id, stripe_product_id,
                  amount_cents, currency, interval, interval_count, trial_days,
                  features, highlight_text, is_popular, is_active, sort_order,
                  created_at, updated_at
        "#,
    )
    .bind(&req.name)
    .bind(&slug)
    .bind(&req.description)
    .bind(&req.stripe_price_id)
    .bind(&req.stripe_product_id)
    .bind(req.amount_cents)
    .bind(currency)
    .bind(interval)
    .bind(interval_count)
    .bind(trial_days)
    .bind(&features)
    .bind(&req.highlight_text)
    .bind(is_popular)
    .bind(is_active)
    .bind(sort_order)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(plan))
}

async fn admin_get_plan(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PlanWithHistory>> {
    let plan = sqlx::query_as::<_, PricingPlan>(
        r#"
        SELECT id, name, slug, description, stripe_price_id, stripe_product_id,
               amount_cents, currency, interval, interval_count, trial_days,
               features, highlight_text, is_popular, is_active, sort_order,
               created_at, updated_at
        FROM pricing_plans
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Pricing plan not found".to_string()))?;

    let change_history = sqlx::query_as::<_, PricingChangeLog>(
        r#"
        SELECT id, plan_id, field_changed, old_value, new_value, changed_by, changed_at
        FROM pricing_change_log
        WHERE plan_id = $1
        ORDER BY changed_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(PlanWithHistory {
        plan,
        change_history,
    }))
}

async fn admin_update_plan(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePricingPlanRequest>,
) -> AppResult<Json<PricingPlan>> {
    let existing = sqlx::query_as::<_, PricingPlan>(
        r#"
        SELECT id, name, slug, description, stripe_price_id, stripe_product_id,
               amount_cents, currency, interval, interval_count, trial_days,
               features, highlight_text, is_popular, is_active, sort_order,
               created_at, updated_at
        FROM pricing_plans
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Pricing plan not found".to_string()))?;

    // Collect changes for the change log
    let mut changes: Vec<(&str, String, String)> = Vec::new();

    if let Some(ref name) = req.name {
        if *name != existing.name {
            changes.push(("name", existing.name.clone(), name.clone()));
        }
    }
    if let Some(ref slug) = req.slug {
        let new_slug = slugify(slug);
        if new_slug != existing.slug {
            changes.push(("slug", existing.slug.clone(), new_slug));
        }
    }
    if let Some(ref description) = req.description {
        let old = existing.description.clone().unwrap_or_default();
        if *description != old {
            changes.push(("description", old, description.clone()));
        }
    }
    if let Some(ref stripe_price_id) = req.stripe_price_id {
        let old = existing.stripe_price_id.clone().unwrap_or_default();
        if *stripe_price_id != old {
            changes.push(("stripe_price_id", old, stripe_price_id.clone()));
        }
    }
    if let Some(ref stripe_product_id) = req.stripe_product_id {
        let old = existing.stripe_product_id.clone().unwrap_or_default();
        if *stripe_product_id != old {
            changes.push(("stripe_product_id", old, stripe_product_id.clone()));
        }
    }
    if let Some(amount_cents) = req.amount_cents {
        if amount_cents != existing.amount_cents {
            changes.push((
                "amount_cents",
                existing.amount_cents.to_string(),
                amount_cents.to_string(),
            ));
        }
    }
    if let Some(ref currency) = req.currency {
        if *currency != existing.currency {
            changes.push(("currency", existing.currency.clone(), currency.clone()));
        }
    }
    if let Some(ref interval) = req.interval {
        if *interval != existing.interval {
            changes.push(("interval", existing.interval.clone(), interval.clone()));
        }
    }
    if let Some(interval_count) = req.interval_count {
        if interval_count != existing.interval_count {
            changes.push((
                "interval_count",
                existing.interval_count.to_string(),
                interval_count.to_string(),
            ));
        }
    }
    if let Some(trial_days) = req.trial_days {
        if trial_days != existing.trial_days {
            changes.push((
                "trial_days",
                existing.trial_days.to_string(),
                trial_days.to_string(),
            ));
        }
    }
    if let Some(ref features) = req.features {
        let old_str = existing.features.to_string();
        let new_str = features.to_string();
        if old_str != new_str {
            changes.push(("features", old_str, new_str));
        }
    }
    if let Some(ref highlight_text) = req.highlight_text {
        let old = existing.highlight_text.clone().unwrap_or_default();
        if *highlight_text != old {
            changes.push(("highlight_text", old, highlight_text.clone()));
        }
    }
    if let Some(is_popular) = req.is_popular {
        if is_popular != existing.is_popular {
            changes.push((
                "is_popular",
                existing.is_popular.to_string(),
                is_popular.to_string(),
            ));
        }
    }
    if let Some(is_active) = req.is_active {
        if is_active != existing.is_active {
            changes.push((
                "is_active",
                existing.is_active.to_string(),
                is_active.to_string(),
            ));
        }
    }
    if let Some(sort_order) = req.sort_order {
        if sort_order != existing.sort_order {
            changes.push((
                "sort_order",
                existing.sort_order.to_string(),
                sort_order.to_string(),
            ));
        }
    }

    // Apply the update
    let name = req.name.as_deref().unwrap_or(&existing.name);
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or(existing.slug.clone());
    let description = req
        .description
        .as_deref()
        .or(existing.description.as_deref());
    let stripe_price_id = req
        .stripe_price_id
        .as_deref()
        .or(existing.stripe_price_id.as_deref());
    let stripe_product_id = req
        .stripe_product_id
        .as_deref()
        .or(existing.stripe_product_id.as_deref());
    let amount_cents = req.amount_cents.unwrap_or(existing.amount_cents);
    let currency = req.currency.as_deref().unwrap_or(&existing.currency);
    let interval = req.interval.as_deref().unwrap_or(&existing.interval);
    let interval_count = req.interval_count.unwrap_or(existing.interval_count);
    let trial_days = req.trial_days.unwrap_or(existing.trial_days);
    let features = req.features.clone().unwrap_or(existing.features.clone());
    let highlight_text = req
        .highlight_text
        .as_deref()
        .or(existing.highlight_text.as_deref());
    let is_popular = req.is_popular.unwrap_or(existing.is_popular);
    let is_active = req.is_active.unwrap_or(existing.is_active);
    let sort_order = req.sort_order.unwrap_or(existing.sort_order);

    let updated = sqlx::query_as::<_, PricingPlan>(
        r#"
        UPDATE pricing_plans
        SET name = $1, slug = $2, description = $3, stripe_price_id = $4,
            stripe_product_id = $5, amount_cents = $6, currency = $7,
            interval = $8, interval_count = $9, trial_days = $10,
            features = $11, highlight_text = $12, is_popular = $13,
            is_active = $14, sort_order = $15, updated_at = NOW()
        WHERE id = $16
        RETURNING id, name, slug, description, stripe_price_id, stripe_product_id,
                  amount_cents, currency, interval, interval_count, trial_days,
                  features, highlight_text, is_popular, is_active, sort_order,
                  created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(&slug)
    .bind(description)
    .bind(stripe_price_id)
    .bind(stripe_product_id)
    .bind(amount_cents)
    .bind(currency)
    .bind(interval)
    .bind(interval_count)
    .bind(trial_days)
    .bind(&features)
    .bind(highlight_text)
    .bind(is_popular)
    .bind(is_active)
    .bind(sort_order)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    // Insert change log entries
    for (field, old_val, new_val) in &changes {
        sqlx::query(
            r#"
            INSERT INTO pricing_change_log (plan_id, field_changed, old_value, new_value, changed_by)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(id)
        .bind(field)
        .bind(old_val)
        .bind(new_val)
        .bind(admin.user_id)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(updated))
}

async fn admin_delete_plan(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query(
        r#"
        UPDATE pricing_plans
        SET is_active = false, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Pricing plan not found".to_string()));
    }

    Ok(Json(
        serde_json::json!({ "message": "Pricing plan deactivated" }),
    ))
}

async fn admin_toggle_plan(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PricingPlan>> {
    let plan = sqlx::query_as::<_, PricingPlan>(
        r#"
        UPDATE pricing_plans
        SET is_active = NOT is_active, updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, slug, description, stripe_price_id, stripe_product_id,
                  amount_cents, currency, interval, interval_count, trial_days,
                  features, highlight_text, is_popular, is_active, sort_order,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Pricing plan not found".to_string()))?;

    Ok(Json(plan))
}

async fn admin_plan_history(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<PricingChangeLog>>> {
    // Verify plan exists
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM pricing_plans WHERE id = $1)")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    if !exists {
        return Err(AppError::NotFound("Pricing plan not found".to_string()));
    }

    let history = sqlx::query_as::<_, PricingChangeLog>(
        r#"
        SELECT id, plan_id, field_changed, old_value, new_value, changed_by, changed_at
        FROM pricing_change_log
        WHERE plan_id = $1
        ORDER BY changed_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(history))
}

// ── Public Handlers ───────────────────────────────────────────────────

async fn public_list_plans(State(state): State<AppState>) -> AppResult<Json<Vec<PricingPlan>>> {
    let plans = sqlx::query_as::<_, PricingPlan>(
        r#"
        SELECT id, name, slug, description, stripe_price_id, stripe_product_id,
               amount_cents, currency, interval, interval_count, trial_days,
               features, highlight_text, is_popular, is_active, sort_order,
               created_at, updated_at
        FROM pricing_plans
        WHERE is_active = true
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(plans))
}
```

#### `backend/src/handlers/webhooks.rs`

```rust
// backend/src/handlers/webhooks.rs
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;

use crate::{db, models::*, AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/stripe", post(stripe_webhook))
}

async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let signature = match headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(sig) => sig.to_string(),
        None => return StatusCode::BAD_REQUEST,
    };

    if state.config.stripe_webhook_secret.is_empty() {
        tracing::warn!("Stripe webhook secret not configured");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Verify webhook signature before parsing the event payload.
    let payload = match std::str::from_utf8(&body) {
        Ok(p) => p,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    if !verify_stripe_signature(payload, &signature, &state.config.stripe_webhook_secret) {
        tracing::warn!("Rejected Stripe webhook due to invalid signature");
        return StatusCode::UNAUTHORIZED;
    }

    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(e) => e,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let Some(event_id) = event.get("id").and_then(|v| v.as_str()) else {
        tracing::warn!("Stripe event missing id");
        return StatusCode::BAD_REQUEST;
    };
    let event_type = event
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    match db::try_claim_stripe_webhook_event(&state.db, event_id, event_type).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::debug!(event_id, "Duplicate Stripe webhook event — acknowledging");
            return StatusCode::OK;
        }
        Err(e) => {
            tracing::error!("Webhook idempotency insert failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    // ~1% of webhooks: prune old idempotency rows so the table stays bounded.
    if rand::thread_rng().gen_bool(0.01) {
        match db::cleanup_old_stripe_webhook_events(&state.db).await {
            Ok(n) if n > 0 => tracing::debug!("Cleaned up {n} old processed_webhook_events rows"),
            Err(e) => tracing::warn!("Webhook idempotency cleanup failed: {e}"),
            _ => {}
        }
    }

    tracing::info!("Stripe webhook received: {event_type} ({event_id})");

    match event_type {
        "customer.subscription.created" | "customer.subscription.updated" => {
            if let Err(e) = handle_subscription_update(&state, &event).await {
                tracing::error!("Failed to handle subscription update: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        "customer.subscription.deleted" => {
            if let Err(e) = handle_subscription_deleted(&state, &event).await {
                tracing::error!("Failed to handle subscription deletion: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        "checkout.session.completed" => {
            if let Err(e) = handle_checkout_completed(&state, &event).await {
                tracing::error!("Failed to handle checkout: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        _ => {
            tracing::debug!("Unhandled webhook event: {event_type}");
        }
    }

    StatusCode::OK
}

fn verify_stripe_signature(payload: &str, signature_header: &str, secret: &str) -> bool {
    type HmacSha256 = Hmac<Sha256>;

    let mut timestamp: Option<i64> = None;
    let mut signatures: Vec<&str> = Vec::new();
    for part in signature_header.split(',') {
        let mut kv = part.splitn(2, '=');
        let key = kv.next().unwrap_or_default().trim();
        let value = kv.next().unwrap_or_default().trim();
        match key {
            "t" => timestamp = value.parse::<i64>().ok(),
            "v1" if !value.is_empty() => signatures.push(value),
            _ => {}
        }
    }

    let Some(ts) = timestamp else {
        return false;
    };
    if signatures.is_empty() {
        return false;
    }

    let now = chrono::Utc::now().timestamp();
    // Stripe recommends a 5 minute tolerance to reduce replay risk.
    if (now - ts).abs() > 300 {
        return false;
    }

    let signed_payload = format!("{ts}.{payload}");
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(signed_payload.as_bytes());
    let computed = mac.finalize().into_bytes();
    let computed_hex = computed
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    signatures
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&computed_hex))
}

async fn handle_subscription_update(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let sub = &event["data"]["object"];
    let customer_id = sub["customer"].as_str().unwrap_or_default();
    let sub_id = sub["id"].as_str().unwrap_or_default();
    let status_str = sub["status"].as_str().unwrap_or("active");

    let status = match status_str {
        "active" => SubscriptionStatus::Active,
        "canceled" => SubscriptionStatus::Canceled,
        "past_due" => SubscriptionStatus::PastDue,
        "trialing" => SubscriptionStatus::Trialing,
        "unpaid" => SubscriptionStatus::Unpaid,
        _ => SubscriptionStatus::Active,
    };

    // Determine plan from price interval
    let plan = if let Some(items) = sub["items"]["data"].as_array() {
        if let Some(first) = items.first() {
            match first["price"]["recurring"]["interval"].as_str() {
                Some("year") => SubscriptionPlan::Annual,
                _ => SubscriptionPlan::Monthly,
            }
        } else {
            SubscriptionPlan::Monthly
        }
    } else {
        SubscriptionPlan::Monthly
    };

    let period_start =
        chrono::DateTime::from_timestamp(sub["current_period_start"].as_i64().unwrap_or(0), 0)
            .unwrap_or_default();

    let period_end =
        chrono::DateTime::from_timestamp(sub["current_period_end"].as_i64().unwrap_or(0), 0)
            .unwrap_or_default();

    // Find user by stripe customer id
    if let Some(user) = db::find_user_by_stripe_customer(&state.db, customer_id).await? {
        db::upsert_subscription(
            &state.db,
            user.id,
            customer_id,
            sub_id,
            &plan,
            &status,
            period_start,
            period_end,
        )
        .await?;
    }

    Ok(())
}

async fn handle_subscription_deleted(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let sub = &event["data"]["object"];
    let sub_id = sub["id"].as_str().unwrap_or_default();

    if let Some(existing) = db::find_subscription_by_stripe_id(&state.db, sub_id).await? {
        let customer_id = &existing.stripe_customer_id;
        db::upsert_subscription(
            &state.db,
            existing.user_id,
            customer_id,
            sub_id,
            &existing.plan,
            &SubscriptionStatus::Canceled,
            existing.current_period_start,
            existing.current_period_end,
        )
        .await?;
    }

    Ok(())
}

async fn handle_checkout_completed(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = &event["data"]["object"];
    let customer_email = session["customer_details"]["email"]
        .as_str()
        .unwrap_or_default();
    let customer_id = session["customer"].as_str().unwrap_or_default();
    let sub_id = session["subscription"].as_str().unwrap_or_default();

    tracing::info!(
        "Checkout completed for {customer_email}, customer: {customer_id}, sub: {sub_id}"
    );

    // If user exists, link their subscription
    if let Some(user) = db::find_user_by_email(&state.db, customer_email).await? {
        let now = chrono::Utc::now();
        db::upsert_subscription(
            &state.db,
            user.id,
            customer_id,
            sub_id,
            &SubscriptionPlan::Monthly, // Will be corrected by subscription.updated event
            &SubscriptionStatus::Active,
            now,
            now + chrono::Duration::days(30),
        )
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::verify_stripe_signature;
    use chrono::Utc;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    fn make_signature(secret: &str, payload: &str, timestamp: i64) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("valid hmac key");
        mac.update(format!("{timestamp}.{payload}").as_bytes());
        let digest = mac.finalize().into_bytes();
        let hex = digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        format!("t={timestamp},v1={hex}")
    }

    #[test]
    fn accepts_valid_signature() {
        let secret = "whsec_test_secret";
        let payload = r#"{"type":"checkout.session.completed"}"#;
        let timestamp = Utc::now().timestamp();
        let header = make_signature(secret, payload, timestamp);
        assert!(verify_stripe_signature(payload, &header, secret));
    }

    #[test]
    fn rejects_tampered_payload() {
        let secret = "whsec_test_secret";
        let payload = r#"{"type":"checkout.session.completed"}"#;
        let timestamp = Utc::now().timestamp();
        let header = make_signature(secret, payload, timestamp);
        assert!(!verify_stripe_signature(
            r#"{"type":"customer.subscription.deleted"}"#,
            &header,
            secret
        ));
    }
}
```

### 2.6 Services, models, persistence, and configuration

#### `backend/src/services/mod.rs`

```rust
// backend/src/services/mod.rs
pub mod storage;

pub use storage::{MediaBackend, R2Storage, StorageError};
```

#### `backend/src/services/storage.rs`

```rust
// backend/src/services/storage.rs
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use bytes::Bytes;
use std::env;

/// Cloudflare R2 (S3-compatible) storage.
#[derive(Clone)]
pub struct R2Storage {
    client: Client,
    bucket: String,
    public_base: String,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum StorageError {
    #[error("R2 upload failed: {0}")]
    Upload(String),
    #[error("R2 delete failed: {0}")]
    Delete(String),
    #[error("R2 configuration error: {0}")]
    Config(String),
}

impl R2Storage {
    /// Load R2 from environment. All of `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`,
    /// `R2_BUCKET_NAME`, `R2_PUBLIC_URL` must be set and non-empty.
    pub fn from_env() -> Result<Self, StorageError> {
        let account_id = env::var("R2_ACCOUNT_ID")
            .map_err(|_| StorageError::Config("R2_ACCOUNT_ID not set".into()))?;
        if account_id.trim().is_empty() {
            return Err(StorageError::Config("R2_ACCOUNT_ID is empty".into()));
        }
        let access_key = env::var("R2_ACCESS_KEY_ID")
            .map_err(|_| StorageError::Config("R2_ACCESS_KEY_ID not set".into()))?;
        let secret_key = env::var("R2_SECRET_ACCESS_KEY")
            .map_err(|_| StorageError::Config("R2_SECRET_ACCESS_KEY not set".into()))?;
        let bucket = env::var("R2_BUCKET_NAME")
            .map_err(|_| StorageError::Config("R2_BUCKET_NAME not set".into()))?;
        let public_url = env::var("R2_PUBLIC_URL")
            .map_err(|_| StorageError::Config("R2_PUBLIC_URL not set".into()))?;

        let endpoint = format!(
            "https://{}.r2.cloudflarestorage.com",
            account_id.trim()
        );
        let credentials = aws_sdk_s3::config::Credentials::new(
            access_key.trim(),
            secret_key.trim(),
            None,
            None,
            "r2",
        );

        let s3_config = aws_sdk_s3::config::Builder::new()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .endpoint_url(&endpoint)
            .credentials_provider(credentials)
            .region(aws_sdk_s3::config::Region::new("auto"))
            .force_path_style(true)
            .build();

        let client = Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket: bucket.trim().to_string(),
            public_base: public_url.trim_end_matches('/').to_string(),
        })
    }

    /// Upload bytes; returns public URL.
    pub async fn upload(
        &self,
        key: &str,
        data: Bytes,
        content_type: &str,
    ) -> Result<String, StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data))
            .content_type(content_type)
            .cache_control("public, max-age=31536000, immutable")
            .send()
            .await
            .map_err(|e| StorageError::Upload(e.to_string()))?;

        Ok(self.public_url_for_key(key))
    }

    pub async fn delete_object(&self, key: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::Delete(e.to_string()))?;
        Ok(())
    }

    /// `media/{year}/{month}/{uuid8}-{sanitized_name}`
    pub fn generate_key(original_filename: &str) -> String {
        let now = chrono::Utc::now();
        let sanitized = sanitize_filename::sanitize(original_filename);
        let unique: String = uuid::Uuid::new_v4().to_string().chars().take(8).collect();
        format!(
            "media/{}/{:02}/{}-{}",
            now.format("%Y"),
            now.format("%m"),
            unique,
            sanitized
        )
    }

    pub fn public_url_for_key(&self, key: &str) -> String {
        format!("{}/{}", self.public_base, key)
    }
}

/// Media persistence: local disk (dev) or Cloudflare R2 (production).
#[derive(Clone)]
pub enum MediaBackend {
    Local { upload_dir: String },
    R2(R2Storage),
}

impl MediaBackend {
    /// Prefer R2 when all variables are set; otherwise local `upload_dir`.
    pub fn resolve(upload_dir: String) -> Self {
        match R2Storage::from_env() {
            Ok(r2) => {
                tracing::info!("Media backend: R2 (bucket configured)");
                MediaBackend::R2(r2)
            }
            Err(e) => {
                tracing::warn!("R2 not configured ({}); using local uploads at {}", e, upload_dir);
                MediaBackend::Local { upload_dir }
            }
        }
    }

    pub fn is_r2(&self) -> bool {
        matches!(self, MediaBackend::R2(_))
    }

    pub fn upload_dir(&self) -> Option<&str> {
        match self {
            MediaBackend::Local { upload_dir } => Some(upload_dir.as_str()),
            MediaBackend::R2(_) => None,
        }
    }
}
```

#### `backend/src/models.rs`

```rust
// backend/src/models.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

// ── User ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub name: String,
    pub role: UserRole,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub position: Option<String>,
    pub website_url: Option<String>,
    pub twitter_url: Option<String>,
    pub linkedin_url: Option<String>,
    pub youtube_url: Option<String>,
    pub instagram_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    Member,
    Admin,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub position: Option<String>,
    pub website_url: Option<String>,
    pub twitter_url: Option<String>,
    pub linkedin_url: Option<String>,
    pub youtube_url: Option<String>,
    pub instagram_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            name: u.name,
            role: u.role,
            avatar_url: u.avatar_url,
            bio: u.bio,
            position: u.position,
            website_url: u.website_url,
            twitter_url: u.twitter_url,
            linkedin_url: u.linkedin_url,
            youtube_url: u.youtube_url,
            instagram_url: u.instagram_url,
            created_at: u.created_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

// ── Password Reset ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordResetToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub new_password: String,
}

// ── Subscription ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub stripe_customer_id: String,
    pub stripe_subscription_id: String,
    pub plan: SubscriptionPlan,
    pub status: SubscriptionStatus,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "subscription_plan", rename_all = "lowercase")]
pub enum SubscriptionPlan {
    Monthly,
    Annual,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "subscription_status", rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    Canceled,
    PastDue,
    Trialing,
    Unpaid,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionStatusResponse {
    pub subscription: Option<Subscription>,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct BillingPortalResponse {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct BillingPortalRequest {
    pub return_url: Option<String>,
}

// ── Watchlist ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Watchlist {
    pub id: Uuid,
    pub title: String,
    pub week_of: chrono::NaiveDate,
    pub video_url: Option<String>,
    pub notes: Option<String>,
    pub published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateWatchlistRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub week_of: chrono::NaiveDate,
    pub video_url: Option<String>,
    pub notes: Option<String>,
    pub published: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWatchlistRequest {
    pub title: Option<String>,
    pub week_of: Option<chrono::NaiveDate>,
    pub video_url: Option<String>,
    pub notes: Option<String>,
    pub published: Option<bool>,
}

// ── Watchlist Alert ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WatchlistAlert {
    pub id: Uuid,
    pub watchlist_id: Uuid,
    pub ticker: String,
    pub direction: TradeDirection,
    pub entry_zone: String,
    pub invalidation: String,
    pub profit_zones: Vec<String>,
    pub notes: Option<String>,
    pub chart_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "trade_direction", rename_all = "lowercase")]
pub enum TradeDirection {
    Bullish,
    Bearish,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateAlertRequest {
    #[validate(length(min = 1, message = "Ticker is required"))]
    pub ticker: String,
    pub direction: TradeDirection,
    #[validate(length(min = 1, message = "Entry zone is required"))]
    pub entry_zone: String,
    #[validate(length(min = 1, message = "Invalidation is required"))]
    pub invalidation: String,
    pub profit_zones: Vec<String>,
    pub notes: Option<String>,
    pub chart_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAlertRequest {
    pub ticker: Option<String>,
    pub direction: Option<TradeDirection>,
    pub entry_zone: Option<String>,
    pub invalidation: Option<String>,
    pub profit_zones: Option<Vec<String>>,
    pub notes: Option<String>,
    pub chart_url: Option<String>,
}

// ── Course Enrollment ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CourseEnrollment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub course_id: String,
    pub progress: i32,
    pub enrolled_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ── Refresh Token ───────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub family_id: Uuid,
    pub used: bool,
}

// ── Watchlist with alerts (joined response) ─────────────────────────────

#[derive(Debug, Serialize)]
pub struct WatchlistWithAlerts {
    #[serde(flatten)]
    pub watchlist: Watchlist,
    pub alerts: Vec<WatchlistAlert>,
}

// ── Admin dashboard stats ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AdminStats {
    pub total_members: i64,
    pub active_subscriptions: i64,
    pub monthly_subscriptions: i64,
    pub annual_subscriptions: i64,
    pub total_watchlists: i64,
    pub total_enrollments: i64,
    pub recent_members: Vec<UserResponse>,
}

// ── Blog Post ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "post_status", rename_all = "snake_case")]
pub enum PostStatus {
    Draft,
    PendingReview,
    Published,
    Private,
    Scheduled,
    Trash,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlogPost {
    pub id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub content_json: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image_id: Option<Uuid>,
    pub status: PostStatus,
    pub pre_trash_status: Option<PostStatus>,
    pub trashed_at: Option<DateTime<Utc>>,
    pub visibility: String,
    pub password_hash: Option<String>,
    pub format: String,
    pub is_sticky: bool,
    pub allow_comments: bool,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub reading_time_minutes: i32,
    pub word_count: i32,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BlogPostResponse {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub author_position: Option<String>,
    pub author_bio: Option<String>,
    pub author_website: Option<String>,
    pub author_twitter: Option<String>,
    pub author_linkedin: Option<String>,
    pub author_youtube: Option<String>,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub content_json: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image_url: Option<String>,
    pub status: PostStatus,
    pub pre_trash_status: Option<PostStatus>,
    pub trashed_at: Option<DateTime<Utc>>,
    pub visibility: String,
    pub is_password_protected: bool,
    pub format: String,
    pub is_sticky: bool,
    pub allow_comments: bool,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub reading_time_minutes: i32,
    pub word_count: i32,
    pub categories: Vec<BlogCategory>,
    pub tags: Vec<BlogTag>,
    pub meta: Vec<PostMeta>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BlogPostListItem {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
    pub featured_image_url: Option<String>,
    pub status: PostStatus,
    pub format: String,
    pub is_sticky: bool,
    pub reading_time_minutes: i32,
    pub word_count: i32,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub categories: Vec<BlogCategory>,
    pub tags: Vec<BlogTag>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub slug: Option<String>,
    pub content: Option<String>,
    pub content_json: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image_id: Option<Uuid>,
    pub status: Option<PostStatus>,
    pub visibility: Option<String>,
    pub is_sticky: Option<bool>,
    pub allow_comments: Option<bool>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub category_ids: Option<Vec<Uuid>>,
    pub tag_ids: Option<Vec<Uuid>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub post_password: Option<String>,
    pub author_id: Option<Uuid>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub content: Option<String>,
    pub content_json: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image_id: Option<Uuid>,
    pub status: Option<PostStatus>,
    pub visibility: Option<String>,
    pub is_sticky: Option<bool>,
    pub allow_comments: Option<bool>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub category_ids: Option<Vec<Uuid>>,
    pub tag_ids: Option<Vec<Uuid>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub post_password: Option<String>,
    pub author_id: Option<Uuid>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostStatusRequest {
    pub status: PostStatus,
}

#[derive(Debug, Deserialize)]
pub struct VerifyPostPasswordRequest {
    pub password: String,
}

// ── Post Meta ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct PostMeta {
    pub id: Uuid,
    pub post_id: Uuid,
    pub meta_key: String,
    pub meta_value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertPostMetaRequest {
    pub meta_key: String,
    pub meta_value: String,
}

#[derive(Debug, Deserialize)]
pub struct PostListParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<PostStatus>,
    pub author_id: Option<Uuid>,
    #[allow(dead_code)]
    pub category_slug: Option<String>,
    #[allow(dead_code)]
    pub tag_slug: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AutosaveRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub content_json: Option<serde_json::Value>,
}

// ── Blog Category ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlogCategory {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCategoryRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: Option<i32>,
}

// ── Blog Tag ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlogTag {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTagRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    pub slug: Option<String>,
}

// ── Blog Revision ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlogRevision {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub content: String,
    pub content_json: Option<serde_json::Value>,
    pub revision_number: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RevisionResponse {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub title: String,
    pub revision_number: i32,
    pub created_at: DateTime<Utc>,
}

// ── Media ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Media {
    pub id: Uuid,
    pub uploader_id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub title: Option<String>,
    pub mime_type: String,
    pub file_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub storage_path: String,
    pub url: String,
    pub focal_x: f64,
    pub focal_y: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMediaRequest {
    pub title: Option<String>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub focal_x: Option<f64>,
    pub focal_y: Option<f64>,
}

// ── Pagination ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl PaginationParams {
    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page();
        (page - 1) * per_page
    }

    pub fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

// ── Analytics ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AnalyticsIngestEvent {
    pub event_type: String,
    pub path: String,
    pub referrer: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsIngestRequest {
    pub session_id: Uuid,
    pub events: Vec<AnalyticsIngestEvent>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsTimeBucket {
    pub date: String,
    pub page_views: i64,
    pub unique_sessions: i64,
    pub impressions: i64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsTopPage {
    pub path: String,
    pub views: i64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsCtrPoint {
    pub date: String,
    pub cta_id: String,
    pub impressions: i64,
    pub clicks: i64,
    pub ctr: f64,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsSummary {
    pub from: String,
    pub to: String,
    pub total_page_views: i64,
    pub total_sessions: i64,
    pub total_impressions: i64,
    pub time_series: Vec<AnalyticsTimeBucket>,
    pub top_pages: Vec<AnalyticsTopPage>,
    pub ctr_series: Vec<AnalyticsCtrPoint>,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsSummaryQuery {
    /// Inclusive start date `YYYY-MM-DD` (UTC).
    pub from: String,
    /// Exclusive end date `YYYY-MM-DD` (UTC), or inclusive depending on interpretation — we use end-exclusive window [from, to).
    pub to: String,
}

// ── Course ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Course {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub short_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub trailer_video_url: Option<String>,
    pub difficulty: String,
    pub instructor_id: Uuid,
    pub price_cents: i32,
    pub currency: String,
    pub is_free: bool,
    pub is_included_in_subscription: bool,
    pub sort_order: i32,
    pub published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub estimated_duration_minutes: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CourseModule {
    pub id: Uuid,
    pub course_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CourseLesson {
    pub id: Uuid,
    pub module_id: Uuid,
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub content: String,
    pub content_json: Option<serde_json::Value>,
    pub video_url: Option<String>,
    pub video_duration_seconds: Option<i32>,
    pub sort_order: i32,
    pub is_preview: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LessonProgress {
    pub id: Uuid,
    pub user_id: Uuid,
    pub lesson_id: Uuid,
    pub completed: bool,
    pub progress_seconds: i32,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_accessed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCourseRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub short_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub trailer_video_url: Option<String>,
    pub difficulty: Option<String>,
    pub price_cents: Option<i32>,
    pub currency: Option<String>,
    pub is_free: Option<bool>,
    pub is_included_in_subscription: Option<bool>,
    pub sort_order: Option<i32>,
    pub published: Option<bool>,
    pub estimated_duration_minutes: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCourseRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub short_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub trailer_video_url: Option<String>,
    pub difficulty: Option<String>,
    pub price_cents: Option<i32>,
    pub currency: Option<String>,
    pub is_free: Option<bool>,
    pub is_included_in_subscription: Option<bool>,
    pub sort_order: Option<i32>,
    pub published: Option<bool>,
    pub estimated_duration_minutes: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateModuleRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModuleRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateLessonRequest {
    #[validate(length(min = 1, message = "Title is required"))]
    pub title: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub content_json: Option<serde_json::Value>,
    pub video_url: Option<String>,
    pub video_duration_seconds: Option<i32>,
    pub sort_order: Option<i32>,
    pub is_preview: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLessonRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub content_json: Option<serde_json::Value>,
    pub video_url: Option<String>,
    pub video_duration_seconds: Option<i32>,
    pub sort_order: Option<i32>,
    pub is_preview: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLessonProgressRequest {
    pub progress_seconds: Option<i32>,
    pub completed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CourseWithModules {
    #[serde(flatten)]
    pub course: Course,
    pub modules: Vec<ModuleWithLessons>,
    pub total_lessons: i64,
    pub total_duration_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct ModuleWithLessons {
    #[serde(flatten)]
    pub module: CourseModule,
    pub lessons: Vec<CourseLesson>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CourseListItem {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub short_description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub difficulty: String,
    pub instructor_name: String,
    pub price_cents: i32,
    pub is_free: bool,
    pub is_included_in_subscription: bool,
    pub published: bool,
    pub estimated_duration_minutes: i32,
    pub total_lessons: i64,
    pub created_at: DateTime<Utc>,
}

// ── Pricing Plans ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PricingPlan {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub stripe_price_id: Option<String>,
    pub stripe_product_id: Option<String>,
    pub amount_cents: i32,
    pub currency: String,
    pub interval: String,
    pub interval_count: i32,
    pub trial_days: i32,
    pub features: serde_json::Value,
    pub highlight_text: Option<String>,
    pub is_popular: bool,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePricingPlanRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub stripe_price_id: Option<String>,
    pub stripe_product_id: Option<String>,
    pub amount_cents: i32,
    pub currency: Option<String>,
    pub interval: Option<String>,
    pub interval_count: Option<i32>,
    pub trial_days: Option<i32>,
    pub features: Option<serde_json::Value>,
    pub highlight_text: Option<String>,
    pub is_popular: Option<bool>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePricingPlanRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub stripe_price_id: Option<String>,
    pub stripe_product_id: Option<String>,
    pub amount_cents: Option<i32>,
    pub currency: Option<String>,
    pub interval: Option<String>,
    pub interval_count: Option<i32>,
    pub trial_days: Option<i32>,
    pub features: Option<serde_json::Value>,
    pub highlight_text: Option<String>,
    pub is_popular: Option<bool>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PricingChangeLog {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub field_changed: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_by: Uuid,
    pub changed_at: DateTime<Utc>,
}

// ── Coupons ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "discount_type", rename_all = "lowercase")]
pub enum DiscountType {
    Percentage,
    #[sqlx(rename = "fixed_amount")]
    FixedAmount,
    #[sqlx(rename = "free_trial")]
    FreeTrial,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Coupon {
    pub id: Uuid,
    pub code: String,
    pub description: Option<String>,
    pub discount_type: DiscountType,
    pub discount_value: rust_decimal::Decimal,
    pub min_purchase_cents: Option<i32>,
    pub max_discount_cents: Option<i32>,
    pub applies_to: String,
    pub applicable_plan_ids: Vec<Uuid>,
    pub applicable_course_ids: Vec<Uuid>,
    pub usage_limit: Option<i32>,
    pub usage_count: i32,
    pub per_user_limit: i32,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub stackable: bool,
    pub first_purchase_only: bool,
    pub stripe_coupon_id: Option<String>,
    pub stripe_promotion_code_id: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCouponRequest {
    pub code: Option<String>,
    pub description: Option<String>,
    pub discount_type: DiscountType,
    pub discount_value: f64,
    pub min_purchase_cents: Option<i32>,
    pub max_discount_cents: Option<i32>,
    pub applies_to: Option<String>,
    pub applicable_plan_ids: Option<Vec<Uuid>>,
    pub applicable_course_ids: Option<Vec<Uuid>>,
    pub usage_limit: Option<i32>,
    pub per_user_limit: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
    pub stackable: Option<bool>,
    pub first_purchase_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCouponRequest {
    pub description: Option<String>,
    pub discount_type: Option<DiscountType>,
    pub discount_value: Option<f64>,
    pub min_purchase_cents: Option<i32>,
    pub max_discount_cents: Option<i32>,
    pub applies_to: Option<String>,
    pub applicable_plan_ids: Option<Vec<Uuid>>,
    pub applicable_course_ids: Option<Vec<Uuid>>,
    pub usage_limit: Option<i32>,
    pub per_user_limit: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
    pub stackable: Option<bool>,
    pub first_purchase_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ValidateCouponRequest {
    pub code: String,
    pub plan_id: Option<Uuid>,
    pub course_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CouponValidationResponse {
    pub valid: bool,
    pub coupon: Option<Coupon>,
    pub discount_amount_cents: Option<i32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CouponUsage {
    pub id: Uuid,
    pub coupon_id: Uuid,
    pub user_id: Uuid,
    pub subscription_id: Option<Uuid>,
    pub discount_applied_cents: i32,
    pub used_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BulkCouponRequest {
    pub count: i32,
    pub prefix: Option<String>,
    pub discount_type: DiscountType,
    pub discount_value: f64,
    pub usage_limit: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
}

// ── Popups ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Popup {
    pub id: Uuid,
    pub name: String,
    pub popup_type: String,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub content_json: serde_json::Value,
    pub style_json: serde_json::Value,
    pub targeting_rules: serde_json::Value,
    pub display_frequency: String,
    pub frequency_config: serde_json::Value,
    pub success_message: Option<String>,
    pub redirect_url: Option<String>,
    pub is_active: bool,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub priority: i32,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePopupRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    pub popup_type: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<serde_json::Value>,
    pub content_json: Option<serde_json::Value>,
    pub style_json: Option<serde_json::Value>,
    pub targeting_rules: Option<serde_json::Value>,
    pub display_frequency: Option<String>,
    pub frequency_config: Option<serde_json::Value>,
    pub success_message: Option<String>,
    pub redirect_url: Option<String>,
    pub is_active: Option<bool>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePopupRequest {
    pub name: Option<String>,
    pub popup_type: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<serde_json::Value>,
    pub content_json: Option<serde_json::Value>,
    pub style_json: Option<serde_json::Value>,
    pub targeting_rules: Option<serde_json::Value>,
    pub display_frequency: Option<String>,
    pub frequency_config: Option<serde_json::Value>,
    pub success_message: Option<String>,
    pub redirect_url: Option<String>,
    pub is_active: Option<bool>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PopupSubmission {
    pub id: Uuid,
    pub popup_id: Uuid,
    pub user_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub form_data: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub page_url: Option<String>,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PopupSubmitRequest {
    pub popup_id: Uuid,
    pub session_id: Option<Uuid>,
    pub form_data: serde_json::Value,
    pub page_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PopupAnalytics {
    pub popup_id: Uuid,
    pub popup_name: String,
    pub total_impressions: i64,
    pub total_closes: i64,
    pub total_submissions: i64,
    pub conversion_rate: f64,
}

// ── Sales / Revenue Analytics ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct SalesEvent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub event_type: String,
    pub amount_cents: i32,
    pub currency: String,
    pub plan_id: Option<Uuid>,
    pub coupon_id: Option<Uuid>,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct MonthlyRevenueSnapshot {
    pub id: Uuid,
    pub year: i32,
    pub month: i32,
    pub mrr_cents: i64,
    pub arr_cents: i64,
    pub total_revenue_cents: i64,
    pub new_subscribers: i32,
    pub churned_subscribers: i32,
    pub net_subscriber_change: i32,
    pub avg_revenue_per_user_cents: i32,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct RevenueAnalytics {
    pub total_revenue_cents: i64,
    pub mrr_cents: i64,
    pub arr_cents: i64,
    pub total_subscribers: i64,
    pub churn_rate: f64,
    pub avg_revenue_per_user_cents: i64,
    pub revenue_by_month: Vec<MonthlyRevenueSummary>,
    pub revenue_by_plan: Vec<PlanRevenueSummary>,
    pub recent_sales: Vec<SalesEvent>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct MonthlyRevenueSummary {
    pub year: i32,
    pub month: i32,
    pub revenue_cents: i64,
    pub new_subscribers: i64,
    pub churned: i64,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct PlanRevenueSummary {
    pub plan_name: String,
    pub subscriber_count: i64,
    pub revenue_cents: i64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RevenueAnalyticsQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub granularity: Option<String>,
}
```

#### `backend/src/db.rs`

```rust
// backend/src/db.rs
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::*;

/// Trim + lowercase so logins match Gmail-style case-insensitive addresses.
#[must_use]
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

// ── Users ───────────────────────────────────────────────────────────────

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    name: &str,
) -> Result<User, sqlx::Error> {
    let email = normalize_email(email);
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, email, password_hash, name, role)
        VALUES ($1, $2, $3, $4, 'member')
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .bind(password_hash)
    .bind(name)
    .fetch_one(pool)
    .await
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    let email = normalize_email(email);
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE lower(btrim(email)) = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_users(
    pool: &PgPool,
    offset: i64,
    limit: i64,
) -> Result<(Vec<User>, i64), sqlx::Error> {
    let users = sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    Ok((users, total.0))
}

pub async fn update_user_role(
    pool: &PgPool,
    user_id: Uuid,
    role: &UserRole,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2 RETURNING *",
    )
    .bind(role)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn delete_user(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn seed_admin(
    pool: &PgPool,
    email: &str,
    password: &str,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Password hash error: {e}"))?
        .to_string();

    let email = normalize_email(email);

    // New installs: insert with seeded password. Existing admin: keep their password — do not
    // overwrite on every server start (that used to reset people back to ADMIN_PASSWORD / defaults).
    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, name, role)
        VALUES ($1, $2, $3, $4, 'admin')
        ON CONFLICT (email) DO UPDATE
            SET name = EXCLUDED.name,
                role = EXCLUDED.role,
                updated_at = NOW()
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&email)
    .bind(&password_hash)
    .bind(name)
    .execute(pool)
    .await?;

    tracing::info!(
        "Admin user seeded (password unchanged if email already existed): {}",
        email
    );
    Ok(())
}

pub async fn recent_members(pool: &PgPool, limit: i64) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE role = 'member' ORDER BY created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

// ── Password Reset Tokens ───────────────────────────────────────────────

pub async fn create_password_reset_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    // Invalidate any existing unused tokens for this user
    sqlx::query("UPDATE password_reset_tokens SET used = TRUE WHERE user_id = $1 AND used = FALSE")
        .bind(user_id)
        .execute(pool)
        .await?;

    sqlx::query(
        "INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_password_reset_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<PasswordResetToken>, sqlx::Error> {
    sqlx::query_as::<_, PasswordResetToken>(
        "SELECT * FROM password_reset_tokens WHERE token_hash = $1 AND used = FALSE AND expires_at > NOW()"
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

pub async fn mark_reset_token_used(pool: &PgPool, token_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE password_reset_tokens SET used = TRUE WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_user_password(
    pool: &PgPool,
    user_id: Uuid,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(password_hash)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Refresh Tokens ──────────────────────────────────────────────────────

pub async fn store_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
    family_id: Uuid,
    used: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, family_id, used) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .bind(family_id)
    .bind(used)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_refresh_token_used(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE refresh_tokens SET used = true WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_refresh_tokens_by_family(
    pool: &PgPool,
    family_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE family_id = $1")
        .bind(family_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_refresh_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<RefreshToken>, sqlx::Error> {
    sqlx::query_as::<_, RefreshToken>(
        "SELECT * FROM refresh_tokens WHERE token_hash = $1 AND expires_at > NOW()",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

pub async fn delete_user_refresh_tokens(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Returns `true` if this event was newly claimed, `false` if it was already processed.
pub async fn try_claim_stripe_webhook_event(
    pool: &PgPool,
    event_id: &str,
    event_type: &str,
) -> Result<bool, sqlx::Error> {
    let res = sqlx::query(
        r#"INSERT INTO processed_webhook_events (event_id, event_type) VALUES ($1, $2)
           ON CONFLICT (event_id) DO NOTHING"#,
    )
    .bind(event_id)
    .bind(event_type)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn cleanup_old_stripe_webhook_events(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "DELETE FROM processed_webhook_events WHERE processed_at < NOW() - INTERVAL '30 days'",
    )
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

// ── Subscriptions ───────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn upsert_subscription(
    pool: &PgPool,
    user_id: Uuid,
    stripe_customer_id: &str,
    stripe_subscription_id: &str,
    plan: &SubscriptionPlan,
    status: &SubscriptionStatus,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Subscription, sqlx::Error> {
    sqlx::query_as::<_, Subscription>(
        r#"
        INSERT INTO subscriptions (id, user_id, stripe_customer_id, stripe_subscription_id, plan, status, current_period_start, current_period_end)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (stripe_subscription_id)
        DO UPDATE SET status = $6, plan = $5, current_period_start = $7, current_period_end = $8, updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(stripe_customer_id)
    .bind(stripe_subscription_id)
    .bind(plan)
    .bind(status)
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await
}

pub async fn find_subscription_by_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<Subscription>, sqlx::Error> {
    sqlx::query_as::<_, Subscription>(
        "SELECT * FROM subscriptions WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn find_subscription_by_stripe_id(
    pool: &PgPool,
    stripe_sub_id: &str,
) -> Result<Option<Subscription>, sqlx::Error> {
    sqlx::query_as::<_, Subscription>(
        "SELECT * FROM subscriptions WHERE stripe_subscription_id = $1",
    )
    .bind(stripe_sub_id)
    .fetch_optional(pool)
    .await
}

pub async fn find_user_by_stripe_customer(
    pool: &PgPool,
    customer_id: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u JOIN subscriptions s ON u.id = s.user_id WHERE s.stripe_customer_id = $1 LIMIT 1",
    )
    .bind(customer_id)
    .fetch_optional(pool)
    .await
}

pub async fn count_active_subscriptions(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM subscriptions WHERE status = 'active'")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn count_subscriptions_by_plan(
    pool: &PgPool,
    plan: &SubscriptionPlan,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM subscriptions WHERE plan = $1 AND status = 'active'")
            .bind(plan)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

// ── Watchlists ──────────────────────────────────────────────────────────

pub async fn create_watchlist(
    pool: &PgPool,
    title: &str,
    week_of: NaiveDate,
    video_url: Option<&str>,
    notes: Option<&str>,
    published: bool,
) -> Result<Watchlist, sqlx::Error> {
    let published_at = if published { Some(Utc::now()) } else { None };

    sqlx::query_as::<_, Watchlist>(
        r#"
        INSERT INTO watchlists (id, title, week_of, video_url, notes, published, published_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(title)
    .bind(week_of)
    .bind(video_url)
    .bind(notes)
    .bind(published)
    .bind(published_at)
    .fetch_one(pool)
    .await
}

pub async fn update_watchlist(
    pool: &PgPool,
    id: Uuid,
    req: &UpdateWatchlistRequest,
) -> Result<Watchlist, sqlx::Error> {
    let existing = sqlx::query_as::<_, Watchlist>("SELECT * FROM watchlists WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;

    let title = req.title.as_deref().unwrap_or(&existing.title);
    let week_of = req.week_of.unwrap_or(existing.week_of);
    let video_url = req.video_url.as_deref().or(existing.video_url.as_deref());
    let notes = req.notes.as_deref().or(existing.notes.as_deref());
    let published = req.published.unwrap_or(existing.published);
    let published_at = if published && existing.published_at.is_none() {
        Some(Utc::now())
    } else {
        existing.published_at
    };

    sqlx::query_as::<_, Watchlist>(
        r#"
        UPDATE watchlists SET title = $1, week_of = $2, video_url = $3, notes = $4, published = $5, published_at = $6, updated_at = NOW()
        WHERE id = $7 RETURNING *
        "#,
    )
    .bind(title)
    .bind(week_of)
    .bind(video_url)
    .bind(notes)
    .bind(published)
    .bind(published_at)
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_watchlist(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM watchlists WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_watchlist(pool: &PgPool, id: Uuid) -> Result<Option<Watchlist>, sqlx::Error> {
    sqlx::query_as::<_, Watchlist>("SELECT * FROM watchlists WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_watchlists(
    pool: &PgPool,
    offset: i64,
    limit: i64,
    published_only: bool,
) -> Result<(Vec<Watchlist>, i64), sqlx::Error> {
    let (watchlists, total) = if published_only {
        let wl = sqlx::query_as::<_, Watchlist>(
            "SELECT * FROM watchlists WHERE published = true ORDER BY week_of DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let t: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM watchlists WHERE published = true")
            .fetch_one(pool)
            .await?;
        (wl, t.0)
    } else {
        let wl = sqlx::query_as::<_, Watchlist>(
            "SELECT * FROM watchlists ORDER BY week_of DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        let t: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM watchlists")
            .fetch_one(pool)
            .await?;
        (wl, t.0)
    };

    Ok((watchlists, total))
}

pub async fn count_watchlists(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM watchlists")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

// ── Watchlist Alerts ────────────────────────────────────────────────────

pub async fn create_alert(
    pool: &PgPool,
    watchlist_id: Uuid,
    req: &CreateAlertRequest,
) -> Result<WatchlistAlert, sqlx::Error> {
    sqlx::query_as::<_, WatchlistAlert>(
        r#"
        INSERT INTO watchlist_alerts (id, watchlist_id, ticker, direction, entry_zone, invalidation, profit_zones, notes, chart_url)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(watchlist_id)
    .bind(&req.ticker)
    .bind(&req.direction)
    .bind(&req.entry_zone)
    .bind(&req.invalidation)
    .bind(&req.profit_zones)
    .bind(req.notes.as_deref())
    .bind(req.chart_url.as_deref())
    .fetch_one(pool)
    .await
}

pub async fn get_alerts_for_watchlist(
    pool: &PgPool,
    watchlist_id: Uuid,
) -> Result<Vec<WatchlistAlert>, sqlx::Error> {
    sqlx::query_as::<_, WatchlistAlert>(
        "SELECT * FROM watchlist_alerts WHERE watchlist_id = $1 ORDER BY created_at ASC",
    )
    .bind(watchlist_id)
    .fetch_all(pool)
    .await
}

pub async fn update_alert(
    pool: &PgPool,
    alert_id: Uuid,
    req: &UpdateAlertRequest,
) -> Result<WatchlistAlert, sqlx::Error> {
    let existing =
        sqlx::query_as::<_, WatchlistAlert>("SELECT * FROM watchlist_alerts WHERE id = $1")
            .bind(alert_id)
            .fetch_one(pool)
            .await?;

    sqlx::query_as::<_, WatchlistAlert>(
        r#"
        UPDATE watchlist_alerts SET
            ticker = $1, direction = $2, entry_zone = $3, invalidation = $4,
            profit_zones = $5, notes = $6, chart_url = $7
        WHERE id = $8 RETURNING *
        "#,
    )
    .bind(req.ticker.as_deref().unwrap_or(&existing.ticker))
    .bind(req.direction.as_ref().unwrap_or(&existing.direction))
    .bind(req.entry_zone.as_deref().unwrap_or(&existing.entry_zone))
    .bind(
        req.invalidation
            .as_deref()
            .unwrap_or(&existing.invalidation),
    )
    .bind(req.profit_zones.as_ref().unwrap_or(&existing.profit_zones))
    .bind(req.notes.as_deref().or(existing.notes.as_deref()))
    .bind(req.chart_url.as_deref().or(existing.chart_url.as_deref()))
    .bind(alert_id)
    .fetch_one(pool)
    .await
}

pub async fn delete_alert(pool: &PgPool, alert_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM watchlist_alerts WHERE id = $1")
        .bind(alert_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Course Enrollments ──────────────────────────────────────────────────

#[allow(dead_code)]
pub async fn enroll_user(
    pool: &PgPool,
    user_id: Uuid,
    course_id: &str,
) -> Result<CourseEnrollment, sqlx::Error> {
    sqlx::query_as::<_, CourseEnrollment>(
        r#"
        INSERT INTO course_enrollments (id, user_id, course_id) VALUES ($1, $2, $3)
        ON CONFLICT (user_id, course_id) DO NOTHING
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(course_id)
    .fetch_one(pool)
    .await
}

pub async fn get_user_enrollments(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<CourseEnrollment>, sqlx::Error> {
    sqlx::query_as::<_, CourseEnrollment>(
        "SELECT * FROM course_enrollments WHERE user_id = $1 ORDER BY enrolled_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn update_course_progress(
    pool: &PgPool,
    user_id: Uuid,
    course_id: &str,
    progress: i32,
) -> Result<CourseEnrollment, sqlx::Error> {
    let completed_at = if progress >= 100 {
        Some(Utc::now())
    } else {
        None
    };

    sqlx::query_as::<_, CourseEnrollment>(
        r#"
        UPDATE course_enrollments SET progress = $1, completed_at = $2
        WHERE user_id = $3 AND course_id = $4
        RETURNING *
        "#,
    )
    .bind(progress)
    .bind(completed_at)
    .bind(user_id)
    .bind(course_id)
    .fetch_one(pool)
    .await
}

pub async fn count_enrollments(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM course_enrollments")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

// ── Blog Posts ─────────────────────────────────────────────────────────

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn compute_word_count(html: &str) -> i32 {
    let text: String = html
        .chars()
        .fold((String::new(), false), |(mut out, in_tag), c| {
            if c == '<' {
                (out, true)
            } else if c == '>' {
                out.push(' ');
                (out, false)
            } else if !in_tag {
                out.push(c);
                (out, false)
            } else {
                (out, true)
            }
        })
        .0;
    text.split_whitespace().count() as i32
}

fn compute_reading_time(word_count: i32) -> i32 {
    (word_count as f64 / 238.0).ceil() as i32
}

pub async fn create_blog_post(
    pool: &PgPool,
    author_id: Uuid,
    req: &CreatePostRequest,
    password_hash: Option<&str>,
) -> Result<BlogPost, sqlx::Error> {
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.title));
    let content = req.content.as_deref().unwrap_or("");
    let wc = compute_word_count(content);
    let rt = compute_reading_time(wc);
    let status = req.status.clone().unwrap_or(PostStatus::Draft);
    let published_at = if status == PostStatus::Published {
        Some(Utc::now())
    } else {
        None
    };

    sqlx::query_as::<_, BlogPost>(
        r#"
        INSERT INTO blog_posts (
            id, author_id, title, slug, content, content_json, excerpt,
            featured_image_id, status, visibility, password_hash, format, is_sticky, allow_comments,
            meta_title, meta_description, canonical_url, og_image_url,
            reading_time_minutes, word_count, scheduled_at, published_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7,
            $8, $9, $10, $11, $12, $13, $14,
            $15, $16, $17, $18,
            $19, $20, $21, $22
        ) RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(author_id)
    .bind(&req.title)
    .bind(&slug)
    .bind(content)
    .bind(&req.content_json)
    .bind(req.excerpt.as_deref().unwrap_or(""))
    .bind(req.featured_image_id)
    .bind(&status)
    .bind(req.visibility.as_deref().unwrap_or("public"))
    .bind(password_hash)
    .bind(req.format.as_deref().unwrap_or("standard"))
    .bind(req.is_sticky.unwrap_or(false))
    .bind(req.allow_comments.unwrap_or(true))
    .bind(req.meta_title.as_deref().unwrap_or(""))
    .bind(req.meta_description.as_deref().unwrap_or(""))
    .bind(req.canonical_url.as_deref().unwrap_or(""))
    .bind(req.og_image_url.as_deref().unwrap_or(""))
    .bind(rt)
    .bind(wc)
    .bind(req.scheduled_at)
    .bind(published_at)
    .fetch_one(pool)
    .await
}

pub async fn update_blog_post(
    pool: &PgPool,
    post_id: Uuid,
    req: &UpdatePostRequest,
    password_hash_update: Option<Option<String>>,
    author_id_update: Option<Uuid>,
) -> Result<BlogPost, sqlx::Error> {
    let existing = sqlx::query_as::<_, BlogPost>("SELECT * FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(pool)
        .await?;

    let title = req.title.as_deref().unwrap_or(&existing.title);
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| existing.slug.clone());
    let content = req.content.as_deref().unwrap_or(&existing.content);
    let wc = compute_word_count(content);
    let rt = compute_reading_time(wc);
    let status = req.status.clone().unwrap_or(existing.status.clone());
    let published_at = if status == PostStatus::Published && existing.published_at.is_none() {
        Some(Utc::now())
    } else {
        existing.published_at
    };
    let (pre_trash_status, trashed_at): (Option<PostStatus>, Option<DateTime<Utc>>) = match &status
    {
        PostStatus::Trash if existing.status != PostStatus::Trash => {
            (Some(existing.status.clone()), Some(Utc::now()))
        }
        PostStatus::Trash => (existing.pre_trash_status, existing.trashed_at),
        _ => (None, None),
    };
    let new_password_hash = match password_hash_update {
        Some(v) => v,
        None => existing.password_hash.clone(),
    };
    let new_author_id = author_id_update.unwrap_or(existing.author_id);

    sqlx::query_as::<_, BlogPost>(
        r#"
        UPDATE blog_posts SET
            title = $1, slug = $2, content = $3, content_json = $4, excerpt = $5,
            featured_image_id = $6, status = $7, visibility = $8, password_hash = $9,
            format = $10, is_sticky = $11, allow_comments = $12, meta_title = $13,
            meta_description = $14, canonical_url = $15, og_image_url = $16,
            reading_time_minutes = $17, word_count = $18, scheduled_at = $19, published_at = $20,
            pre_trash_status = $21, trashed_at = $22,
            author_id = $23, updated_at = NOW()
        WHERE id = $24 RETURNING *
        "#,
    )
    .bind(title)
    .bind(&slug)
    .bind(content)
    .bind(req.content_json.as_ref().or(existing.content_json.as_ref()))
    .bind(
        req.excerpt
            .as_deref()
            .unwrap_or(existing.excerpt.as_deref().unwrap_or("")),
    )
    .bind(req.featured_image_id.or(existing.featured_image_id))
    .bind(&status)
    .bind(req.visibility.as_deref().unwrap_or(&existing.visibility))
    .bind(new_password_hash.as_deref())
    .bind(req.format.as_deref().unwrap_or(&existing.format))
    .bind(req.is_sticky.unwrap_or(existing.is_sticky))
    .bind(req.allow_comments.unwrap_or(existing.allow_comments))
    .bind(
        req.meta_title
            .as_deref()
            .unwrap_or(existing.meta_title.as_deref().unwrap_or("")),
    )
    .bind(
        req.meta_description
            .as_deref()
            .unwrap_or(existing.meta_description.as_deref().unwrap_or("")),
    )
    .bind(
        req.canonical_url
            .as_deref()
            .unwrap_or(existing.canonical_url.as_deref().unwrap_or("")),
    )
    .bind(
        req.og_image_url
            .as_deref()
            .unwrap_or(existing.og_image_url.as_deref().unwrap_or("")),
    )
    .bind(rt)
    .bind(wc)
    .bind(req.scheduled_at.or(existing.scheduled_at))
    .bind(published_at)
    .bind(pre_trash_status)
    .bind(trashed_at)
    .bind(new_author_id)
    .bind(post_id)
    .fetch_one(pool)
    .await
}

pub async fn autosave_blog_post(
    pool: &PgPool,
    post_id: Uuid,
    req: &AutosaveRequest,
) -> Result<BlogPost, sqlx::Error> {
    let existing = sqlx::query_as::<_, BlogPost>("SELECT * FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(pool)
        .await?;

    let title = req.title.as_deref().unwrap_or(&existing.title);
    let content = req.content.as_deref().unwrap_or(&existing.content);
    let wc = compute_word_count(content);
    let rt = compute_reading_time(wc);

    sqlx::query_as::<_, BlogPost>(
        r#"
        UPDATE blog_posts SET title = $1, content = $2, content_json = $3,
            reading_time_minutes = $4, word_count = $5, updated_at = NOW()
        WHERE id = $6 RETURNING *
        "#,
    )
    .bind(title)
    .bind(content)
    .bind(req.content_json.as_ref().or(existing.content_json.as_ref()))
    .bind(rt)
    .bind(wc)
    .bind(post_id)
    .fetch_one(pool)
    .await
}

/// Changes status for posts that are not in the trash. Does not handle `trash` or restore — use
/// [`move_post_to_trash`] / [`restore_post_from_trash`] from handlers instead.
pub async fn update_post_status(
    pool: &PgPool,
    post_id: Uuid,
    status: &PostStatus,
) -> Result<BlogPost, sqlx::Error> {
    debug_assert!(*status != PostStatus::Trash);
    let published_at_expr = if *status == PostStatus::Published {
        "COALESCE(published_at, NOW())"
    } else {
        "published_at"
    };

    let q = format!(
        "UPDATE blog_posts SET status = $1, published_at = {}, updated_at = NOW() WHERE id = $2 RETURNING *",
        published_at_expr
    );

    sqlx::query_as::<_, BlogPost>(&q)
        .bind(status)
        .bind(post_id)
        .fetch_one(pool)
        .await
}

pub async fn move_post_to_trash(pool: &PgPool, post_id: Uuid) -> Result<BlogPost, sqlx::Error> {
    let existing = sqlx::query_as::<_, BlogPost>("SELECT * FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(pool)
        .await?;
    if existing.status == PostStatus::Trash {
        return Ok(existing);
    }
    let prev = existing.status.clone();
    sqlx::query_as::<_, BlogPost>(
        r#"
        UPDATE blog_posts SET
            status = 'trash',
            pre_trash_status = $1,
            trashed_at = NOW(),
            updated_at = NOW()
        WHERE id = $2
        RETURNING *
        "#,
    )
    .bind(prev)
    .bind(post_id)
    .fetch_one(pool)
    .await
}

pub async fn restore_post_from_trash(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<BlogPost, sqlx::Error> {
    let existing = sqlx::query_as::<_, BlogPost>("SELECT * FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .fetch_one(pool)
        .await?;
    if existing.status != PostStatus::Trash {
        return Ok(existing);
    }
    let new_status = existing
        .pre_trash_status
        .clone()
        .unwrap_or(PostStatus::Draft);
    let published_at_expr = if new_status == PostStatus::Published {
        "COALESCE(published_at, NOW())"
    } else {
        "published_at"
    };
    let q = format!(
        "UPDATE blog_posts SET status = $1, pre_trash_status = NULL, trashed_at = NULL, published_at = {}, updated_at = NOW() WHERE id = $2 RETURNING *",
        published_at_expr
    );
    sqlx::query_as::<_, BlogPost>(&q)
        .bind(new_status)
        .bind(post_id)
        .fetch_one(pool)
        .await
}

pub async fn delete_blog_post(pool: &PgPool, post_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_blog_post(pool: &PgPool, post_id: Uuid) -> Result<Option<BlogPost>, sqlx::Error> {
    sqlx::query_as::<_, BlogPost>("SELECT * FROM blog_posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_blog_post_by_slug(
    pool: &PgPool,
    slug: &str,
) -> Result<Option<BlogPost>, sqlx::Error> {
    sqlx::query_as::<_, BlogPost>(
        "SELECT * FROM blog_posts WHERE slug = $1 AND status = 'published'",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
}

pub async fn list_blog_posts_admin(
    pool: &PgPool,
    offset: i64,
    limit: i64,
    status: Option<&PostStatus>,
    author_id: Option<Uuid>,
    search: Option<&str>,
) -> Result<(Vec<BlogPost>, i64), sqlx::Error> {
    let mut where_clauses = vec!["1=1".to_string()];
    if status.is_none() {
        where_clauses.push("status <> 'trash'".to_string());
    }
    if status.is_some() {
        where_clauses.push("status = $3".to_string());
    }
    if author_id.is_some() {
        where_clauses.push("author_id = $4".to_string());
    }
    if search.is_some() {
        where_clauses.push("(title ILIKE $5 OR content ILIKE $5)".to_string());
    }
    let where_clause = where_clauses.join(" AND ");

    let query_str = format!(
        "SELECT * FROM blog_posts WHERE {} ORDER BY updated_at DESC LIMIT $1 OFFSET $2",
        where_clause
    );
    let count_str = format!("SELECT COUNT(*) FROM blog_posts WHERE {}", where_clause);

    let mut q = sqlx::query_as::<_, BlogPost>(&query_str)
        .bind(limit)
        .bind(offset);
    let mut cq = sqlx::query_as::<_, (i64,)>(&count_str);

    if let Some(s) = status {
        q = q.bind(s);
        cq = cq.bind(s);
    }
    if let Some(a) = author_id {
        q = q.bind(a);
        cq = cq.bind(a);
    }
    if let Some(s) = search {
        let pattern = format!("%{}%", s);
        q = q.bind(pattern.clone());
        cq = cq.bind(pattern);
    }

    let posts = q.fetch_all(pool).await?;
    let total = cq.fetch_one(pool).await?.0;
    Ok((posts, total))
}

pub async fn list_published_posts(
    pool: &PgPool,
    offset: i64,
    limit: i64,
) -> Result<(Vec<BlogPost>, i64), sqlx::Error> {
    let posts = sqlx::query_as::<_, BlogPost>(
        "SELECT * FROM blog_posts WHERE status = 'published' ORDER BY is_sticky DESC, published_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM blog_posts WHERE status = 'published'")
            .fetch_one(pool)
            .await?;
    Ok((posts, total.0))
}

pub async fn list_published_posts_by_category(
    pool: &PgPool,
    category_slug: &str,
    offset: i64,
    limit: i64,
) -> Result<(Vec<BlogPost>, i64), sqlx::Error> {
    let posts = sqlx::query_as::<_, BlogPost>(
        r#"
        SELECT bp.* FROM blog_posts bp
        JOIN blog_post_categories bpc ON bp.id = bpc.post_id
        JOIN blog_categories bc ON bpc.category_id = bc.id
        WHERE bp.status = 'published' AND bc.slug = $1
        ORDER BY bp.is_sticky DESC, bp.published_at DESC LIMIT $2 OFFSET $3
        "#,
    )
    .bind(category_slug)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM blog_posts bp
        JOIN blog_post_categories bpc ON bp.id = bpc.post_id
        JOIN blog_categories bc ON bpc.category_id = bc.id
        WHERE bp.status = 'published' AND bc.slug = $1
        "#,
    )
    .bind(category_slug)
    .fetch_one(pool)
    .await?;
    Ok((posts, total.0))
}

pub async fn list_published_posts_by_tag(
    pool: &PgPool,
    tag_slug: &str,
    offset: i64,
    limit: i64,
) -> Result<(Vec<BlogPost>, i64), sqlx::Error> {
    let posts = sqlx::query_as::<_, BlogPost>(
        r#"
        SELECT bp.* FROM blog_posts bp
        JOIN blog_post_tags bpt ON bp.id = bpt.post_id
        JOIN blog_tags bt ON bpt.tag_id = bt.id
        WHERE bp.status = 'published' AND bt.slug = $1
        ORDER BY bp.is_sticky DESC, bp.published_at DESC LIMIT $2 OFFSET $3
        "#,
    )
    .bind(tag_slug)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM blog_posts bp
        JOIN blog_post_tags bpt ON bp.id = bpt.post_id
        JOIN blog_tags bt ON bpt.tag_id = bt.id
        WHERE bp.status = 'published' AND bt.slug = $1
        "#,
    )
    .bind(tag_slug)
    .fetch_one(pool)
    .await?;
    Ok((posts, total.0))
}

pub async fn list_all_published_slugs(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT slug FROM blog_posts WHERE status = 'published' ORDER BY published_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

// ── Post ↔ Category / Tag junctions ───────────────────────────────────

pub async fn set_post_categories(
    pool: &PgPool,
    post_id: Uuid,
    category_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM blog_post_categories WHERE post_id = $1")
        .bind(post_id)
        .execute(pool)
        .await?;

    for cid in category_ids {
        sqlx::query("INSERT INTO blog_post_categories (post_id, category_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(post_id)
            .bind(cid)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn set_post_tags(
    pool: &PgPool,
    post_id: Uuid,
    tag_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM blog_post_tags WHERE post_id = $1")
        .bind(post_id)
        .execute(pool)
        .await?;

    for tid in tag_ids {
        sqlx::query(
            "INSERT INTO blog_post_tags (post_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(post_id)
        .bind(tid)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn get_categories_for_post(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<Vec<BlogCategory>, sqlx::Error> {
    sqlx::query_as::<_, BlogCategory>(
        r#"
        SELECT bc.* FROM blog_categories bc
        JOIN blog_post_categories bpc ON bc.id = bpc.category_id
        WHERE bpc.post_id = $1 ORDER BY bc.sort_order, bc.name
        "#,
    )
    .bind(post_id)
    .fetch_all(pool)
    .await
}

pub async fn get_tags_for_post(pool: &PgPool, post_id: Uuid) -> Result<Vec<BlogTag>, sqlx::Error> {
    sqlx::query_as::<_, BlogTag>(
        r#"
        SELECT bt.* FROM blog_tags bt
        JOIN blog_post_tags bpt ON bt.id = bpt.tag_id
        WHERE bpt.post_id = $1 ORDER BY bt.name
        "#,
    )
    .bind(post_id)
    .fetch_all(pool)
    .await
}

// ── Blog Categories ───────────────────────────────────────────────────

pub async fn create_blog_category(
    pool: &PgPool,
    req: &CreateCategoryRequest,
) -> Result<BlogCategory, sqlx::Error> {
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.name));

    sqlx::query_as::<_, BlogCategory>(
        r#"
        INSERT INTO blog_categories (id, name, slug, description, parent_id, sort_order)
        VALUES ($1, $2, $3, $4, $5, $6) RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&req.name)
    .bind(&slug)
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(req.parent_id)
    .bind(req.sort_order.unwrap_or(0))
    .fetch_one(pool)
    .await
}

pub async fn update_blog_category(
    pool: &PgPool,
    id: Uuid,
    req: &UpdateCategoryRequest,
) -> Result<BlogCategory, sqlx::Error> {
    let existing = sqlx::query_as::<_, BlogCategory>("SELECT * FROM blog_categories WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;

    let name = req.name.as_deref().unwrap_or(&existing.name);
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| existing.slug.clone());

    sqlx::query_as::<_, BlogCategory>(
        r#"
        UPDATE blog_categories SET name = $1, slug = $2, description = $3, parent_id = $4, sort_order = $5
        WHERE id = $6 RETURNING *
        "#,
    )
    .bind(name)
    .bind(&slug)
    .bind(req.description.as_deref().unwrap_or(existing.description.as_deref().unwrap_or("")))
    .bind(req.parent_id.or(existing.parent_id))
    .bind(req.sort_order.unwrap_or(existing.sort_order))
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_blog_category(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM blog_categories WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_blog_categories(pool: &PgPool) -> Result<Vec<BlogCategory>, sqlx::Error> {
    sqlx::query_as::<_, BlogCategory>("SELECT * FROM blog_categories ORDER BY sort_order, name")
        .fetch_all(pool)
        .await
}

// ── Blog Tags ─────────────────────────────────────────────────────────

pub async fn create_blog_tag(
    pool: &PgPool,
    req: &CreateTagRequest,
) -> Result<BlogTag, sqlx::Error> {
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.name));

    sqlx::query_as::<_, BlogTag>(
        "INSERT INTO blog_tags (id, name, slug) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(&req.name)
    .bind(&slug)
    .fetch_one(pool)
    .await
}

pub async fn delete_blog_tag(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM blog_tags WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_blog_tags(pool: &PgPool) -> Result<Vec<BlogTag>, sqlx::Error> {
    sqlx::query_as::<_, BlogTag>("SELECT * FROM blog_tags ORDER BY name")
        .fetch_all(pool)
        .await
}

// ── Blog Revisions ────────────────────────────────────────────────────

pub async fn create_blog_revision(
    pool: &PgPool,
    post_id: Uuid,
    author_id: Uuid,
    title: &str,
    content: &str,
    content_json: Option<&serde_json::Value>,
) -> Result<BlogRevision, sqlx::Error> {
    let next_rev: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM blog_revisions WHERE post_id = $1",
    )
    .bind(post_id)
    .fetch_one(pool)
    .await?;

    sqlx::query_as::<_, BlogRevision>(
        r#"
        INSERT INTO blog_revisions (id, post_id, author_id, title, content, content_json, revision_number)
        VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(post_id)
    .bind(author_id)
    .bind(title)
    .bind(content)
    .bind(content_json)
    .bind(next_rev.0 as i32)
    .fetch_one(pool)
    .await
}

pub async fn list_blog_revisions(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<Vec<BlogRevision>, sqlx::Error> {
    sqlx::query_as::<_, BlogRevision>(
        "SELECT * FROM blog_revisions WHERE post_id = $1 ORDER BY revision_number DESC",
    )
    .bind(post_id)
    .fetch_all(pool)
    .await
}

pub async fn get_blog_revision(
    pool: &PgPool,
    revision_id: Uuid,
) -> Result<Option<BlogRevision>, sqlx::Error> {
    sqlx::query_as::<_, BlogRevision>("SELECT * FROM blog_revisions WHERE id = $1")
        .bind(revision_id)
        .fetch_optional(pool)
        .await
}

// ── Media ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn create_media(
    pool: &PgPool,
    uploader_id: Uuid,
    filename: &str,
    original_filename: &str,
    title: Option<&str>,
    mime_type: &str,
    file_size: i64,
    width: Option<i32>,
    height: Option<i32>,
    storage_path: &str,
    url: &str,
) -> Result<Media, sqlx::Error> {
    sqlx::query_as::<_, Media>(
        r#"
        INSERT INTO media (id, uploader_id, filename, original_filename, title, mime_type, file_size, width, height, storage_path, url)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(uploader_id)
    .bind(filename)
    .bind(original_filename)
    .bind(title)
    .bind(mime_type)
    .bind(file_size)
    .bind(width)
    .bind(height)
    .bind(storage_path)
    .bind(url)
    .fetch_one(pool)
    .await
}

pub async fn update_media(
    pool: &PgPool,
    id: Uuid,
    req: &UpdateMediaRequest,
) -> Result<Media, sqlx::Error> {
    let existing = sqlx::query_as::<_, Media>("SELECT * FROM media WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;

    sqlx::query_as::<_, Media>(
        "UPDATE media SET title = $1, alt_text = $2, caption = $3, focal_x = $4, focal_y = $5 WHERE id = $6 RETURNING *",
    )
    .bind(req.title.as_deref().or(existing.title.as_deref()))
    .bind(req.alt_text.as_deref().or(existing.alt_text.as_deref()))
    .bind(req.caption.as_deref().or(existing.caption.as_deref()))
    .bind(req.focal_x.unwrap_or(existing.focal_x))
    .bind(req.focal_y.unwrap_or(existing.focal_y))
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn delete_media(pool: &PgPool, id: Uuid) -> Result<Option<Media>, sqlx::Error> {
    sqlx::query_as::<_, Media>("DELETE FROM media WHERE id = $1 RETURNING *")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn get_media(pool: &PgPool, id: Uuid) -> Result<Option<Media>, sqlx::Error> {
    sqlx::query_as::<_, Media>("SELECT * FROM media WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_media(
    pool: &PgPool,
    offset: i64,
    limit: i64,
) -> Result<(Vec<Media>, i64), sqlx::Error> {
    let items = sqlx::query_as::<_, Media>(
        "SELECT * FROM media ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM media")
        .fetch_one(pool)
        .await?;
    Ok((items, total.0))
}

// ── Post Meta ──────────────────────────────────────────────────────────────

pub async fn list_post_meta(pool: &PgPool, post_id: Uuid) -> Result<Vec<PostMeta>, sqlx::Error> {
    sqlx::query_as::<_, PostMeta>(
        "SELECT * FROM post_meta WHERE post_id = $1 ORDER BY meta_key ASC",
    )
    .bind(post_id)
    .fetch_all(pool)
    .await
}

pub async fn upsert_post_meta(
    pool: &PgPool,
    post_id: Uuid,
    meta_key: &str,
    meta_value: &str,
) -> Result<PostMeta, sqlx::Error> {
    sqlx::query_as::<_, PostMeta>(
        r#"
        INSERT INTO post_meta (id, post_id, meta_key, meta_value)
        VALUES (gen_random_uuid(), $1, $2, $3)
        ON CONFLICT (post_id, meta_key)
        DO UPDATE SET meta_value = EXCLUDED.meta_value, updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(post_id)
    .bind(meta_key)
    .bind(meta_value)
    .fetch_one(pool)
    .await
}

pub async fn delete_post_meta(
    pool: &PgPool,
    post_id: Uuid,
    meta_key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM post_meta WHERE post_id = $1 AND meta_key = $2")
        .bind(post_id)
        .bind(meta_key)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Analytics ─────────────────────────────────────────────────────────────

pub async fn ingest_analytics_events(
    pool: &PgPool,
    session_id: Uuid,
    user_id: Option<Uuid>,
    events: Vec<(String, String, Option<String>, serde_json::Value)>,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO analytics_sessions (id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (id) DO UPDATE SET
            user_id = COALESCE(EXCLUDED.user_id, analytics_sessions.user_id),
            updated_at = NOW()
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    for (event_type, path, referrer, metadata) in events {
        sqlx::query(
            r#"
            INSERT INTO analytics_events (session_id, event_type, path, referrer, metadata)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(session_id)
        .bind(&event_type)
        .bind(&path)
        .bind(referrer.as_deref())
        .bind(&metadata)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct AnalyticsDayRow {
    pub day: NaiveDate,
    pub page_views: i64,
    pub unique_sessions: i64,
    pub impressions: i64,
}

pub async fn analytics_time_series(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<Vec<AnalyticsDayRow>, sqlx::Error> {
    sqlx::query_as::<_, AnalyticsDayRow>(
        r#"
        SELECT
            (date_trunc('day', created_at AT TIME ZONE 'UTC'))::date AS day,
            SUM(CASE WHEN event_type = 'page_view' THEN 1 ELSE 0 END)::bigint AS page_views,
            COUNT(DISTINCT CASE WHEN event_type = 'page_view' THEN session_id END)::bigint AS unique_sessions,
            SUM(CASE WHEN event_type = 'impression' THEN 1 ELSE 0 END)::bigint AS impressions
        FROM analytics_events
        WHERE created_at >= $1 AND created_at < $2
        GROUP BY 1
        ORDER BY 1 ASC
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct AnalyticsTopPageRow {
    pub path: String,
    pub views: i64,
}

pub async fn analytics_top_pages(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<AnalyticsTopPageRow>, sqlx::Error> {
    sqlx::query_as::<_, AnalyticsTopPageRow>(
        r#"
        SELECT path, COUNT(*)::bigint AS views
        FROM analytics_events
        WHERE event_type = 'page_view' AND created_at >= $1 AND created_at < $2
        GROUP BY path
        ORDER BY views DESC
        LIMIT $3
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .bind(limit)
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct AnalyticsCtrRow {
    pub day: NaiveDate,
    pub cta_id: String,
    pub impressions: i64,
    pub clicks: i64,
}

pub async fn analytics_ctr_breakdown(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<Vec<AnalyticsCtrRow>, sqlx::Error> {
    sqlx::query_as::<_, AnalyticsCtrRow>(
        r#"
        SELECT
            (date_trunc('day', created_at AT TIME ZONE 'UTC'))::date AS day,
            COALESCE(metadata->>'cta_id', '') AS cta_id,
            SUM(CASE WHEN event_type = 'impression' THEN 1 ELSE 0 END)::bigint AS impressions,
            SUM(CASE WHEN event_type = 'click' THEN 1 ELSE 0 END)::bigint AS clicks
        FROM analytics_events
        WHERE created_at >= $1 AND created_at < $2
          AND event_type IN ('impression', 'click')
          AND COALESCE(metadata->>'cta_id', '') <> ''
        GROUP BY 1, 2
        ORDER BY 1 ASC, 2 ASC
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_all(pool)
    .await
}

pub async fn analytics_totals(
    pool: &PgPool,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> Result<(i64, i64, i64), sqlx::Error> {
    let row: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE event_type = 'page_view')::bigint,
            COUNT(DISTINCT session_id) FILTER (WHERE event_type = 'page_view')::bigint,
            COUNT(*) FILTER (WHERE event_type = 'impression')::bigint
        FROM analytics_events
        WHERE created_at >= $1 AND created_at < $2
        "#,
    )
    .bind(start)
    .bind(end_exclusive)
    .fetch_one(pool)
    .await?;
    Ok(row)
}
```

#### `backend/src/config.rs`

```rust
// backend/src/config.rs
use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub refresh_token_expiration_days: i64,
    pub port: u16,
    pub frontend_url: String,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub upload_dir: String,
    pub api_url: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub app_url: String,
    pub app_env: String,
    pub cors_allowed_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let frontend_url =
            env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
        let cors_allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| frontend_url.clone())
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            jwt_expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .expect("JWT_EXPIRATION_HOURS must be a number"),
            refresh_token_expiration_days: env::var("REFRESH_TOKEN_EXPIRATION_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .expect("REFRESH_TOKEN_EXPIRATION_DAYS must be a number"),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .expect("PORT must be a number"),
            frontend_url,
            stripe_secret_key: env::var("STRIPE_SECRET_KEY").unwrap_or_default(),
            stripe_webhook_secret: env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default(),
            upload_dir: env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()),
            api_url: env::var("API_URL").unwrap_or_else(|_| "http://localhost:3001".to_string()),
            smtp_host: std::env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.gmail.com".to_string()),
            smtp_port: std::env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .unwrap_or(587),
            smtp_user: std::env::var("SMTP_USER").unwrap_or_default(),
            smtp_password: std::env::var("SMTP_PASSWORD").unwrap_or_default(),
            smtp_from: std::env::var("SMTP_FROM")
                .unwrap_or_else(|_| "noreply@precisionoptionsignals.com".to_string()),
            app_url: std::env::var("APP_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
            app_env: std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
            cors_allowed_origins,
        }
    }

    pub fn is_production(&self) -> bool {
        self.app_env.eq_ignore_ascii_case("production")
    }

    /// Panics in production when required secrets or URLs are missing. Call right after `from_env()`.
    pub fn assert_production_ready(&self) {
        if !self.is_production() {
            return;
        }

        let mut missing: Vec<String> = Vec::new();

        if self.database_url.trim().is_empty() {
            missing.push("DATABASE_URL".into());
        }
        if self.jwt_secret.trim().is_empty() {
            missing.push("JWT_SECRET".into());
        }
        if self.api_url.trim().is_empty() {
            missing.push("API_URL".into());
        }
        if self.frontend_url.trim().is_empty() {
            missing.push("FRONTEND_URL".into());
        }
        if self.stripe_secret_key.trim().is_empty() {
            missing.push("STRIPE_SECRET_KEY".into());
        }
        if self.stripe_webhook_secret.trim().is_empty() {
            missing.push("STRIPE_WEBHOOK_SECRET".into());
        }

        if env::var("ADMIN_EMAIL").unwrap_or_default().trim().is_empty() {
            missing.push("ADMIN_EMAIL".into());
        }
        if env::var("ADMIN_PASSWORD").unwrap_or_default().trim().is_empty() {
            missing.push("ADMIN_PASSWORD".into());
        }

        if crate::services::R2Storage::from_env().is_err() {
            missing.push(
                "R2_ACCOUNT_ID, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_BUCKET_NAME, R2_PUBLIC_URL"
                    .into(),
            );
        }

        if !missing.is_empty() {
            panic!(
                "APP_ENV=production but required configuration is missing or invalid:\n  - {}",
                missing.join("\n  - ")
            );
        }
    }
}
```

#### `backend/src/error.rs`

```rust
// backend/src/error.rs
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Authentication required")]
    Unauthorized,

    #[error("Insufficient permissions")]
    Forbidden,

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Conflict(String),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("{0}")]
    TokenReuseDetected(String),

    #[error(transparent)]
    Storage(#[from] crate::services::StorageError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            AppError::Internal(err) => {
                tracing::error!("Internal error: {err:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::Database(err) => {
                tracing::error!("Database error: {err:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
            AppError::TokenReuseDetected(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Storage(e) => match e {
                crate::services::StorageError::Config(_) => {
                    (StatusCode::BAD_REQUEST, e.to_string())
                }
                crate::services::StorageError::Upload(_) | crate::services::StorageError::Delete(_) => {
                    tracing::error!("Storage error: {e}");
                    (
                        StatusCode::BAD_GATEWAY,
                        "Storage operation failed".to_string(),
                    )
                }
            },
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

### 2.7 Middleware and extractors

#### `backend/src/middleware.rs`

```rust
// backend/src/middleware.rs
// Auth middleware is handled via extractors (AuthUser, AdminUser).
pub mod rate_limit;
```

#### `backend/src/middleware/rate_limit.rs`

```rust
// backend/src/middleware/rate_limit.rs
//! Per-route rate limits (IP-based via [`tower_governor::key_extractor::SmartIpKeyExtractor`]).
//!
//! **Trust note:** `SmartIpKeyExtractor` reads `X-Forwarded-For` / `Forwarded`. Only enable that
//! behavior when your reverse proxy (e.g. Railway) strips or overwrites client-supplied values.

use std::time::Duration;

use axum::body::Body;
use governor::middleware::NoOpMiddleware;
use tower_governor::{
    governor::GovernorConfigBuilder,
    key_extractor::SmartIpKeyExtractor,
    GovernorLayer,
};

type AuthGovernorLayer = GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, Body>;

fn ip_layer(period: Duration, burst: u32) -> AuthGovernorLayer {
    let mut b = GovernorConfigBuilder::default();
    b.period(period);
    b.burst_size(burst);
    GovernorLayer::new(b.key_extractor(SmartIpKeyExtractor).finish().expect("non-zero quota"))
}

/// ~5 requests per minute per IP (burst 5, one token every 12s).
pub fn login_layer() -> AuthGovernorLayer {
    ip_layer(Duration::from_secs(12), 5)
}

/// 10 requests per hour per IP (burst 10, one token every 360s).
pub fn register_layer() -> AuthGovernorLayer {
    ip_layer(Duration::from_secs(360), 10)
}

/// 3 requests per hour per IP (burst 3, one token every 1200s).
pub fn forgot_password_layer() -> AuthGovernorLayer {
    ip_layer(Duration::from_secs(1200), 3)
}
```

#### `backend/src/extractors.rs`

```rust
// backend/src/extractors.rs
use std::convert::Infallible;

use axum::{extract::FromRequestParts, http::request::Parts};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::AppError, AppState};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

pub struct AuthUser {
    pub user_id: Uuid,
    pub role: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        Ok(AuthUser {
            user_id: token_data.claims.sub,
            role: token_data.claims.role,
        })
    }
}

pub struct AdminUser {
    pub user_id: Uuid,
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;

        if auth_user.role != "admin" {
            return Err(AppError::Forbidden);
        }

        Ok(AdminUser {
            user_id: auth_user.user_id,
        })
    }
}

/// Bearer JWT if present and valid; otherwise `user_id: None` (for optional auth on public endpoints).
pub struct OptionalAuthUser {
    pub user_id: Option<Uuid>,
}

impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user_id = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .and_then(|token| {
                decode::<Claims>(
                    token,
                    &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
                    &Validation::default(),
                )
                .ok()
                .map(|t| t.claims.sub)
            });

        Ok(OptionalAuthUser { user_id })
    }
}
```

### 2.8 Email service (`backend/src/email.rs`)

**Note:** Email HTML templates are embedded as string constants in this file (no separate `templates/` directory).

```rust
// backend/src/email.rs
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use tera::{Context, Tera};

use crate::config::Config;

const BASE_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{{ subject }}</title>
  <style>
    body {
      margin: 0;
      padding: 0;
      background-color: #0a0f1c;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
      color: #e2e8f0;
      -webkit-font-smoothing: antialiased;
    }
    .wrapper {
      width: 100%;
      background-color: #0a0f1c;
      padding: 40px 0;
    }
    .container {
      max-width: 600px;
      margin: 0 auto;
      background-color: #111827;
      border-radius: 12px;
      overflow: hidden;
      border: 1px solid #1e293b;
    }
    .header {
      background: linear-gradient(135deg, #0a0f1c 0%, #1a1f3c 100%);
      padding: 32px 40px;
      text-align: center;
      border-bottom: 2px solid #0fa4af;
    }
    .logo {
      font-size: 28px;
      font-weight: 800;
      color: #0fa4af;
      letter-spacing: -0.5px;
      text-decoration: none;
    }
    .body {
      padding: 40px;
    }
    .body h1 {
      font-size: 24px;
      font-weight: 700;
      color: #f1f5f9;
      margin: 0 0 16px 0;
    }
    .body p {
      font-size: 16px;
      line-height: 1.6;
      color: #94a3b8;
      margin: 0 0 16px 0;
    }
    .cta-wrapper {
      text-align: center;
      margin: 32px 0;
    }
    .cta-button {
      display: inline-block;
      background-color: #0fa4af;
      color: #ffffff !important;
      text-decoration: none;
      padding: 14px 40px;
      border-radius: 8px;
      font-size: 16px;
      font-weight: 600;
      letter-spacing: 0.3px;
    }
    .note {
      background-color: #0f172a;
      border-left: 3px solid #0fa4af;
      padding: 14px 18px;
      border-radius: 0 6px 6px 0;
      margin: 24px 0;
    }
    .note p {
      font-size: 14px;
      color: #64748b;
      margin: 0;
    }
    .divider {
      height: 1px;
      background-color: #1e293b;
      margin: 24px 0;
    }
    .footer {
      background-color: #0a0f1c;
      padding: 24px 40px;
      text-align: center;
      border-top: 1px solid #1e293b;
    }
    .footer p {
      font-size: 12px;
      color: #475569;
      margin: 0 0 8px 0;
      line-height: 1.5;
    }
    .footer a {
      color: #0fa4af;
      text-decoration: none;
    }
  </style>
</head>
<body>
  <div class="wrapper">
    <div class="container">
      <div class="header">
        <a href="{{ app_url }}" class="logo">Precision Options Signals</a>
      </div>
      <div class="body">
        {% block content %}{% endblock content %}
      </div>
      <div class="footer">
        <p>&copy; {{ year }} Precision Options Signals. All rights reserved.</p>
        <p>If you no longer wish to receive these emails, you can <a href="{{ app_url }}/settings">manage your preferences</a>.</p>
      </div>
    </div>
  </div>
</body>
</html>"#;

const PASSWORD_RESET_TEMPLATE: &str = r#"{% extends "base" %}
{% block content %}
<h1>Reset Your Password</h1>
<p>Hi {{ name }},</p>
<p>We received a request to reset the password for your Precision Options Signals account. Click the button below to choose a new password.</p>
<div class="cta-wrapper">
  <a href="{{ reset_url }}" class="cta-button">Reset Password</a>
</div>
<div class="note">
  <p>This link will expire in <strong style="color: #e2e8f0;">1 hour</strong>. After that, you&#39;ll need to request a new password reset.</p>
</div>
<div class="divider"></div>
<p style="font-size: 14px;">If you didn&#39;t request a password reset, you can safely ignore this email. Your password will remain unchanged.</p>
<p style="font-size: 13px; color: #475569;">If the button above doesn&#39;t work, copy and paste the following URL into your browser:</p>
<p style="font-size: 13px; color: #0fa4af; word-break: break-all;">{{ reset_url }}</p>
{% endblock content %}"#;

const WELCOME_TEMPLATE: &str = r#"{% extends "base" %}
{% block content %}
<h1>Welcome to Precision Options Signals!</h1>
<p>Hi {{ name }},</p>
<p>Thanks for creating your account. We&#39;re excited to have you on board!</p>
<p>Precision Options Signals gives you access to professional swing trading analysis, real-time alerts, and a community of traders dedicated to consistent results.</p>
<div class="cta-wrapper">
  <a href="{{ app_url }}" class="cta-button">Get Started</a>
</div>
<div class="divider"></div>
<p>Here&#39;s what you can do next:</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">1.</strong> Explore our latest market analysis on the blog</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">2.</strong> Subscribe to a plan for full access to trade alerts</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">3.</strong> Set up your profile and notification preferences</p>
<div class="divider"></div>
<p style="font-size: 14px;">Questions? Just reply to this email &mdash; we&#39;d love to hear from you.</p>
{% endblock content %}"#;

const SUBSCRIPTION_CONFIRMATION_TEMPLATE: &str = r#"{% extends "base" %}
{% block content %}
<h1>Subscription Confirmed!</h1>
<p>Hi {{ name }},</p>
<p>Great news &mdash; your <strong style="color: #0fa4af;">{{ plan_name }}</strong> subscription is now active!</p>
<p>You now have full access to all premium features including real-time trade alerts, detailed analysis, and exclusive member content.</p>
<div class="cta-wrapper">
  <a href="{{ app_url }}/member" class="cta-button">Go to Dashboard</a>
</div>
<div class="divider"></div>
<p>Your subscription includes:</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">&#10003;</strong> Real-time swing trade alerts</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">&#10003;</strong> Detailed entry, stop-loss, and target levels</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">&#10003;</strong> Market analysis and commentary</p>
<p style="margin-bottom: 8px;"><strong style="color: #f1f5f9;">&#10003;</strong> Members-only content and resources</p>
<div class="note">
  <p>You can manage your subscription anytime from your <a href="{{ app_url }}/member" style="color: #0fa4af; text-decoration: none;">account dashboard</a>.</p>
</div>
{% endblock content %}"#;

const SUBSCRIPTION_CANCELLED_TEMPLATE: &str = r#"{% extends "base" %}
{% block content %}
<h1>Subscription Cancelled</h1>
<p>Hi {{ name }},</p>
<p>We&#39;re sorry to see you go. Your subscription has been cancelled, but you&#39;ll continue to have access to premium features until <strong style="color: #0fa4af;">{{ end_date }}</strong>.</p>
<p>After that date, your account will revert to the free tier.</p>
<div class="divider"></div>
<p>A few things to keep in mind:</p>
<div class="note">
  <p>You can re-subscribe at any time to regain full access. Your account and preferences will be preserved.</p>
</div>
<div class="cta-wrapper">
  <a href="{{ app_url }}/member" class="cta-button">Resubscribe</a>
</div>
<div class="divider"></div>
<p style="font-size: 14px;">We&#39;d love to know how we can improve. If you have any feedback, simply reply to this email.</p>
{% endblock content %}"#;

pub struct EmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
    templates: Tera,
    app_url: String,
}

impl EmailService {
    pub fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let mailer = if config.smtp_user.is_empty() {
            // Local dev: connect without authentication
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
                .port(config.smtp_port)
                .build()
        } else {
            let creds = Credentials::new(config.smtp_user.clone(), config.smtp_password.clone());
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)?
                .port(config.smtp_port)
                .credentials(creds)
                .build()
        };

        let mut templates = Tera::default();
        templates.add_raw_template("base", BASE_TEMPLATE)?;
        templates.add_raw_template("password_reset", PASSWORD_RESET_TEMPLATE)?;
        templates.add_raw_template("welcome", WELCOME_TEMPLATE)?;
        templates.add_raw_template(
            "subscription_confirmation",
            SUBSCRIPTION_CONFIRMATION_TEMPLATE,
        )?;
        templates.add_raw_template("subscription_cancelled", SUBSCRIPTION_CANCELLED_TEMPLATE)?;

        Ok(Self {
            mailer,
            from: config.smtp_from.clone(),
            templates,
            app_url: config.app_url.clone(),
        })
    }

    fn current_year() -> String {
        chrono::Utc::now().format("%Y").to_string()
    }

    fn base_context(&self) -> Context {
        let mut ctx = Context::new();
        ctx.insert("app_url", &self.app_url);
        ctx.insert("year", &Self::current_year());
        ctx
    }

    async fn send_email(
        &self,
        to_email: &str,
        to_name: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let email = Message::builder()
            .from(self.from.parse()?)
            .to(format!("{to_name} <{to_email}>").parse()?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_string())?;

        self.mailer.send(email).await?;
        tracing::info!("Email sent to {to_email}: {subject}");
        Ok(())
    }

    pub async fn send_password_reset(
        &self,
        to_email: &str,
        to_name: &str,
        reset_token: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let reset_url = format!(
            "{}/admin/reset-password?token={}",
            self.app_url, reset_token
        );

        let mut ctx = self.base_context();
        ctx.insert("subject", "Reset Your Password");
        ctx.insert("name", to_name);
        ctx.insert("reset_url", &reset_url);

        let html = self.templates.render("password_reset", &ctx)?;
        self.send_email(
            to_email,
            to_name,
            "Reset Your Password — Precision Options Signals",
            &html,
        )
        .await
    }

    pub async fn send_welcome(
        &self,
        to_email: &str,
        to_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = self.base_context();
        ctx.insert("subject", "Welcome to Precision Options Signals");
        ctx.insert("name", to_name);

        let html = self.templates.render("welcome", &ctx)?;
        self.send_email(
            to_email,
            to_name,
            "Welcome to Precision Options Signals!",
            &html,
        )
        .await
    }

    pub async fn send_subscription_confirmation(
        &self,
        to_email: &str,
        to_name: &str,
        plan_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = self.base_context();
        ctx.insert("subject", "Subscription Confirmed");
        ctx.insert("name", to_name);
        ctx.insert("plan_name", plan_name);

        let html = self.templates.render("subscription_confirmation", &ctx)?;
        self.send_email(
            to_email,
            to_name,
            "Your Subscription is Active — Precision Options Signals",
            &html,
        )
        .await
    }

    pub async fn send_subscription_cancelled(
        &self,
        to_email: &str,
        to_name: &str,
        end_date: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut ctx = self.base_context();
        ctx.insert("subject", "Subscription Cancelled");
        ctx.insert("name", to_name);
        ctx.insert("end_date", end_date);

        let html = self.templates.render("subscription_cancelled", &ctx)?;
        self.send_email(
            to_email,
            to_name,
            "Your Subscription Has Been Cancelled — Precision Options Signals",
            &html,
        )
        .await
    }
}
```

### 2.9 Stripe integration and webhooks

#### `backend/src/stripe_api.rs`

```rust
// backend/src/stripe_api.rs
//! Stripe API helpers (billing portal, subscription updates).

use stripe_rust::{
    BillingPortalSession, Client, CreateBillingPortalSession, CustomerId, Subscription,
    SubscriptionId, UpdateSubscription,
};

use crate::{
    error::{AppError, AppResult},
    AppState,
};

fn map_stripe(e: stripe_rust::StripeError) -> AppError {
    AppError::BadRequest(format!("Stripe: {e}"))
}

pub fn client(state: &AppState) -> AppResult<Client> {
    if state.config.stripe_secret_key.is_empty() {
        return Err(AppError::BadRequest(
            "Stripe is not configured (missing STRIPE_SECRET_KEY)".to_string(),
        ));
    }
    Ok(Client::new(&state.config.stripe_secret_key))
}

pub async fn create_billing_portal_session(
    state: &AppState,
    stripe_customer_id: &str,
    return_url: &str,
) -> AppResult<String> {
    let c = client(state)?;
    let customer: CustomerId = stripe_customer_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid Stripe customer id".to_string()))?;

    let mut params = CreateBillingPortalSession::new(customer);
    params.return_url = Some(return_url);

    let session = BillingPortalSession::create(&c, params)
        .await
        .map_err(map_stripe)?;

    Ok(session.url)
}

pub async fn set_subscription_cancel_at_period_end(
    state: &AppState,
    stripe_subscription_id: &str,
    cancel: bool,
) -> AppResult<()> {
    let c = client(state)?;
    let sid: SubscriptionId = stripe_subscription_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid Stripe subscription id".to_string()))?;

    let mut params = UpdateSubscription::new();
    params.cancel_at_period_end = Some(cancel);

    Subscription::update(&c, &sid, params)
        .await
        .map_err(map_stripe)?;

    Ok(())
}
```

#### `backend/src/handlers/webhooks.rs`

```rust
// backend/src/handlers/webhooks.rs
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;

use crate::{db, models::*, AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/stripe", post(stripe_webhook))
}

async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let signature = match headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(sig) => sig.to_string(),
        None => return StatusCode::BAD_REQUEST,
    };

    if state.config.stripe_webhook_secret.is_empty() {
        tracing::warn!("Stripe webhook secret not configured");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Verify webhook signature before parsing the event payload.
    let payload = match std::str::from_utf8(&body) {
        Ok(p) => p,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    if !verify_stripe_signature(payload, &signature, &state.config.stripe_webhook_secret) {
        tracing::warn!("Rejected Stripe webhook due to invalid signature");
        return StatusCode::UNAUTHORIZED;
    }

    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(e) => e,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let Some(event_id) = event.get("id").and_then(|v| v.as_str()) else {
        tracing::warn!("Stripe event missing id");
        return StatusCode::BAD_REQUEST;
    };
    let event_type = event
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    match db::try_claim_stripe_webhook_event(&state.db, event_id, event_type).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::debug!(event_id, "Duplicate Stripe webhook event — acknowledging");
            return StatusCode::OK;
        }
        Err(e) => {
            tracing::error!("Webhook idempotency insert failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    // ~1% of webhooks: prune old idempotency rows so the table stays bounded.
    if rand::thread_rng().gen_bool(0.01) {
        match db::cleanup_old_stripe_webhook_events(&state.db).await {
            Ok(n) if n > 0 => tracing::debug!("Cleaned up {n} old processed_webhook_events rows"),
            Err(e) => tracing::warn!("Webhook idempotency cleanup failed: {e}"),
            _ => {}
        }
    }

    tracing::info!("Stripe webhook received: {event_type} ({event_id})");

    match event_type {
        "customer.subscription.created" | "customer.subscription.updated" => {
            if let Err(e) = handle_subscription_update(&state, &event).await {
                tracing::error!("Failed to handle subscription update: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        "customer.subscription.deleted" => {
            if let Err(e) = handle_subscription_deleted(&state, &event).await {
                tracing::error!("Failed to handle subscription deletion: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        "checkout.session.completed" => {
            if let Err(e) = handle_checkout_completed(&state, &event).await {
                tracing::error!("Failed to handle checkout: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        _ => {
            tracing::debug!("Unhandled webhook event: {event_type}");
        }
    }

    StatusCode::OK
}

fn verify_stripe_signature(payload: &str, signature_header: &str, secret: &str) -> bool {
    type HmacSha256 = Hmac<Sha256>;

    let mut timestamp: Option<i64> = None;
    let mut signatures: Vec<&str> = Vec::new();
    for part in signature_header.split(',') {
        let mut kv = part.splitn(2, '=');
        let key = kv.next().unwrap_or_default().trim();
        let value = kv.next().unwrap_or_default().trim();
        match key {
            "t" => timestamp = value.parse::<i64>().ok(),
            "v1" if !value.is_empty() => signatures.push(value),
            _ => {}
        }
    }

    let Some(ts) = timestamp else {
        return false;
    };
    if signatures.is_empty() {
        return false;
    }

    let now = chrono::Utc::now().timestamp();
    // Stripe recommends a 5 minute tolerance to reduce replay risk.
    if (now - ts).abs() > 300 {
        return false;
    }

    let signed_payload = format!("{ts}.{payload}");
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(signed_payload.as_bytes());
    let computed = mac.finalize().into_bytes();
    let computed_hex = computed
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    signatures
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&computed_hex))
}

async fn handle_subscription_update(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let sub = &event["data"]["object"];
    let customer_id = sub["customer"].as_str().unwrap_or_default();
    let sub_id = sub["id"].as_str().unwrap_or_default();
    let status_str = sub["status"].as_str().unwrap_or("active");

    let status = match status_str {
        "active" => SubscriptionStatus::Active,
        "canceled" => SubscriptionStatus::Canceled,
        "past_due" => SubscriptionStatus::PastDue,
        "trialing" => SubscriptionStatus::Trialing,
        "unpaid" => SubscriptionStatus::Unpaid,
        _ => SubscriptionStatus::Active,
    };

    // Determine plan from price interval
    let plan = if let Some(items) = sub["items"]["data"].as_array() {
        if let Some(first) = items.first() {
            match first["price"]["recurring"]["interval"].as_str() {
                Some("year") => SubscriptionPlan::Annual,
                _ => SubscriptionPlan::Monthly,
            }
        } else {
            SubscriptionPlan::Monthly
        }
    } else {
        SubscriptionPlan::Monthly
    };

    let period_start =
        chrono::DateTime::from_timestamp(sub["current_period_start"].as_i64().unwrap_or(0), 0)
            .unwrap_or_default();

    let period_end =
        chrono::DateTime::from_timestamp(sub["current_period_end"].as_i64().unwrap_or(0), 0)
            .unwrap_or_default();

    // Find user by stripe customer id
    if let Some(user) = db::find_user_by_stripe_customer(&state.db, customer_id).await? {
        db::upsert_subscription(
            &state.db,
            user.id,
            customer_id,
            sub_id,
            &plan,
            &status,
            period_start,
            period_end,
        )
        .await?;
    }

    Ok(())
}

async fn handle_subscription_deleted(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let sub = &event["data"]["object"];
    let sub_id = sub["id"].as_str().unwrap_or_default();

    if let Some(existing) = db::find_subscription_by_stripe_id(&state.db, sub_id).await? {
        let customer_id = &existing.stripe_customer_id;
        db::upsert_subscription(
            &state.db,
            existing.user_id,
            customer_id,
            sub_id,
            &existing.plan,
            &SubscriptionStatus::Canceled,
            existing.current_period_start,
            existing.current_period_end,
        )
        .await?;
    }

    Ok(())
}

async fn handle_checkout_completed(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = &event["data"]["object"];
    let customer_email = session["customer_details"]["email"]
        .as_str()
        .unwrap_or_default();
    let customer_id = session["customer"].as_str().unwrap_or_default();
    let sub_id = session["subscription"].as_str().unwrap_or_default();

    tracing::info!(
        "Checkout completed for {customer_email}, customer: {customer_id}, sub: {sub_id}"
    );

    // If user exists, link their subscription
    if let Some(user) = db::find_user_by_email(&state.db, customer_email).await? {
        let now = chrono::Utc::now();
        db::upsert_subscription(
            &state.db,
            user.id,
            customer_id,
            sub_id,
            &SubscriptionPlan::Monthly, // Will be corrected by subscription.updated event
            &SubscriptionStatus::Active,
            now,
            now + chrono::Duration::days(30),
        )
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::verify_stripe_signature;
    use chrono::Utc;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    fn make_signature(secret: &str, payload: &str, timestamp: i64) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("valid hmac key");
        mac.update(format!("{timestamp}.{payload}").as_bytes());
        let digest = mac.finalize().into_bytes();
        let hex = digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        format!("t={timestamp},v1={hex}")
    }

    #[test]
    fn accepts_valid_signature() {
        let secret = "whsec_test_secret";
        let payload = r#"{"type":"checkout.session.completed"}"#;
        let timestamp = Utc::now().timestamp();
        let header = make_signature(secret, payload, timestamp);
        assert!(verify_stripe_signature(payload, &header, secret));
    }

    #[test]
    fn rejects_tampered_payload() {
        let secret = "whsec_test_secret";
        let payload = r#"{"type":"checkout.session.completed"}"#;
        let timestamp = Utc::now().timestamp();
        let header = make_signature(secret, payload, timestamp);
        assert!(!verify_stripe_signature(
            r#"{"type":"customer.subscription.deleted"}"#,
            &header,
            secret
        ));
    }
}
```

### 2.10 backend/.env.example

```dotenv
// backend/.env.example
DATABASE_URL=postgres://user:password@localhost:5432/swings
# Optional in development, required in production when auto-seeding admin
APP_ENV=development
ADMIN_EMAIL=admin@example.com
ADMIN_PASSWORD=change-me
ADMIN_NAME=Admin
JWT_SECRET=your-super-secret-jwt-key-change-this
JWT_EXPIRATION_HOURS=24
REFRESH_TOKEN_EXPIRATION_DAYS=30
PORT=3001
FRONTEND_URL=http://localhost:5173
# Comma-separated, no trailing slashes. Browsers treat apex and www as different origins.
CORS_ALLOWED_ORIGINS=http://localhost:5173
# Production example (both hosts if you use www in DNS):
# CORS_ALLOWED_ORIGINS=https://precisionoptionsignals.com,https://www.precisionoptionsignals.com
STRIPE_SECRET_KEY=sk_test_xxx
STRIPE_WEBHOOK_SECRET=whsec_xxx
```

### 2.11 backend/Dockerfile

```dockerfile
// backend/Dockerfile
# Stage 1: Build
FROM rust:latest AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Build actual app
COPY . .
RUN touch src/main.rs
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/swings-api .
COPY --from=builder /app/migrations ./migrations

RUN mkdir -p uploads

EXPOSE 3001

CMD ["./swings-api"]
```

## Section 3: Database Migrations

### 3.1 `backend/migrations/001_initial.sql`

```sql
// backend/migrations/001_initial.sql
-- Custom enum types
CREATE TYPE user_role AS ENUM ('member', 'admin');
CREATE TYPE subscription_plan AS ENUM ('monthly', 'annual');
CREATE TYPE subscription_status AS ENUM ('active', 'canceled', 'past_due', 'trialing', 'unpaid');
CREATE TYPE trade_direction AS ENUM ('bullish', 'bearish');

-- Users
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    role user_role NOT NULL DEFAULT 'member',
    avatar_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users (email);

-- Refresh tokens
CREATE TABLE refresh_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refresh_tokens_hash ON refresh_tokens (token_hash);
CREATE INDEX idx_refresh_tokens_user ON refresh_tokens (user_id);

-- Subscriptions
CREATE TABLE subscriptions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stripe_customer_id TEXT NOT NULL,
    stripe_subscription_id TEXT NOT NULL UNIQUE,
    plan subscription_plan NOT NULL,
    status subscription_status NOT NULL DEFAULT 'active',
    current_period_start TIMESTAMPTZ NOT NULL,
    current_period_end TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subscriptions_user ON subscriptions (user_id);
CREATE INDEX idx_subscriptions_stripe ON subscriptions (stripe_subscription_id);
CREATE INDEX idx_subscriptions_customer ON subscriptions (stripe_customer_id);

-- Watchlists
CREATE TABLE watchlists (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    week_of DATE NOT NULL,
    video_url TEXT,
    notes TEXT,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_watchlists_week ON watchlists (week_of DESC);

-- Watchlist alerts
CREATE TABLE watchlist_alerts (
    id UUID PRIMARY KEY,
    watchlist_id UUID NOT NULL REFERENCES watchlists(id) ON DELETE CASCADE,
    ticker TEXT NOT NULL,
    direction trade_direction NOT NULL DEFAULT 'bullish',
    entry_zone TEXT NOT NULL,
    invalidation TEXT NOT NULL,
    profit_zones TEXT[] NOT NULL DEFAULT '{}',
    notes TEXT,
    chart_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alerts_watchlist ON watchlist_alerts (watchlist_id);

-- Course enrollments
CREATE TABLE course_enrollments (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    course_id TEXT NOT NULL,
    progress INTEGER NOT NULL DEFAULT 0,
    enrolled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    UNIQUE(user_id, course_id)
);

CREATE INDEX idx_enrollments_user ON course_enrollments (user_id);
```

### 3.2 `backend/migrations/002_blog.sql`

```sql
// backend/migrations/002_blog.sql
-- Blog post status enum
CREATE TYPE post_status AS ENUM ('draft', 'pending_review', 'published', 'private', 'scheduled', 'trash');

-- Media table (used by blog posts and general uploads)
CREATE TABLE media (
    id UUID PRIMARY KEY,
    uploader_id UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    filename TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    width INT,
    height INT,
    alt_text TEXT DEFAULT '',
    caption TEXT DEFAULT '',
    storage_path TEXT NOT NULL,
    url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_media_uploader ON media (uploader_id);
CREATE INDEX idx_media_mime ON media (mime_type);
CREATE INDEX idx_media_created ON media (created_at DESC);

-- Blog categories (hierarchical via parent_id)
CREATE TABLE blog_categories (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    description TEXT DEFAULT '',
    parent_id UUID REFERENCES blog_categories(id) ON DELETE SET NULL,
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_blog_categories_slug ON blog_categories (slug);
CREATE INDEX idx_blog_categories_parent ON blog_categories (parent_id);

-- Blog tags (flat)
CREATE TABLE blog_tags (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_blog_tags_slug ON blog_tags (slug);

-- Blog posts
CREATE TABLE blog_posts (
    id UUID PRIMARY KEY,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL DEFAULT '',
    content_json JSONB,
    excerpt TEXT DEFAULT '',
    featured_image_id UUID REFERENCES media(id) ON DELETE SET NULL,
    status post_status NOT NULL DEFAULT 'draft',
    visibility TEXT NOT NULL DEFAULT 'public',
    password_hash TEXT,
    is_sticky BOOLEAN NOT NULL DEFAULT FALSE,
    allow_comments BOOLEAN NOT NULL DEFAULT TRUE,
    meta_title TEXT DEFAULT '',
    meta_description TEXT DEFAULT '',
    canonical_url TEXT DEFAULT '',
    og_image_url TEXT DEFAULT '',
    reading_time_minutes INT NOT NULL DEFAULT 0,
    word_count INT NOT NULL DEFAULT 0,
    scheduled_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_blog_posts_slug ON blog_posts (slug);
CREATE INDEX idx_blog_posts_author ON blog_posts (author_id);
CREATE INDEX idx_blog_posts_status ON blog_posts (status);
CREATE INDEX idx_blog_posts_published ON blog_posts (published_at DESC);
CREATE INDEX idx_blog_posts_created ON blog_posts (created_at DESC);

-- Blog post ↔ category junction
CREATE TABLE blog_post_categories (
    post_id UUID NOT NULL REFERENCES blog_posts(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES blog_categories(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, category_id)
);

-- Blog post ↔ tag junction
CREATE TABLE blog_post_tags (
    post_id UUID NOT NULL REFERENCES blog_posts(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES blog_tags(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);

-- Blog revisions (full content snapshots)
CREATE TABLE blog_revisions (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES blog_posts(id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    content_json JSONB,
    revision_number INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_blog_revisions_post ON blog_revisions (post_id, revision_number DESC);

-- Seed a default "Uncategorized" category
INSERT INTO blog_categories (id, name, slug, description, sort_order)
VALUES ('00000000-0000-0000-0000-000000000001', 'Uncategorized', 'uncategorized', 'Default category', 0);
```

### 3.3 `backend/migrations/003_password_resets.sql`

```sql
// backend/migrations/003_password_resets.sql
-- Password reset tokens
CREATE TABLE password_reset_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_password_reset_tokens_hash ON password_reset_tokens (token_hash);
CREATE INDEX idx_password_reset_tokens_user ON password_reset_tokens (user_id);
```

### 3.4 `backend/migrations/004_media_title.sql`

```sql
// backend/migrations/004_media_title.sql
-- Add human-readable title to media items (separate from filename)
ALTER TABLE media ADD COLUMN IF NOT EXISTS title TEXT;
```

### 3.5 `backend/migrations/005_user_author_profile.sql`

```sql
// backend/migrations/005_user_author_profile.sql
-- WordPress-equivalent author profile fields
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS bio TEXT,
    ADD COLUMN IF NOT EXISTS position TEXT,
    ADD COLUMN IF NOT EXISTS website_url TEXT,
    ADD COLUMN IF NOT EXISTS twitter_url TEXT,
    ADD COLUMN IF NOT EXISTS linkedin_url TEXT,
    ADD COLUMN IF NOT EXISTS youtube_url TEXT,
    ADD COLUMN IF NOT EXISTS instagram_url TEXT;
```

### 3.6 `backend/migrations/006_post_format.sql`

```sql
// backend/migrations/006_post_format.sql
ALTER TABLE blog_posts
    ADD COLUMN IF NOT EXISTS format TEXT NOT NULL DEFAULT 'standard';
```

### 3.7 `backend/migrations/007_post_meta.sql`

```sql
// backend/migrations/007_post_meta.sql
CREATE TABLE IF NOT EXISTS post_meta (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL REFERENCES blog_posts(id) ON DELETE CASCADE,
    meta_key    TEXT NOT NULL,
    meta_value  TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_post_meta_post_id ON post_meta(post_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_post_meta_post_key ON post_meta(post_id, meta_key);
```

### 3.8 `backend/migrations/008_media_focal.sql`

```sql
// backend/migrations/008_media_focal.sql
-- Add focal point coordinates to media items
ALTER TABLE media
    ADD COLUMN IF NOT EXISTS focal_x DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    ADD COLUMN IF NOT EXISTS focal_y DOUBLE PRECISION NOT NULL DEFAULT 0.5;
```

### 3.9 `backend/migrations/009_analytics.sql`

```sql
// backend/migrations/009_analytics.sql
-- Analytics: client sessions and events (page views, impressions, clicks for CTR)

CREATE TABLE analytics_sessions (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_analytics_sessions_user ON analytics_sessions (user_id);

CREATE TABLE analytics_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES analytics_sessions(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    path TEXT NOT NULL DEFAULT '/',
    referrer TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT analytics_events_type_chk CHECK (
        event_type IN ('page_view', 'impression', 'click')
    ),
    CONSTRAINT analytics_events_path_len CHECK (char_length(path) <= 2048)
);

CREATE INDEX idx_analytics_events_created ON analytics_events (created_at DESC);
CREATE INDEX idx_analytics_events_session ON analytics_events (session_id);
CREATE INDEX idx_analytics_events_type_time ON analytics_events (event_type, created_at DESC);
CREATE INDEX idx_analytics_events_path ON analytics_events (path);
CREATE INDEX idx_analytics_events_metadata_gin ON analytics_events USING GIN (metadata);
```

### 3.10 `backend/migrations/010_normalize_user_emails.sql`

```sql
// backend/migrations/010_normalize_user_emails.sql
-- Emails are compared case-insensitively in the app; store canonical lowercase form
-- so UNIQUE(email) matches real-world addresses (e.g. Gmail).
UPDATE users SET email = lower(btrim(email));
```

### 3.11 `backend/migrations/011_courses.sql`

```sql
// backend/migrations/011_courses.sql
-- Migration 011: Full courses system with modules, lessons, and progress tracking

-- courses table
CREATE TABLE courses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    short_description TEXT DEFAULT '',
    thumbnail_url TEXT,
    trailer_video_url TEXT,
    difficulty TEXT NOT NULL DEFAULT 'beginner' CHECK (difficulty IN ('beginner', 'intermediate', 'advanced')),
    instructor_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    price_cents INTEGER NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'usd',
    is_free BOOLEAN NOT NULL DEFAULT FALSE,
    is_included_in_subscription BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    published_at TIMESTAMPTZ,
    estimated_duration_minutes INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_courses_slug ON courses (slug);
CREATE INDEX idx_courses_published ON courses (published, sort_order);
CREATE INDEX idx_courses_instructor ON courses (instructor_id);

-- course_modules table
CREATE TABLE course_modules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id UUID NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_course_modules_course ON course_modules (course_id, sort_order);

-- course_lessons table
CREATE TABLE course_lessons (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    module_id UUID NOT NULL REFERENCES course_modules(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    slug TEXT NOT NULL,
    description TEXT DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    content_json JSONB,
    video_url TEXT,
    video_duration_seconds INTEGER DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_preview BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(module_id, slug)
);

CREATE INDEX idx_course_lessons_module ON course_lessons (module_id, sort_order);

-- lesson progress tracking (enhances existing course_enrollments)
CREATE TABLE lesson_progress (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lesson_id UUID NOT NULL REFERENCES course_lessons(id) ON DELETE CASCADE,
    completed BOOLEAN NOT NULL DEFAULT FALSE,
    progress_seconds INTEGER NOT NULL DEFAULT 0,
    completed_at TIMESTAMPTZ,
    last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, lesson_id)
);

CREATE INDEX idx_lesson_progress_user ON lesson_progress (user_id);
CREATE INDEX idx_lesson_progress_lesson ON lesson_progress (lesson_id);

-- Add last_lesson_id to course_enrollments
ALTER TABLE course_enrollments ADD COLUMN last_lesson_id UUID REFERENCES course_lessons(id) ON DELETE SET NULL;
ALTER TABLE course_enrollments ADD COLUMN last_accessed_at TIMESTAMPTZ DEFAULT NOW();
```

### 3.12 `backend/migrations/012_pricing_plans.sql`

```sql
// backend/migrations/012_pricing_plans.sql
-- Migration 012: Admin-configurable pricing plans with change audit log

CREATE TABLE pricing_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT DEFAULT '',
    stripe_price_id TEXT,
    stripe_product_id TEXT,
    amount_cents INTEGER NOT NULL,
    currency TEXT NOT NULL DEFAULT 'usd',
    interval TEXT NOT NULL DEFAULT 'month' CHECK (interval IN ('month', 'year', 'one_time')),
    interval_count INTEGER NOT NULL DEFAULT 1,
    trial_days INTEGER NOT NULL DEFAULT 0,
    features JSONB NOT NULL DEFAULT '[]',
    highlight_text TEXT DEFAULT '',
    is_popular BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pricing_plans_active ON pricing_plans (is_active, sort_order);
CREATE INDEX idx_pricing_plans_slug ON pricing_plans (slug);

CREATE TABLE pricing_change_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id UUID NOT NULL REFERENCES pricing_plans(id) ON DELETE CASCADE,
    field_changed TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    changed_by UUID NOT NULL REFERENCES users(id),
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pricing_change_log_plan ON pricing_change_log (plan_id, changed_at DESC);

-- Seed default plans
INSERT INTO pricing_plans (id, name, slug, amount_cents, currency, interval, interval_count, features, is_popular, sort_order)
VALUES
    (gen_random_uuid(), 'Monthly', 'monthly', 4900, 'usd', 'month', 1, '["Weekly watchlists & trade alerts", "Full course library access", "Members-only community", "Mobile app access"]'::jsonb, false, 1),
    (gen_random_uuid(), 'Annual', 'annual', 39900, 'usd', 'year', 1, '["Everything in Monthly", "Save $189/year vs monthly", "Priority support", "Exclusive annual member content"]'::jsonb, true, 2);
```

### 3.13 `backend/migrations/013_coupons.sql`

```sql
// backend/migrations/013_coupons.sql
-- Migration 013: Full coupon system with usage tracking

CREATE TYPE discount_type AS ENUM ('percentage', 'fixed_amount', 'free_trial');

CREATE TABLE coupons (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code TEXT NOT NULL UNIQUE,
    description TEXT DEFAULT '',
    discount_type discount_type NOT NULL DEFAULT 'percentage',
    discount_value NUMERIC(10, 2) NOT NULL DEFAULT 0,
    min_purchase_cents INTEGER DEFAULT 0,
    max_discount_cents INTEGER,
    applies_to TEXT NOT NULL DEFAULT 'all' CHECK (applies_to IN ('all', 'monthly', 'annual', 'course', 'specific_plans')),
    applicable_plan_ids UUID[] DEFAULT '{}',
    applicable_course_ids UUID[] DEFAULT '{}',
    usage_limit INTEGER,
    usage_count INTEGER NOT NULL DEFAULT 0,
    per_user_limit INTEGER NOT NULL DEFAULT 1,
    starts_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    stackable BOOLEAN NOT NULL DEFAULT FALSE,
    first_purchase_only BOOLEAN NOT NULL DEFAULT FALSE,
    stripe_coupon_id TEXT,
    stripe_promotion_code_id TEXT,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_coupons_code ON coupons (code);
CREATE INDEX idx_coupons_active ON coupons (is_active, starts_at, expires_at);

CREATE TABLE coupon_usages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    coupon_id UUID NOT NULL REFERENCES coupons(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subscription_id UUID REFERENCES subscriptions(id) ON DELETE SET NULL,
    discount_applied_cents INTEGER NOT NULL DEFAULT 0,
    used_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_coupon_usages_coupon ON coupon_usages (coupon_id);
CREATE INDEX idx_coupon_usages_user ON coupon_usages (user_id);
CREATE INDEX idx_coupon_usages_coupon_user ON coupon_usages (coupon_id, user_id);
```

### 3.14 `backend/migrations/014_analytics_enhanced.sql`

```sql
// backend/migrations/014_analytics_enhanced.sql
-- Migration 014: Enhanced analytics for revenue, sales, and funnel tracking

CREATE TABLE sales_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN ('new_subscription', 'renewal', 'upgrade', 'downgrade', 'cancellation', 'refund', 'course_purchase')),
    amount_cents INTEGER NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'usd',
    plan_id UUID REFERENCES pricing_plans(id) ON DELETE SET NULL,
    coupon_id UUID REFERENCES coupons(id) ON DELETE SET NULL,
    stripe_payment_intent_id TEXT,
    stripe_invoice_id TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sales_events_user ON sales_events (user_id);
CREATE INDEX idx_sales_events_type ON sales_events (event_type, created_at DESC);
CREATE INDEX idx_sales_events_created ON sales_events (created_at DESC);
CREATE INDEX idx_sales_events_plan ON sales_events (plan_id);

CREATE TABLE monthly_revenue_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    year INTEGER NOT NULL,
    month INTEGER NOT NULL,
    mrr_cents BIGINT NOT NULL DEFAULT 0,
    arr_cents BIGINT NOT NULL DEFAULT 0,
    total_revenue_cents BIGINT NOT NULL DEFAULT 0,
    new_subscribers INTEGER NOT NULL DEFAULT 0,
    churned_subscribers INTEGER NOT NULL DEFAULT 0,
    net_subscriber_change INTEGER NOT NULL DEFAULT 0,
    avg_revenue_per_user_cents INTEGER NOT NULL DEFAULT 0,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(year, month)
);

CREATE INDEX idx_monthly_snapshots_date ON monthly_revenue_snapshots (year DESC, month DESC);
```

### 3.15 `backend/migrations/015_popups.sql`

```sql
// backend/migrations/015_popups.sql
-- Migration 015: Full popup/form builder system with submissions and event tracking

CREATE TABLE popups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    popup_type TEXT NOT NULL DEFAULT 'modal' CHECK (popup_type IN ('modal', 'slide_in', 'banner', 'fullscreen', 'floating_bar', 'inline')),
    trigger_type TEXT NOT NULL DEFAULT 'time_delay' CHECK (trigger_type IN ('on_load', 'exit_intent', 'scroll_percentage', 'time_delay', 'click', 'manual', 'inactivity')),
    trigger_config JSONB NOT NULL DEFAULT '{"delay_ms": 3000}',
    content_json JSONB NOT NULL DEFAULT '{"elements": []}',
    style_json JSONB NOT NULL DEFAULT '{"background": "#1a1a2e", "textColor": "#ffffff", "accentColor": "#0fa4af", "borderRadius": "16px", "maxWidth": "480px", "animation": "fade", "backdrop": true, "backdropColor": "rgba(0,0,0,0.6)"}',
    targeting_rules JSONB NOT NULL DEFAULT '{"pages": ["*"], "devices": ["desktop", "mobile", "tablet"], "userStatus": ["all"]}',
    display_frequency TEXT NOT NULL DEFAULT 'once_per_session' CHECK (display_frequency IN ('every_time', 'once_per_session', 'once_ever', 'custom')),
    frequency_config JSONB NOT NULL DEFAULT '{}',
    success_message TEXT DEFAULT 'Thank you!',
    redirect_url TEXT,
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    starts_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    priority INTEGER NOT NULL DEFAULT 0,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_popups_active ON popups (is_active, priority DESC);

CREATE TABLE popup_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    popup_id UUID NOT NULL REFERENCES popups(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    session_id UUID,
    form_data JSONB NOT NULL DEFAULT '{}',
    ip_address TEXT,
    user_agent TEXT,
    page_url TEXT,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_popup_submissions_popup ON popup_submissions (popup_id, submitted_at DESC);
CREATE INDEX idx_popup_submissions_user ON popup_submissions (user_id);

CREATE TABLE popup_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    popup_id UUID NOT NULL REFERENCES popups(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN ('impression', 'close', 'submit', 'click')),
    session_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_popup_events_popup ON popup_events (popup_id, event_type, created_at DESC);
```

### 3.16 `backend/migrations/016_blog_trash_meta.sql`

```sql
// backend/migrations/016_blog_trash_meta.sql
-- Remember status before moving to trash (WordPress-style restore)
ALTER TABLE blog_posts
    ADD COLUMN pre_trash_status post_status NULL,
    ADD COLUMN trashed_at TIMESTAMPTZ NULL;

CREATE INDEX idx_blog_posts_trashed ON blog_posts (trashed_at DESC) WHERE status = 'trash';
```

### 3.17 `backend/migrations/017_webhook_idempotency.sql`

```sql
// backend/migrations/017_webhook_idempotency.sql
-- Prevent duplicate processing of Stripe webhook events
CREATE TABLE IF NOT EXISTS processed_webhook_events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_processed_webhook_events_date
    ON processed_webhook_events(processed_at);
```

### 3.18 `backend/migrations/018_refresh_token_families.sql`

```sql
// backend/migrations/018_refresh_token_families.sql
-- Refresh token rotation: family tracking + single-use detection
ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS family_id UUID NOT NULL DEFAULT gen_random_uuid();
ALTER TABLE refresh_tokens ADD COLUMN IF NOT EXISTS used BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_family_id
    ON refresh_tokens(family_id);
```

## Section 4: Configuration

### 4.1 `backend/render.yaml`

```yaml
// backend/render.yaml
services:
  - type: web
    name: swings-api
    runtime: docker
    dockerContext: .
    dockerfilePath: ./Dockerfile
    envVars:
      - key: DATABASE_URL
        fromDatabase:
          name: swings-db
          property: connectionString
      - key: JWT_SECRET
        generateValue: true
      - key: JWT_EXPIRATION_HOURS
        value: 24
      - key: PORT
        value: 3001
      - key: FRONTEND_URL
        value: https://swings-gamma.vercel.app
      - key: UPLOAD_DIR
        value: ./uploads

databases:
  - name: swings-db
    plan: free
```

### 4.2 Environment template files (project root)

#### `.env.example`

```dotenv
// .env.example
# Stripe API Keys
# Get these from https://dashboard.stripe.com/apikeys
STRIPE_SECRET_KEY=sk_test_your_secret_key_here
PUBLIC_STRIPE_PUBLISHABLE_KEY=pk_test_your_publishable_key_here

# Stripe Price IDs
# Create products and prices in Stripe Dashboard
PUBLIC_STRIPE_MONTHLY_PRICE_ID=price_monthly_id_here
PUBLIC_STRIPE_ANNUAL_PRICE_ID=price_annual_id_here

# App URL (for Stripe redirects). Production: https://precisionoptionsignals.com
PUBLIC_APP_URL=http://localhost:5173

# Public Rust API origin for **server-side** fetches (blog loaders, sitemap, etc.) during `pnpm build` / Vercel SSR.
# Browser bundles use same-origin `/api` (see `vercel.json` rewrites) so visitors are not blocked by CORS.
# VITE_API_URL=https://your-api.onrender.com

# Set to 1 only when you need the real service worker while running `pnpm dev` (default: SW off in dev).
# Must match `svelte.config.js` / `src/hooks.client.ts` policy.
# PUBLIC_SERVICE_WORKER_IN_DEV=1
```

**File does not exist:** `.env.local.example`

### 4.3 Docker Compose

#### `docker-compose.yml`

```yaml
// docker-compose.yml
services:
  db:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_USER: swings
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-swings_secret}
      POSTGRES_DB: swings
    volumes:
      - pgdata:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U swings -d swings"]
      interval: 5s
      timeout: 5s
      retries: 5

  api:
    build:
      context: ./backend
      dockerfile: Dockerfile
    restart: unless-stopped
    depends_on:
      db:
        condition: service_healthy
    environment:
      DATABASE_URL: postgres://swings:${POSTGRES_PASSWORD:-swings_secret}@db:5432/swings
      JWT_SECRET: ${JWT_SECRET:-explosive-swings-jwt-secret-key-2026}
      JWT_EXPIRATION_HOURS: 24
      REFRESH_TOKEN_EXPIRATION_DAYS: 30
      PORT: 3001
      FRONTEND_URL: ${FRONTEND_URL:-http://localhost:5173}
      STRIPE_SECRET_KEY: ${STRIPE_SECRET_KEY:-}
      STRIPE_WEBHOOK_SECRET: ${STRIPE_WEBHOOK_SECRET:-}
      UPLOAD_DIR: /app/uploads
      API_URL: ${API_URL:-http://localhost:3001}
    ports:
      - "3001:3001"
    volumes:
      - uploads:/app/uploads

volumes:
  pgdata:
  uploads:
```

### 4.4 `vercel.json`

```json
// vercel.json
{
	"rewrites": [
		{
			"source": "/api/:path*",
			"destination": "https://swings-api.onrender.com/api/:path*"
		},
		{
			"source": "/uploads/:path*",
			"destination": "https://swings-api.onrender.com/uploads/:path*"
		}
	]
}
```

### 4.5 `.gitignore`

```gitignore
// .gitignore
node_modules

# Output
.output
.vercel
.netlify
.wrangler
/.svelte-kit
/build

# OS
.DS_Store
Thumbs.db

# Env
.env
.env.*
!.env.example
!.env.test

# Vite
vite.config.js.timestamp-*
vite.config.ts.timestamp-*
# Playwright
test-results
.env*.local
```

### 4.6 README files

#### `README.md`

````markdown
// README.md

# Precision Options Signals — SvelteKit Landing Page

Premium stock alert service landing page built with **SvelteKit** (Svelte 5), **TailwindCSS v4**, **GSAP**, and **Stripe** integration.

## 🚀 Features

- ✅ **Svelte 5 Runes** — Modern reactive patterns (`$state`, `$derived`, `$effect`, `$props`)
- ✅ **Stripe Checkout** — Subscription payments for monthly/annual plans
- ✅ **Courses System** — Full course listing and detail pages
- ✅ **GSAP Animations** — Cinematic scroll-triggered animations
- ✅ **Responsive Design** — Mobile-first with TailwindCSS v4
- ✅ **TypeScript Strict Mode** — Zero `any`, full type safety
- ✅ **Traders Modal** — Interactive modal with grid and profile views
- ✅ **SEO Optimized** — Meta tags, semantic HTML, accessibility

## 📋 Prerequisites

- **Node.js** 18+
- **pnpm** (required — not npm or yarn)
- **Stripe Account** (for payment processing)

## 🛠️ Setup

### 1. Install Dependencies

```bash
pnpm install
```
````

### 2. Configure Environment Variables

Create a `.env` file in the root directory:

```bash
cp .env.example .env
```

Edit `.env` and add your Stripe keys:

```env
# Stripe API Keys (get from https://dashboard.stripe.com/apikeys)
STRIPE_SECRET_KEY=sk_test_your_secret_key_here
PUBLIC_STRIPE_PUBLISHABLE_KEY=pk_test_your_publishable_key_here

# Stripe Price IDs (create products in Stripe Dashboard)
STRIPE_MONTHLY_PRICE_ID=price_monthly_id_here
STRIPE_ANNUAL_PRICE_ID=price_annual_id_here

# App URL (for Stripe redirects)
PUBLIC_APP_URL=http://localhost:5173
```

### 3. Set Up Stripe Products

1. Go to [Stripe Dashboard](https://dashboard.stripe.com/)
2. Create two **Products**:
   - **Monthly Plan** — $49/month recurring
   - **Annual Plan** — $399/year recurring
3. Copy the **Price IDs** and add them to your `.env` file

### 4. Run Development Server

```bash
pnpm run dev
```

Open [http://localhost:5173](http://localhost:5173) in your browser.

## 📁 Project Structure

```
src/
├── routes/
│   ├── +page.svelte              # Landing page
│   ├── +layout.svelte            # Root layout (nav + footer)
│   ├── success/+page.svelte      # Stripe success page
│   ├── courses/+page.svelte      # Courses listing
│   ├── courses/[slug]/+page.svelte  # Individual course pages
│   └── api/
│       └── create-checkout-session/+server.ts  # Stripe API
├── lib/
│   ├── components/
│   │   ├── landing/              # Landing page sections
│   │   ├── traders/              # Traders modal system
│   │   └── ui/                   # Reusable UI components
│   ├── data/                     # Static data (traders, courses, pricing)
│   ├── stores/                   # Svelte 5 reactive stores
│   └── utils/                    # Utilities (Stripe helpers)
└── app.css                       # Global styles + Tailwind
```

## 🎨 Tech Stack

- **Framework**: SvelteKit (Svelte 5)
- **Styling**: TailwindCSS v4
- **Animations**: GSAP + ScrollTrigger
- **Icons**: Phosphor Icons
- **Payments**: Stripe
- **Fonts**: Montserrat + Inter (Google Fonts)
- **Package Manager**: pnpm

## 🧪 Testing

```bash
# Type checking
pnpm run check

# Build for production
pnpm run build

# Preview production build
pnpm run preview

# SEO policy checks
pnpm run ci:seo
```

SEO operational guidance lives in `SEO_RUNBOOK.md`.

## 🚢 Deployment

### Environment Variables (Production)

Set these in your deployment platform (Vercel, Netlify, etc.):

```
STRIPE_SECRET_KEY=sk_live_...
PUBLIC_STRIPE_PUBLISHABLE_KEY=pk_live_...
STRIPE_MONTHLY_PRICE_ID=price_...
STRIPE_ANNUAL_PRICE_ID=price_...
PUBLIC_APP_URL=https://your-domain.com
```

### Build Command

```bash
pnpm run build
```

### Recommended Adapters

- **Vercel**: `@sveltejs/adapter-vercel`
- **Netlify**: `@sveltejs/adapter-netlify`
- **Node**: `@sveltejs/adapter-node`

## 📝 Key Pages

- `/` — Landing page with all sections
- `/courses` — Course listing
- `/courses/beginning-options-trading` — Beginner course
- `/courses/options-trading-101` — Intermediate course
- `/success` — Post-checkout success page

## 🎯 Stripe Webhook Setup (Optional)

For production, set up webhooks to handle subscription events:

1. Go to Stripe Dashboard → Developers → Webhooks
2. Add endpoint: `https://your-domain.com/api/webhooks/stripe`
3. Select events: `checkout.session.completed`, `customer.subscription.updated`, etc.
4. Add webhook secret to `.env`: `STRIPE_WEBHOOK_SECRET=whsec_...`

## 🔒 Security Notes

- Never commit `.env` to version control
- Use Stripe test keys in development
- Switch to live keys only in production
- Validate all webhook signatures

## 📚 Documentation

- [SvelteKit Docs](https://svelte.dev/docs/kit)
- [Svelte 5 Runes](https://svelte.dev/docs/svelte/$state)
- [Stripe Checkout](https://stripe.com/docs/payments/checkout)
- [GSAP ScrollTrigger](https://greensock.com/docs/v3/Plugins/ScrollTrigger)
- [TailwindCSS v4](https://tailwindcss.com/docs)

## 📄 License

Private — All Rights Reserved

````

#### `backend/README.md`

```markdown
// backend/README.md
# Swings API - Rust Backend

Axum + Tokio + SQLx + PostgreSQL backend for the Precision Options Signals membership platform.

## Prerequisites

- **Rust** (1.75+): https://rustup.rs
- **PostgreSQL** (15+): running locally or via Docker
- **sqlx-cli** (for migrations): `cargo install sqlx-cli --no-default-features --features postgres`

## Setup

```bash
# 1. Copy env file and fill in your values
cp .env.example .env

# 2. Create the database
createdb swings

# 3. Run migrations
sqlx migrate run

# 4. Start the server
cargo run
````

The API will start on `http://localhost:3001` by default.

## Environment Variables

| Variable                        | Required                     | Default                 | Description                                                                                                                                                          |
| ------------------------------- | ---------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `DATABASE_URL`                  | Yes                          | -                       | PostgreSQL connection string. For Neon, include `sslmode=require`.                                                                                                   |
| `JWT_SECRET`                    | Yes                          | -                       | Secret key for JWT signing                                                                                                                                           |
| `JWT_EXPIRATION_HOURS`          | No                           | `24`                    | Access token lifetime                                                                                                                                                |
| `REFRESH_TOKEN_EXPIRATION_DAYS` | No                           | `30`                    | Refresh token lifetime                                                                                                                                               |
| `PORT`                          | No                           | `3001`                  | Server port                                                                                                                                                          |
| `FRONTEND_URL`                  | No                           | `http://localhost:5173` | Frontend URL for CORS                                                                                                                                                |
| `CORS_ALLOWED_ORIGINS`          | No                           | `FRONTEND_URL`          | Comma-separated **exact** browser origins (scheme + host + port). No trailing `/`. List every origin browsers use (e.g. apex and `www` separately if both are live). |
| `API_URL`                       | No                           | `http://localhost:3001` | Public base URL of this API (used for absolute media URLs in local upload mode).                                                                                     |
| `UPLOAD_DIR`                    | No                           | `./uploads`             | Local media directory when R2 is not configured.                                                                                                                     |
| `STRIPE_SECRET_KEY`             | No                           | -                       | Stripe secret key                                                                                                                                                    |
| `STRIPE_WEBHOOK_SECRET`         | No                           | -                       | Stripe webhook signing secret                                                                                                                                        |
| `APP_ENV`                       | No                           | `development`           | Set to `production` to enforce production-only guards (admin seed, R2, Stripe, JWT, URLs, etc.).                                                                     |
| `ADMIN_EMAIL`                   | Dev optional / Prod required | -                       | Seed admin email                                                                                                                                                     |
| `ADMIN_PASSWORD`                | Dev optional / Prod required | -                       | Seed admin password                                                                                                                                                  |
| `ADMIN_NAME`                    | No                           | `Admin`                 | Seed admin display name                                                                                                                                              |
| `R2_ACCOUNT_ID`                 | Prod required                | -                       | Cloudflare account id for S3-compatible endpoint                                                                                                                     |
| `R2_ACCESS_KEY_ID`              | Prod required                | -                       | R2 API token access key                                                                                                                                              |
| `R2_SECRET_ACCESS_KEY`          | Prod required                | -                       | R2 API token secret                                                                                                                                                  |
| `R2_BUCKET_NAME`                | Prod required                | -                       | Bucket name                                                                                                                                                          |
| `R2_PUBLIC_URL`                 | Prod required                | -                       | Public base URL for objects (no trailing `/`)                                                                                                                        |

### CORS

Allowed origins come **only** from `CORS_ALLOWED_ORIGINS` and/or the default of `FRONTEND_URL`. Preflight accepts **any** request header so browser extensions (Sentry, etc.) cannot break `OPTIONS`.

### Media (R2 vs local)

When all `R2_*` variables are set, uploads go to Cloudflare R2 and `/uploads` static serving is skipped in **production**. If any R2 variable is missing, the API logs a warning and stores files under `UPLOAD_DIR` (and serves them at `/uploads/...`).

### Rate limiting

`POST /api/auth/login`, `POST /api/auth/register`, and `POST /api/auth/forgot-password` are rate-limited per client IP using `tower-governor`. The stack uses **`SmartIpKeyExtractor`**, which reads `X-Forwarded-For` / `Forwarded` when present. Deploy behind a trusted reverse proxy (e.g. Railway) that sets those headers from the real client; otherwise clients could spoof IPs.

### Production checklist

With `APP_ENV=production`, the process **panics on startup** unless `DATABASE_URL`, `JWT_SECRET`, `ADMIN_EMAIL`, `ADMIN_PASSWORD`, `API_URL`, `FRONTEND_URL`, `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, and **all** `R2_*` variables are set and non-empty, and R2 client initialization succeeds.

### Database pool (Neon)

The API uses SQLx pool options suited to serverless Postgres: bounded acquire time, idle timeout, and max connection lifetime. Keep `sslmode=require` in `DATABASE_URL` for Neon.

## API Endpoints

### Auth

- `POST /api/auth/register` - Create account
- `POST /api/auth/login` - Sign in
- `POST /api/auth/refresh` - Refresh tokens
- `GET /api/auth/me` - Get current user
- `POST /api/auth/logout` - Sign out (invalidates refresh tokens)

### Member (requires auth)

- `GET /api/member/profile` - Get profile
- `PUT /api/member/profile` - Update profile
- `GET /api/member/subscription` - Get subscription status
- `GET /api/member/watchlists` - List published watchlists
- `GET /api/member/watchlists/:id` - Get watchlist with alerts
- `GET /api/member/courses` - Get enrolled courses
- `PUT /api/member/courses/:id/progress` - Update course progress

### Admin (requires admin role)

- `GET /api/admin/stats` - Dashboard statistics
- `GET /api/admin/members` - List all members (paginated)
- `GET /api/admin/members/:id` - Get member details
- `PUT /api/admin/members/:id/role` - Update member role
- `DELETE /api/admin/members/:id` - Delete member
- `GET /api/admin/watchlists` - List all watchlists (paginated)
- `POST /api/admin/watchlists` - Create watchlist
- `GET /api/admin/watchlists/:id` - Get watchlist with alerts
- `PUT /api/admin/watchlists/:id` - Update watchlist
- `DELETE /api/admin/watchlists/:id` - Delete watchlist
- `GET /api/admin/watchlists/:id/alerts` - List alerts
- `POST /api/admin/watchlists/:id/alerts` - Create alert
- `PUT /api/admin/alerts/:id` - Update alert
- `DELETE /api/admin/alerts/:id` - Delete alert

### Webhooks

- `POST /api/webhooks/stripe` - Stripe webhook handler

## Database Schema

- `users` - Members and admins with argon2 password hashing
- `refresh_tokens` - JWT refresh token storage with rotation
- `subscriptions` - Stripe subscription sync
- `watchlists` - Weekly watchlists with publish control
- `watchlist_alerts` - Trade alerts (ticker, entry, invalidation, targets)
- `course_enrollments` - Course progress tracking

## Creating an Admin User

After registering via the frontend, promote a user to admin via psql:

```sql
UPDATE users SET role = 'admin' WHERE email = 'your@email.com';
```

```

```
