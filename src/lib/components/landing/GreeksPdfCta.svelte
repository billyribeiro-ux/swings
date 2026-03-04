<script lang="ts">
  import { onMount } from 'svelte';
  import { gsap } from 'gsap';
  import DownloadSimple from 'phosphor-svelte/lib/DownloadSimple';
  import EnvelopeSimple from 'phosphor-svelte/lib/EnvelopeSimple';
  import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
  import FilePdf from 'phosphor-svelte/lib/FilePdf';

  let email = $state('');
  let isSubmitting = $state(false);
  let isSuccess = $state(false);
  let error = $state('');
  let containerRef: HTMLElement | undefined = $state();

  onMount(() => {
    if (!containerRef) return;

    const els = ['.greeks-icon', '.greeks-title', '.greeks-desc', '.greeks-form'];
    gsap.set(els, { opacity: 0, y: 20, willChange: 'transform, opacity', force3D: true });

    const ctx = gsap.context(() => {
      const tl = gsap.timeline({ defaults: { ease: 'expo.out', duration: 1.2, force3D: true }, delay: 0.15 });
      tl.to('.greeks-icon', { opacity: 1, y: 0, scale: 1 })
        .to('.greeks-title', { opacity: 1, y: 0 }, '-=0.9')
        .to('.greeks-desc', { opacity: 1, y: 0 }, '-=1.0')
        .to('.greeks-form', { opacity: 1, y: 0 }, '-=1.0')
        .call(() => { gsap.set(els, { willChange: 'auto' }); });
    }, containerRef as HTMLElement);

    return () => { ctx.revert(); gsap.set(els, { clearProps: 'all' }); };
  });

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';
    
    if (!email || !email.includes('@')) {
      error = 'Please enter a valid email address';
      return;
    }

    isSubmitting = true;

    try {
      const response = await fetch('/api/greeks-pdf', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email })
      });

      if (!response.ok) {
        const data = await response.json();
        throw new Error(data.error || 'Failed to send PDF');
      }

      isSuccess = true;
      email = '';
    } catch (err) {
      error = err instanceof Error ? err.message : 'Something went wrong. Please try again.';
    } finally {
      isSubmitting = false;
    }
  }
</script>

<section bind:this={containerRef} class="py-16 sm:py-20 lg:py-28 bg-linear-to-br from-navy via-navy-mid to-deep-blue">
  <div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
    <div class="mx-auto max-w-3xl overflow-hidden rounded-3xl border border-white/10 bg-white/5 backdrop-blur-sm">
      <div class="grid gap-8 p-8 sm:p-10 lg:grid-cols-[auto_1fr] lg:gap-12 lg:p-12">
        <!-- Icon -->
        <div class="greeks-icon flex justify-center lg:justify-start">
          <div class="flex h-20 w-20 items-center justify-center rounded-2xl bg-teal/15 ring-4 ring-teal/10">
            <FilePdf size={40} weight="duotone" color="#15C5D1" />
          </div>
        </div>

        <!-- Content -->
        <div>
          <h2 class="greeks-title mb-3 text-2xl font-bold text-white sm:text-3xl font-heading">
            Free Guide: The Ultimate Guide to Options Greeks
          </h2>

          <p class="greeks-desc mb-6 text-base text-grey-300 leading-relaxed">
            Master Delta, Gamma, Theta, and Vega with our comprehensive 20-page PDF guide. Learn how to use Greeks to make smarter trading decisions and manage risk like a pro.
          </p>

          {#if isSuccess}
            <div class="rounded-xl border border-green/30 bg-green/10 p-4">
              <div class="flex items-start gap-3">
                <CheckCircle size={24} weight="fill" color="#22B573" class="shrink-0" />
                <div>
                  <h3 class="mb-1 text-sm font-semibold text-white">Check Your Inbox!</h3>
                  <p class="text-xs text-grey-300">
                    We've sent the Greeks PDF guide to your email. Check your spam folder if you don't see it within a few minutes.
                  </p>
                </div>
              </div>
            </div>
          {:else}
            <form onsubmit={handleSubmit} class="greeks-form">
              <div class="flex flex-col gap-3 sm:flex-row">
                <div class="relative flex-1">
                  <EnvelopeSimple
                    size={18}
                    weight="bold"
                    class="absolute left-4 top-1/2 -translate-y-1/2 text-grey-400"
                  />
                  <input
                    type="email"
                    bind:value={email}
                    placeholder="Enter your email"
                    required
                    disabled={isSubmitting}
                    class="w-full rounded-xl border border-white/20 bg-white/10 py-3.5 pl-12 pr-4 text-sm text-white placeholder-grey-400 backdrop-blur-sm transition-all duration-200 focus:border-teal/50 focus:bg-white/15 focus:outline-none focus:ring-2 focus:ring-teal/30 disabled:opacity-50"
                  />
                </div>
                <button
                  type="submit"
                  disabled={isSubmitting}
                  class="inline-flex items-center justify-center gap-2 rounded-xl bg-teal px-6 py-3.5 text-sm font-semibold text-white shadow-lg shadow-teal/25 transition-all duration-300 hover:bg-teal-light hover:shadow-xl hover:shadow-teal/30 hover:-translate-y-px active:scale-[0.97] disabled:pointer-events-none disabled:opacity-50 sm:w-auto"
                >
                  {#if isSubmitting}
                    Sending...
                  {:else}
                    <DownloadSimple size={18} weight="bold" />
                    Get Free PDF
                  {/if}
                </button>
              </div>

              {#if error}
                <p class="mt-3 text-xs text-red">{error}</p>
              {/if}

              <p class="mt-3 text-xs text-grey-400">
                No spam, ever. Unsubscribe anytime. We respect your privacy.
              </p>
            </form>
          {/if}
        </div>
      </div>
    </div>
  </div>
</section>
