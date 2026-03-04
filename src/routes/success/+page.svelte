<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
  import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
  import Button from '$lib/components/ui/Button.svelte';

  let sessionId = $state('');

  onMount(() => {
    sessionId = $page.url.searchParams.get('session_id') || '';
  });
</script>

<svelte:head>
  <title>Welcome to Explosive Swings!</title>
  <meta name="robots" content="noindex" />
</svelte:head>

<div class="min-h-screen bg-linear-to-br from-navy via-navy-mid to-deep-blue flex items-center justify-center px-4 py-20">
  <div class="max-w-2xl mx-auto text-center">
    <!-- Success Icon -->
    <div class="mb-8 flex justify-center">
      <div class="w-24 h-24 bg-green/20 rounded-full flex items-center justify-center">
        <CheckCircle size={56} weight="fill" color="#22B573" />
      </div>
    </div>

    <!-- Title -->
    <h1 class="text-4xl md:text-5xl font-bold text-white mb-6 font-heading">
      Welcome to Explosive Swings!
    </h1>

    <!-- Subtitle -->
    <p class="text-xl text-grey-300 mb-8 leading-relaxed">
      Your subscription is confirmed. Check your email for access details and your first Sunday night watchlist.
    </p>

    <!-- What's Next -->
    <div class="bg-white/5 border border-white/10 rounded-2xl p-8 mb-8 text-left">
      <h2 class="text-2xl font-bold text-white mb-6 font-heading">What Happens Next?</h2>
      
      <div class="space-y-4">
        <div class="flex gap-4">
          <div class="shrink-0 w-8 h-8 bg-teal/20 rounded-full flex items-center justify-center text-teal-light font-bold">
            1
          </div>
          <div>
            <h3 class="text-white font-semibold mb-1">Check Your Email</h3>
            <p class="text-grey-400 text-sm">You'll receive login credentials and onboarding instructions within the next few minutes.</p>
          </div>
        </div>

        <div class="flex gap-4">
          <div class="shrink-0 w-8 h-8 bg-teal/20 rounded-full flex items-center justify-center text-teal-light font-bold">
            2
          </div>
          <div>
            <h3 class="text-white font-semibold mb-1">Access Your Dashboard</h3>
            <p class="text-grey-400 text-sm">Log in to view your member area, past watchlists, and educational resources.</p>
          </div>
        </div>

        <div class="flex gap-4">
          <div class="shrink-0 w-8 h-8 bg-teal/20 rounded-full flex items-center justify-center text-teal-light font-bold">
            3
          </div>
          <div>
            <h3 class="text-white font-semibold mb-1">Get Your First Watchlist</h3>
            <p class="text-grey-400 text-sm">Every Sunday night at 8 PM ET, your weekly watchlist will be delivered via email and SMS.</p>
          </div>
        </div>
      </div>
    </div>

    <!-- CTA -->
    <div class="flex flex-col sm:flex-row gap-4 justify-center">
      <Button variant="primary" href="/">
        Back to Home
        <ArrowRight size={20} weight="bold" />
      </Button>
    </div>

    {#if sessionId}
      <p class="text-xs text-grey-500 mt-8">
        Session ID: {sessionId}
      </p>
    {/if}
  </div>
</div>
