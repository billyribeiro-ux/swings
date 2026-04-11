# Explosive Swings — SvelteKit Landing Page

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
PUBLIC_APP_URL=http://localhost:5180
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

Open [http://localhost:5180](http://localhost:5180) in your browser.

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
```

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
