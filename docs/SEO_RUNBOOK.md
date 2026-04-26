# SEO Runbook

This runbook defines the SEO operating standard for Precision Options Signals.

## 1) Metadata and Canonicals

- Use `src/lib/seo/Seo.svelte` for every indexable public page.
- Always provide unique `title` and `description` that match user intent.
- Canonicals must be absolute (`https://precisionoptionsignals.com/...`) and point to the preferred URL.
- Keep `og:*` and `twitter:*` aligned with canonical and page topic.

## 2) Indexing Controls

- Public marketing, blog, course, and legal pages are indexable.
- Authenticated app areas (`/dashboard`, `/admin`, auth endpoints) must remain `noindex, nofollow`.
- Keep crawler restrictions in `static/robots.txt` synchronized with route behavior.

## 3) Sitemap Policy

- `src/routes/sitemap.xml/+server.ts` is the source of truth.
- Include all indexable route families:
  - static public pages
  - course detail pages
  - blog posts
  - blog categories and tags (if intended to rank)
- Provide `lastmod` for dynamic content when data is available.
- Fail gracefully when external API sources are unavailable.

## 4) Structured Data

- Sitewide: Organization + WebSite graph from root layout.
- Route-level:
  - `WebPage` on top-level content pages
  - `BlogPosting` on article pages
  - `Course` on course detail pages
  - `Product`/`Offer` on pricing pages
- Validate representative pages in Google Rich Results Test after schema changes.

## 5) Content Quality Standard (Google-aligned)

- Prioritize people-first usefulness over keyword volume.
- Ensure each page has:
  - clear primary intent in first screenful
  - concise, non-clickbait title and description
  - trustworthy claims and disclaimers where required
  - internal links to the next relevant step for the user
- Avoid thin, repetitive, or scaled boilerplate pages.

## 6) Release Checklist

Run before every SEO-impacting merge:

1. `pnpm check`
2. `pnpm lint`
3. `pnpm ci:seo`
4. `pnpm build`

## 7) Monitoring Checklist

- Google Search Console:
  - Coverage: newly indexed vs excluded URLs
  - Enhancements: rich result warnings/errors
  - Performance: query and page trend deltas after releases
- Core Web Vitals:
  - watch route-level regressions on high-traffic pages (`/`, `/blog`, `/pricing/*`, `/courses/*`)
