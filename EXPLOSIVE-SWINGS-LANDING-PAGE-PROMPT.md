# Explosive Swings Landing Page — SvelteKit 5 Implementation Prompt

## Project Overview

Build a production-grade, SEO-optimized landing page for **Explosive Swings**, a premium stock alert service by Billy Ribeiro. This is a conversion-focused marketing page for a trading education brand serving 18,000+ active traders.

The page must be built in **SvelteKit (latest, Feb 2026)** using **Svelte 5 runes** exclusively, with **GSAP** for scroll-triggered and entrance animations, **Svelte's built-in transitions** for component-level motion, **Phosphor Icons** via `@nickvdh/phosphor-icons-svelte` (NOT Lucide — never use Lucide), **TailwindCSS v4** for utility styling, and a custom **Revolution Typography System** (provided below).

---

## Hard Technical Requirements

### Stack & Tooling

- **Framework**: SvelteKit latest (Feb 2026 release) with Svelte 5
- **Language**: TypeScript strict mode — zero `any`, zero warnings, zero errors
- **Package Manager**: `pnpm` only — never npm or yarn
- **Styling**: TailwindCSS v4 + CSS custom properties for brand tokens
- **Icons**: Phosphor Icons via `@nickvdh/phosphor-icons-svelte` — NEVER Lucide (they look childish)
- **Animations**: GSAP (`gsap` + `ScrollTrigger` plugin) for scroll-based reveals and hero entrance, Svelte 5 built-in `transition:`, `in:`, `out:` directives for component-level motion (modals, tabs, etc.)
- **Fonts**: Google Fonts — Montserrat (headings) + Inter (body/UI) loaded via `<link>` in `app.html`

### Svelte 5 Runes — Mandatory Patterns

Every component MUST use Svelte 5 runes. Do NOT use any Svelte 4 patterns.

```svelte
<!-- ✅ CORRECT — Svelte 5 -->
<script lang="ts">
  import { type Snippet } from 'svelte';

  // Props via $props() rune — NEVER export let
  interface Props {
    title: string;
    variant?: 'primary' | 'ghost';
    children: Snippet;
  }
  let { title, variant = 'primary', children }: Props = $props();

  // Reactive state via $state rune — NEVER let x = value
  let isOpen = $state(false);
  let count = $state(0);

  // Derived values via $derived rune — NEVER $: reactive statements
  let isActive = $derived(count > 0);
  let label = $derived.by(() => {
    if (count === 0) return 'Start';
    return `Count: ${count}`;
  });

  // Side effects via $effect rune — NEVER $: for side effects
  $effect(() => {
    if (isOpen) {
      document.body.style.overflow = 'hidden';
      return () => {
        document.body.style.overflow = '';
      };
    }
  });
</script>

<!-- ✅ CORRECT — Snippets, not slots -->
{@render children()}

<!-- ❌ WRONG — NEVER use these -->
<!-- export let title -->
<!-- $: isActive = count > 0 -->
<!-- <slot /> -->
<!-- <slot name="header" /> -->
```

### Snippets Over Slots

Svelte 5 uses `Snippet` for content projection. Never use `<slot>`.

```svelte
<!-- Parent -->
<Card>
  {#snippet header()}
    <h2>Title</h2>
  {/snippet}
  {#snippet content()}
    <p>Body text</p>
  {/snippet}
</Card>

<!-- Card.svelte -->
<script lang="ts">
  import { type Snippet } from 'svelte';

  interface Props {
    header: Snippet;
    content: Snippet;
  }
  let { header, content }: Props = $props();
</script>

<div class="card">
  <div class="card-header">{@render header()}</div>
  <div class="card-body">{@render content()}</div>
</div>
```

### Event Handling — Svelte 5 Syntax

```svelte
<!-- ✅ Svelte 5 — standard HTML event attributes -->
<button onclick={() => handleClick()}>Click</button>
<button onclick={handleClick}>Click</button>
<input oninput={(e) => search = e.currentTarget.value} />

<!-- ❌ NEVER — Svelte 4 directive syntax -->
<!-- <button on:click={handleClick}> -->
<!-- <input on:input={handleInput}> -->
```

### GSAP Integration Pattern

```svelte
<script lang="ts">
  import { gsap } from 'gsap';
  import { ScrollTrigger } from 'gsap/ScrollTrigger';

  // Register plugin once
  if (typeof window !== 'undefined') {
    gsap.registerPlugin(ScrollTrigger);
  }

  let sectionRef: HTMLElement | undefined = $state();

  $effect(() => {
    if (!sectionRef) return;

    const ctx = gsap.context(() => {
      gsap.from('.reveal-item', {
        y: 40,
        opacity: 0,
        duration: 0.8,
        stagger: 0.15,
        ease: 'power3.out',
        scrollTrigger: {
          trigger: sectionRef,
          start: 'top 80%',
          once: true,
        },
      });
    }, sectionRef);

    return () => ctx.revert();
  });
</script>

<section bind:this={sectionRef}>
  <div class="reveal-item">...</div>
  <div class="reveal-item">...</div>
</section>
```

### Svelte Built-in Transitions (for component-level motion)

Use Svelte transitions for modals, dropdowns, and UI state changes — NOT GSAP. GSAP is for scroll-triggered page animations only.

```svelte
<script lang="ts">
  import { fade, fly, scale, slide } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
</script>

{#if isOpen}
  <!-- Overlay -->
  <div transition:fade={{ duration: 300 }}>
    <!-- Modal container -->
    <div in:fly={{ y: 30, duration: 400, easing: cubicOut }}
         out:fade={{ duration: 200 }}>
      ...
    </div>
  </div>
{/if}
```

---

## Revolution Typography System

Integrate this CSS custom property system into TailwindCSS. Load Montserrat (400, 500, 600, 700, 800) and Inter (400, 500, 600, 700) from Google Fonts.

```css
/* =========================================
   REVOLUTION TYPOGRAPHY SYSTEM (Mobile-First)
   Pairing: Montserrat (brand headings) + Inter (UI/data/body)
   ========================================= */

:root {
  --font-heading: "Montserrat", "Inter", "Segoe UI", Roboto, Arial, sans-serif;
  --font-ui: "Inter", "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;

  --w-regular: 400;
  --w-medium: 500;
  --w-semibold: 600;
  --w-bold: 700;
  --w-extrabold: 800;

  --lh-tight: 1.1;
  --lh-snug: 1.25;
  --lh-normal: 1.5;
  --lh-relaxed: 1.65;

  --ls-tight: -0.02em;
  --ls-normal: 0;
  --ls-wide: 0.01em;

  /* MOBILE TYPE SCALE (default) */
  --fs-2xs: 0.6875rem;
  --fs-xs: 0.75rem;
  --fs-sm: 0.875rem;
  --fs-md: 1rem;
  --fs-lg: 1.125rem;
  --fs-xl: 1.25rem;
  --fs-2xl: 1.5rem;
  --fs-3xl: 1.875rem;
  --fs-4xl: 2.25rem;
}

@media (min-width: 48rem) {
  :root {
    --fs-xs: 0.8125rem;
    --fs-sm: 0.9375rem;
    --fs-md: 1rem;
    --fs-lg: 1.1875rem;
    --fs-xl: 1.375rem;
    --fs-2xl: 1.75rem;
    --fs-3xl: 2.125rem;
    --fs-4xl: 2.75rem;
  }
}

@media (min-width: 64rem) {
  :root {
    --fs-xs: 0.875rem;
    --fs-sm: 1rem;
    --fs-md: 1.0625rem;
    --fs-lg: 1.25rem;
    --fs-xl: 1.5rem;
    --fs-2xl: 2rem;
    --fs-3xl: 2.5rem;
    --fs-4xl: 3.25rem;
  }
}

html {
  -webkit-text-size-adjust: 100%;
  text-rendering: optimizeLegibility;
}

body {
  font-family: var(--font-ui);
  font-size: var(--fs-sm);
  font-weight: var(--w-regular);
  line-height: var(--lh-normal);
}

h1, h2, h3 {
  font-family: var(--font-heading);
  letter-spacing: var(--ls-tight);
  line-height: var(--lh-snug);
}
h1 { font-size: var(--fs-3xl); font-weight: var(--w-extrabold); }
h2 { font-size: var(--fs-2xl); font-weight: var(--w-bold); }
h3 { font-size: var(--fs-xl); font-weight: var(--w-bold); }

h4, h5, h6 { font-family: var(--font-ui); line-height: var(--lh-snug); }
h4 { font-size: var(--fs-lg); font-weight: var(--w-semibold); }
h5 { font-size: var(--fs-md); font-weight: var(--w-semibold); }
h6 { font-size: var(--fs-sm); font-weight: var(--w-semibold); }

/* Financial UI roles */
.kpi-value {
  font-family: var(--font-ui);
  font-size: var(--fs-2xl);
  font-weight: var(--w-bold);
  line-height: var(--lh-tight);
  font-variant-numeric: tabular-nums lining-nums;
}

.kpi-label {
  font-size: var(--fs-xs);
  font-weight: var(--w-medium);
  text-transform: uppercase;
  letter-spacing: var(--ls-wide);
}

.hero-eyebrow {
  font-family: var(--font-ui);
  font-size: var(--fs-xs);
  font-weight: var(--w-semibold);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.hero-title {
  font-family: var(--font-heading);
  font-size: clamp(1.875rem, 5vw, 3.75rem);
  font-weight: var(--w-extrabold);
  line-height: 1.05;
  letter-spacing: -0.025em;
}

.hero-subtitle {
  font-size: var(--fs-md);
  line-height: var(--lh-relaxed);
  max-width: 65ch;
}
```

---

## Brand Design Tokens

```css
:root {
  /* Primary palette */
  --navy: #0B1D3A;
  --navy-mid: #132B50;
  --deep-blue: #1A3A6B;
  --teal: #0FA4AF;
  --teal-light: #15C5D1;
  --teal-glow: rgba(15, 164, 175, 0.15);
  --gold: #D4A843;
  --gold-light: #E8C76A;

  /* Neutrals */
  --white: #FFFFFF;
  --off-white: #F7F8FA;
  --grey-100: #EEF0F4;
  --grey-200: #D8DCE4;
  --grey-400: #8B95A8;
  --grey-600: #5A6478;
  --grey-800: #2E3749;

  /* Semantic */
  --red: #E04848;
  --green: #22B573;
}
```

Map these into TailwindCSS `theme.extend.colors` so they're available as utilities (e.g., `text-navy`, `bg-teal`, `border-grey-200`).

---

## File Structure

```
src/
├── routes/
│   └── +page.svelte                    # Landing page route
│   └── +layout.svelte                  # Root layout (nav + footer + floating button)
├── lib/
│   ├── components/
│   │   ├── landing/
│   │   │   ├── Hero.svelte             # Hero section with sample alert card
│   │   │   ├── SampleAlertCard.svelte  # Interactive alert card (reusable)
│   │   │   ├── WhyDifferent.svelte     # 3-card differentiator section
│   │   │   ├── WhatYouGet.svelte       # Sunday watchlist deliverables
│   │   │   ├── WhoItsFor.svelte        # Target audience section
│   │   │   ├── Pricing.svelte          # Monthly / Annual pricing cards
│   │   │   ├── Courses.svelte          # Education courses section
│   │   │   ├── About.svelte            # About Billy + stats grid
│   │   │   └── FinalCta.svelte         # Bottom CTA section
│   │   ├── traders/
│   │   │   ├── TradersModal.svelte     # Full modal with grid + profile views
│   │   │   ├── TraderCard.svelte       # Clickable trader card (grid view)
│   │   │   └── TraderProfile.svelte    # Full bio + stats + action buttons
│   │   └── ui/
│   │       ├── Nav.svelte              # Sticky nav bar
│   │       ├── Footer.svelte           # Site footer
│   │       ├── FloatingButton.svelte   # "Meet The Traders" FAB
│   │       ├── Button.svelte           # Reusable button (primary/ghost/outline)
│   │       ├── SectionHeader.svelte    # Eyebrow + title + subtitle pattern
│   │       └── ScrollReveal.svelte     # GSAP scroll-triggered wrapper
│   ├── data/
│   │   ├── traders.ts                  # Billy & Freddie trader data
│   │   ├── pricing.ts                  # Plan definitions
│   │   ├── courses.ts                  # Course definitions
│   │   └── alerts.ts                   # Sample alert data
│   ├── stores/
│   │   └── modal.svelte.ts             # Modal state using Svelte 5 runes
│   └── styles/
│       ├── typography.css              # Revolution Typography System
│       └── tokens.css                  # Brand design tokens
```

---

## Section-by-Section Specifications

### 1. Nav (`Nav.svelte`)

- Fixed top, `z-50`, frosted glass effect: `bg-navy/92 backdrop-blur-md`
- Left: Logo text "Explosive" in white + "Swings" in `--teal-light`
- Right: CTA button "Get Instant Access" linking to `#pricing`
- 64px height, max-width 1200px inner container
- Font: Use `--font-heading` for logo, `--font-ui` for CTA
- Phosphor icon: none needed, text-only CTA

### 2. Hero (`Hero.svelte`)

Full viewport height, dark gradient background with decorative elements.

**Layout**: CSS Grid, 2 columns on desktop (1fr 1fr), single column on mobile. `padding-top: 64px` to clear the fixed nav.

**Background layers** (bottom to top):
1. Base gradient: `linear-gradient(168deg, var(--navy) 0%, var(--navy-mid) 45%, var(--deep-blue) 100%)`
2. Grid overlay: Subtle 60px grid lines at 2% white opacity, masked with radial gradient
3. Teal glow orb: 700px radial gradient circle, positioned top-right, animated with GSAP `pulse` (scale 1→1.08, opacity 0.6→1, 6s ease-in-out infinite)

**Left column** (staggered GSAP entrance, `power3.out`, 0.6s each with 0.1s stagger):
- **Eyebrow badge**: Pill shape, teal border/bg tint, blinking dot, text "Weekly watchlist delivered every Sunday night"
- **H1**: "Simple, Early Stock Alerts _You Can Actually Use_" — accent text in `--teal-light`, use `hero-title` class from typography system
- **Subtitle**: "Every Sunday night, get a detailed watchlist of 5–7 top stock picks with defined entries, targets, exits, and stops — so you're ready before the market opens."
- **Actions**: Primary CTA "Get Instant Access" + Ghost button "See How It Works" (scrolls to `#how-it-works`). Phosphor icons: `ArrowRight` (weight: bold) in primary CTA
- **Trust line**: Avatar circle "BR" with gradient bg + text: "Created by Billy Ribeiro — former lead trader at Simpler Trading, mentored by Goldman Sachs' Mark McGoldrick"

**Right column** — `SampleAlertCard.svelte`:
- Glassmorphic card: `bg-white/4 border border-white/8 backdrop-blur-sm rounded-2xl`
- Header row: "SAMPLE ALERT" label + green "Live Format" indicator with blinking dot
- Ticker: "AAPL" in monospace (`font-variant-numeric: tabular-nums`)
- Data rows: Entry Zone (teal), Invalidation (red), Profit Zones 1/2/3 (green)
- Notes callout: Left teal border, teal-tinted bg
- GSAP entrance: `fly` in from right with slight delay

**Sample Alert Data**:
```typescript
{
  ticker: 'AAPL',
  entryZone: '182.50 – 183.20',
  invalidation: 'Below 181.90',
  profitZones: ['185.20', '186.40', '188.00'],
  notes: 'Watching for a breakout above the 20-day MA with strong volume. Early entry zone gives room before momentum kicks in.'
}
```

### 3. Why Different (`WhyDifferent.svelte`)

**Background**: `--off-white`

**Section header**:
- Eyebrow: "Why We're Different"
- Title: "Most Alert Services Tell You _After_ the Move. We Tell You _Before_."
- Subtitle: "Built on the proprietary 'Move Prior to The Move' methodology — your watchlist arrives Sunday night so you can set alerts and be positioned before the action starts."

**3-column card grid** (single column on mobile):

Each card has:
- Top accent bar: 3px gradient teal, `scaleX(0)` → `scaleX(1)` on hover via CSS transition
- Icon in teal-tinted square (Phosphor icons, weight: duotone):
  1. `Clock` — **Early Entry Alerts**: "Most services notify you after the move has already happened. We send alerts before the breakout, with the exact price level we're watching — giving you time to plan, not chase."
  2. `ShieldCheck` — **Clear Invalidation Levels**: "Every alert includes a simple 'no longer valid' level. If price breaks below it, the setup is done — no confusion, no guessing, no hoping. You always know where you stand."
  3. `CurrencyDollar` — **Simple Profit-Taking Guidance**: "We highlight logical profit zones so you can scale out with confidence, even if you're not an experienced trader. No complicated calculations — just clear, actionable targets."

GSAP: Staggered `fadeInUp` on scroll, 0.15s stagger between cards.

### 4. What You Get (`WhatYouGet.svelte`)

**Background**: White

**Section header**:
- Eyebrow: "What's Included"
- Title: "Your Sunday Night Watchlist"
- Subtitle: "Every Sunday night, your watchlist drops with everything you need to trade the week ahead — no guessing, no scrambling Monday morning."

**2-column grid of feature items** (single column mobile):

Each item: Phosphor `CheckCircle` (weight: fill, color teal) + title + description.

1. **5–7 Top Picks Every Sunday Night** — "A detailed watchlist delivered before the week starts — complete with entries, targets, exits, and stops so you're fully prepared before Monday's open."
2. **Charts with Marked Zones** — "Every alert comes with annotated charts showing entry, invalidation, and profit targets visually."
3. **Entries, Targets, Exits & Stops** — "Every pick comes with precise levels — no ambiguity. You know exactly where to get in, where to take profit, and where to cut it."
4. **Detailed Weekly Video Walkthrough** — "A thorough breakdown of every pick on the watchlist — so you understand the setup, the levels, and can set your alerts ahead of time with confidence."
5. **Multi-Channel Delivery** (spans full width on desktop) — "Watchlist delivered via email, SMS, and your members-only dashboard every Sunday night — so you never miss a week."

GSAP: Staggered reveal, 0.12s stagger.

### 5. Who It's For (`WhoItsFor.svelte`)

**Background**: `--off-white`

**Section header**:
- Eyebrow: "Who This Is For"
- Title: "If You Want Clarity Without Complexity, This Was Built for You"

**2-column grid**, each item is a bordered row with Phosphor `CaretRight` (weight: bold, teal) + text:

1. Traders who want **simple, followable alerts**
2. People who don't have time to learn advanced strategies
3. Traders who want **early entries**, not late chases
4. Anyone who wants guidance without sitting in a live room

### 6. Pricing (`Pricing.svelte`)

**Background**: White

**Section header**:
- Eyebrow: "Pricing"
- Title: "Straightforward Pricing. No Contracts. Cancel Anytime."

**2-column grid** (max-width 800px), single column mobile:

**Monthly Card**:
- Name: "Monthly"
- Amount: `$49` + `/mo` suffix
- Note: "Cancel anytime. No commitment."
- CTA: Outline button "Start Monthly"

**Annual Card** (featured):
- "Best Value" badge: Absolute positioned pill above card, teal bg
- Name: "Annual"
- Amount: `$399` + `/yr` suffix
- Note: "Billed once per year."
- Save badge: Green tinted pill "Save 32%"
- CTA: Primary filled button "Start Annual Plan"

Featured card: Teal border, subtle teal gradient at top.

### 7. Courses (`Courses.svelte`)

**Background**: `--off-white`

**Section header**:
- Eyebrow: "Education"
- Title: "Level Up Your Options Game"
- Subtitle: "Structured courses designed to take you from the basics to confidently trading options — at your own pace."

**2-column card grid** (single column mobile):

Each course card has:
- **Visual header** (160px): Gradient bg (navy→deep-blue for beginner, deep-blue→teal for intermediate) with grid pattern overlay, centered icon in frosted square, level badge (pill, top-left)
- **Body**: Title, description, meta row (Self-Paced + Level), "Learn More" link with `ArrowRight` icon

**Course 1 — Beginning to Options Trading** (Beginner):
- Icon: Phosphor `BookOpen` (weight: duotone)
- Description: "Start from scratch. Learn what options are, how they work, and how to place your first trades with confidence — no prior experience needed."
- Meta: Self-Paced | All Levels

**Course 2 — Options Trading 101** (Intermediate):
- Icon: Phosphor `Pulse` (weight: duotone)
- Description: "Go deeper into calls, puts, spreads, and real strategies. Learn how to read the options chain, manage risk, and build consistent setups that work."
- Meta: Self-Paced | Intermediate

### 8. About (`About.svelte`)

**Background**: `--navy` (dark section)

**2-column layout**: Left = text, Right = stats grid. Single column on mobile.

**Left column**:
- Eyebrow: "About Billy Ribeiro" (teal-light)
- Title: "The Trader Behind the Alerts" (white)
- Bio paragraphs (grey-400 text, white for strong/bold):
  1. "Billy Ribeiro is a **high-performance options trader** known for his precision, discipline, and consistency. During his time at **Simpler Trading**, he quickly became their **lead trader** — at one point generating more winning trades in a single week than the entire staff combined."
  2. "Mentored by **Mark McGoldrick of Goldman Sachs**, Billy developed his proprietary **'Move Prior to The Move'** methodology — a framework for identifying institutional-quality setups before the crowd catches on."
  3. "After recovering from cancer, Billy shifted his focus toward sustainable, high-impact trading education. This alert service allows him to share his expertise without the stress of daily live trading — and gives members access to **clean, early, high-probability setups**."

**Right column — Stats grid** (2x2):
- `18K+` — Active Traders
- `600%` — OXY Overnight Return
- `573x` — 0DTE SPX Return
- `5–7` — Picks Per Week

Each stat: Dark card with subtle border, teal-light number using `kpi-value` typography class, uppercase label using `kpi-label` class.

### 9. Final CTA (`FinalCta.svelte`)

**Background**: Navy→deep-blue gradient with centered teal glow orb.

- Title: "Trade with Clarity. Trade with Confidence." (white)
- Subtitle: "Get your weekly watchlist every Sunday night — detailed entries, targets, exits, and stops so you're prepared before the market opens."
- CTA: "Get Instant Access to Alerts" with `ArrowRight` icon

### 10. Footer (`Footer.svelte`)

- Navy bg with top border
- Copyright line with Terms | Privacy | Contact links (teal-light)
- Risk disclaimer: "Trading involves risk. Past performance does not guarantee future results. Always do your own due diligence."

---

## Floating "Meet The Traders" Button (`FloatingButton.svelte`)

Fixed position, `bottom-8 right-8`, `z-40`.

- Pill shape on desktop: Phosphor `UsersThree` icon (weight: duotone, teal-light) + "Meet The Traders" text
- Icon-only circle on mobile (`< 640px`): Just the `UsersThree` icon
- Navy gradient bg, teal border accent, subtle pulse animation (box-shadow glow)
- On click: Opens `TradersModal`

---

## Traders Modal System (`TradersModal.svelte`)

Full-screen overlay modal with **two views** managed by reactive state.

### Modal State (`modal.svelte.ts`)

```typescript
// Svelte 5 reactive module state
let isOpen = $state(false);
let activeView = $state<'grid' | 'profile'>('grid');
let activeTrader = $state<string | null>(null);

export function openModal() {
  isOpen = true;
  activeView = 'grid';
  activeTrader = null;
}

export function closeModal() {
  isOpen = false;
}

export function showProfile(traderId: string) {
  activeTrader = traderId;
  activeView = 'profile';
}

export function backToGrid() {
  activeView = 'grid';
  activeTrader = null;
}

export { isOpen, activeView, activeTrader };
```

### Overlay

- `fixed inset-0 z-50`, `bg-navy/85 backdrop-blur-lg`
- Svelte transition: `fade` on overlay, `fly` (y: 30) on container
- Close on: Backdrop click, X button, `Escape` key (use `$effect` with `keydown` listener)
- Lock body scroll when open

### Grid View (`TraderCard.svelte`)

2-column grid of trader cards:

**Billy Ribeiro**:
- Avatar: Gradient circle (teal→deep-blue) with initials "BR"
- Name, Role: "Founder & Lead Trader"
- Tagline: "Former lead trader at Simpler Trading. Creator of 'The Move Prior to The Move' methodology. Mentored by Goldman Sachs."
- "View Profile" link with `ArrowRight`

**Freddie Ferber**:
- Avatar: Gradient circle (gold→navy-mid) with initials "FF"
- Name, Role: "Senior Trader"
- Tagline: "Disciplined swing trader with a sharp eye for high-probability setups and clean risk-to-reward entries."
- "View Profile" link with `ArrowRight`

Cards hover: Lift up 4px, teal border glow, shadow.

### Profile View (`TraderProfile.svelte`)

Back button: Phosphor `ArrowLeft` + "All Traders" — calls `backToGrid()`. Use Svelte `fly` transition when switching views.

**Profile header**: Avatar + Name + Role

**Bio**: 2-3 paragraphs (content differs per trader — see trader data below)

**Highlights grid** (3 columns): KPI-style stat cards

**Action buttons row** (these are scaffolded — content to be added later):
- **Favorite Setups**: Primary button, Phosphor `Star` (weight: fill)
- **Trading Style**: Secondary/outline button, Phosphor `Pulse`
- **Notable Trades**: Secondary/outline button, Phosphor `BookOpen`

### Trader Data (`traders.ts`)

```typescript
export interface Trader {
  id: string;
  name: string;
  initials: string;
  role: string;
  tagline: string;
  avatarGradient: { from: string; to: string };
  accentColor: string;
  bio: string[];
  highlights: { value: string; label: string }[];
  actions: { label: string; icon: string; variant: 'primary' | 'secondary' }[];
}

export const traders: Trader[] = [
  {
    id: 'billy',
    name: 'Billy Ribeiro',
    initials: 'BR',
    role: 'Founder & Lead Trader',
    tagline: 'Former lead trader at Simpler Trading. Creator of "The Move Prior to The Move" methodology. Mentored by Goldman Sachs.',
    avatarGradient: { from: '#0FA4AF', to: '#1A3A6B' },
    accentColor: '#0FA4AF',
    bio: [
      'Billy Ribeiro is a <strong>high-performance options trader</strong> known for his precision, discipline, and consistency. During his time at <strong>Simpler Trading</strong>, he quickly became their <strong>lead trader</strong> — at one point generating more winning trades in a single week than the entire staff combined.',
      'Mentored by <strong>Mark McGoldrick of Goldman Sachs</strong>, Billy developed his proprietary <strong>"Move Prior to The Move"</strong> methodology — a framework for identifying institutional-quality setups before the crowd catches on.',
      'After recovering from cancer, Billy shifted his focus toward sustainable, high-impact trading education. He now leads <strong>Explosive Swings</strong> and <strong>Revolution Trading Pros</strong>, serving over <strong>18,000 active traders</strong>.',
    ],
    highlights: [
      { value: '18K+', label: 'Active Traders' },
      { value: '600%', label: 'OXY Overnight' },
      { value: '573x', label: '0DTE SPX Return' },
    ],
    actions: [
      { label: 'Favorite Setups', icon: 'Star', variant: 'primary' },
      { label: 'Trading Style', icon: 'Pulse', variant: 'secondary' },
      { label: 'Notable Trades', icon: 'BookOpen', variant: 'secondary' },
    ],
  },
  {
    id: 'freddie',
    name: 'Freddie Ferber',
    initials: 'FF',
    role: 'Senior Trader',
    tagline: 'Disciplined swing trader with a sharp eye for high-probability setups and clean risk-to-reward entries.',
    avatarGradient: { from: '#D4A843', to: '#132B50' },
    accentColor: '#D4A843',
    bio: [
      'Freddie Ferber is a <strong>disciplined swing trader</strong> who specializes in identifying high-probability setups with clean risk-to-reward profiles. His methodical approach to the markets focuses on patience, precision, and letting the trade come to you.',
      'With deep expertise in <strong>technical analysis</strong> and <strong>price action</strong>, Freddie brings a complementary edge to the Explosive Swings watchlist — helping surface setups that meet the team\'s rigorous standards for quality and clarity.',
      'Freddie\'s trading philosophy centers on <strong>capital preservation first</strong> — every setup must have a defined risk before it earns a spot on the watchlist.',
    ],
    highlights: [
      { value: 'Swing', label: 'Primary Style' },
      { value: 'Stocks', label: 'Focus' },
      { value: 'R:R', label: 'Risk-First' },
    ],
    actions: [
      { label: 'Favorite Setups', icon: 'Star', variant: 'primary' },
      { label: 'Trading Style', icon: 'Pulse', variant: 'secondary' },
      { label: 'Notable Trades', icon: 'BookOpen', variant: 'secondary' },
    ],
  },
];
```

---

## GSAP Animation Strategy

### Scroll Reveal Wrapper (`ScrollReveal.svelte`)

Create a reusable wrapper component that applies GSAP ScrollTrigger to its children:

```svelte
<script lang="ts">
  import { type Snippet } from 'svelte';
  import { gsap } from 'gsap';
  import { ScrollTrigger } from 'gsap/ScrollTrigger';

  if (typeof window !== 'undefined') {
    gsap.registerPlugin(ScrollTrigger);
  }

  interface Props {
    children: Snippet;
    selector?: string;       // Child selector for stagger targets
    stagger?: number;         // Stagger delay between children
    y?: number;               // Starting Y offset
    duration?: number;
    delay?: number;
    start?: string;           // ScrollTrigger start position
  }

  let {
    children,
    selector = '.reveal-item',
    stagger = 0.12,
    y = 40,
    duration = 0.8,
    delay = 0,
    start = 'top 80%',
  }: Props = $props();

  let container: HTMLElement | undefined = $state();

  $effect(() => {
    if (!container) return;
    const targets = container.querySelectorAll(selector);
    if (!targets.length) return;

    const ctx = gsap.context(() => {
      gsap.from(targets, {
        y,
        opacity: 0,
        duration,
        delay,
        stagger,
        ease: 'power3.out',
        scrollTrigger: {
          trigger: container,
          start,
          once: true,
        },
      });
    }, container);

    return () => ctx.revert();
  });
</script>

<div bind:this={container}>
  {@render children()}
</div>
```

### Hero Entrance Timeline

Use a GSAP timeline in `Hero.svelte` for orchestrated entrance:

```typescript
$effect(() => {
  if (!heroRef) return;

  const ctx = gsap.context(() => {
    const tl = gsap.timeline({ defaults: { ease: 'power3.out' } });

    tl.from('.hero-badge', { y: 24, opacity: 0, duration: 0.6 })
      .from('.hero-title', { y: 24, opacity: 0, duration: 0.6 }, '-=0.4')
      .from('.hero-subtitle', { y: 24, opacity: 0, duration: 0.6 }, '-=0.4')
      .from('.hero-actions', { y: 24, opacity: 0, duration: 0.6 }, '-=0.4')
      .from('.hero-trust', { y: 24, opacity: 0, duration: 0.6 }, '-=0.4')
      .from('.hero-card', { x: 60, opacity: 0, duration: 0.8 }, '-=0.8');
  }, heroRef);

  return () => ctx.revert();
});
```

### Glow Orb Animation

GSAP infinite tween for the decorative glow:

```typescript
gsap.to('.glow-orb', {
  scale: 1.08,
  opacity: 1,
  duration: 3,
  ease: 'sine.inOut',
  yoyo: true,
  repeat: -1,
});
```

---

## SEO & Performance

### Meta Tags (`+page.svelte`)

```svelte
<svelte:head>
  <title>Explosive Swings — Early Stock Alerts You Can Actually Use</title>
  <meta name="description" content="Every Sunday night, get 5–7 top stock picks with defined entries, targets, exits, and stops. Created by Billy Ribeiro, former lead trader at Simpler Trading." />
  <meta property="og:title" content="Explosive Swings — Weekly Stock Watchlist" />
  <meta property="og:description" content="Clear, actionable trade ideas delivered every Sunday night. 5–7 picks with entries, targets, exits, and stops." />
  <meta property="og:type" content="website" />
  <meta name="twitter:card" content="summary_large_image" />
</svelte:head>
```

### Performance Rules

- GSAP and ScrollTrigger: Dynamic import only in `$effect` or `onMount`-equivalent patterns — never SSR
- Guard all browser APIs with `typeof window !== 'undefined'`
- Preload fonts in `app.html` with `<link rel="preconnect">` and `<link rel="preload">`
- Use `loading="lazy"` on any images below the fold
- All GSAP contexts must return cleanup: `return () => ctx.revert()`

---

## Accessibility

- All interactive elements must be keyboard accessible
- Modal must trap focus when open
- Modal close on `Escape` key
- Floating button must have `aria-label="Meet the traders"`
- Use semantic HTML: `<nav>`, `<main>`, `<section>`, `<footer>`
- Section headers should use proper heading hierarchy (h1 in hero only, h2 for sections, h3 for cards)
- Skip-to-content link recommended

---

## Critical Rules — DO NOT Violate

1. **Svelte 5 runes ONLY** — no `export let`, no `$:`, no `<slot>`, no `on:event` directives
2. **TypeScript strict** — zero `any`, zero warnings, all interfaces/types explicit
3. **Phosphor Icons ONLY** — never Lucide, never inline SVGs for icons
4. **pnpm ONLY** — never npm or yarn
5. **GSAP for scroll animations** — Svelte transitions for UI state changes (modals, tabs)
6. **Revolution Typography System** — use the CSS custom properties and classes, don't hardcode font sizes
7. **Mobile-first** — all layouts start single column, expand at breakpoints
8. **Every `$effect` using GSAP must return cleanup** — `return () => ctx.revert()`
9. **No Svelte 4 patterns anywhere** — if you see `on:click`, `export let`, `$:`, or `<slot>`, it's wrong
10. **Build for 10-year longevity** — no hacks, no shortcuts, enterprise-grade code quality
