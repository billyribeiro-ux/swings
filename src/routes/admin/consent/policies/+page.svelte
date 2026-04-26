<!--
  Phase 2.3 — Permanent redirect target. The `+page.ts` `load` throws a
  308 to `/admin/consent/policy`, so this component never renders. Kept as
  a stub so SvelteKit's route-resolution doesn't 404 if the redirect is
  ever short-circuited (e.g. during prerender).
-->
<p>Redirecting to /admin/consent/policy …</p>
