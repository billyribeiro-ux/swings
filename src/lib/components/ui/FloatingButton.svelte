<script lang="ts">
	import { modal } from '$lib/stores/modal.svelte';
	import UsersThreeIcon from 'phosphor-svelte/lib/UsersThreeIcon';
</script>

<button onclick={modal.open} aria-label="Meet the traders" class="floating-btn">
	<span class="floating-btn__ping"></span>
	<UsersThreeIcon size={24} weight="duotone" color="#15C5D1" />
	<span class="floating-btn__label">Meet The Traders</span>
</button>

<style>
	.floating-btn {
		position: fixed;
		right: 2rem;
		bottom: 2rem;
		z-index: var(--z-40);
		display: flex;
		align-items: center;
		gap: 0.5rem;
		border-radius: var(--radius-full);
		border: 1px solid rgba(15, 164, 175, 0.3);
		background: linear-gradient(to right, var(--color-navy), var(--color-navy-mid));
		padding: 0.75rem 1rem;
		box-shadow: var(--shadow-lg);
		transition: all 300ms var(--ease-out);
	}

	.floating-btn:hover {
		transform: translateY(-0.25rem);
		box-shadow: var(--shadow-xl);
	}

	.floating-btn__ping {
		position: absolute;
		inset: 0;
		border-radius: var(--radius-full);
		background-color: rgba(15, 164, 175, 0.2);
		opacity: 0;
		pointer-events: none;
	}

	.floating-btn:hover .floating-btn__ping {
		opacity: 1;
		animation: ping 1s cubic-bezier(0, 0, 0.2, 1) infinite;
	}

	.floating-btn__label {
		display: none;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}

	@media (min-width: 640px) {
		.floating-btn__label {
			display: inline;
		}
	}
</style>
