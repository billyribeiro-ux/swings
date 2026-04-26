<script lang="ts">
	import { tick } from 'svelte';
	import { goto } from '$app/navigation';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import ChartBarIcon from 'phosphor-svelte/lib/ChartBarIcon';
	import PresentationChartIcon from 'phosphor-svelte/lib/PresentationChartIcon';
	import UsersIcon from 'phosphor-svelte/lib/UsersIcon';
	import ArticleIcon from 'phosphor-svelte/lib/ArticleIcon';
	import PlusCircleIcon from 'phosphor-svelte/lib/PlusCircleIcon';
	import TagIcon from 'phosphor-svelte/lib/TagIcon';
	import FolderOpenIcon from 'phosphor-svelte/lib/FolderOpenIcon';
	import ImageIcon from 'phosphor-svelte/lib/ImageIcon';
	import ListChecksIcon from 'phosphor-svelte/lib/ListChecksIcon';
	import UserCircleIcon from 'phosphor-svelte/lib/UserCircleIcon';

	interface PaletteItem {
		label: string;
		description?: string;
		icon: typeof MagnifyingGlassIcon;
		action: () => void;
		keywords: string[];
	}

	interface Props {
		open: boolean;
		onClose: () => void;
	}

	let { open, onClose }: Props = $props();

	const ITEMS: PaletteItem[] = [
		{
			label: 'Dashboard',
			description: 'Admin overview',
			icon: ChartBarIcon,
			action: () => goto('/admin'),
			keywords: ['home', 'dash', 'overview', 'stats']
		},
		{
			label: 'Analytics',
			description: 'Traffic, pages, CTR',
			icon: PresentationChartIcon,
			action: () => goto('/admin/analytics'),
			keywords: ['analytics', 'traffic', 'views', 'ctr', 'charts', 'three']
		},
		{
			label: 'Posts',
			description: 'Manage blog posts',
			icon: ArticleIcon,
			action: () => goto('/admin/blog'),
			keywords: ['post', 'blog', 'articles']
		},
		{
			label: 'New Post',
			description: 'Create a new blog post',
			icon: PlusCircleIcon,
			action: () => goto('/admin/blog/new'),
			keywords: ['new', 'create', 'write', 'post']
		},
		{
			label: 'Categories',
			description: 'Manage blog categories',
			icon: FolderOpenIcon,
			action: () => goto('/admin/blog/categories'),
			keywords: ['category', 'categories', 'folder']
		},
		{
			label: 'Tags',
			description: 'Manage post tags',
			icon: TagIcon,
			action: () => goto('/admin/blog/tags'),
			keywords: ['tag', 'tags', 'label']
		},
		{
			label: 'Media',
			description: 'Media library',
			icon: ImageIcon,
			action: () => goto('/admin/blog/media'),
			keywords: ['media', 'image', 'upload', 'library']
		},
		{
			label: 'Members',
			description: 'Manage members',
			icon: UsersIcon,
			action: () => goto('/admin/members'),
			keywords: ['member', 'user', 'people']
		},
		{
			label: 'Watchlists',
			description: 'Manage watchlists',
			icon: ListChecksIcon,
			action: () => goto('/admin/watchlists'),
			keywords: ['watchlist', 'watch', 'list']
		},
		{
			label: 'Author Profile',
			description: 'Edit your author profile',
			icon: UserCircleIcon,
			action: () => goto('/admin/author'),
			keywords: ['author', 'profile', 'bio']
		}
	];

	let query = $state('');
	let activeIndex = $state(0);
	let inputEl: HTMLInputElement | undefined = $state();

	let filtered = $derived(
		query.trim() === ''
			? ITEMS
			: ITEMS.filter((item) => {
					const q = query.toLowerCase();
					return (
						item.label.toLowerCase().includes(q) ||
						(item.description ?? '').toLowerCase().includes(q) ||
						item.keywords.some((k) => k.includes(q))
					);
				})
	);

	$effect(() => {
		if (open) {
			query = '';
			activeIndex = 0;
			// `await tick()` defers focus until after the DOM has been patched and
			// the input element is mounted — replaces the legacy `setTimeout(..., 30)`.
			tick().then(() => inputEl?.focus());
		}
	});

	function selectItem(item: PaletteItem) {
		item.action();
		onClose();
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (!open) return;
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			activeIndex = (activeIndex + 1) % (filtered.length || 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			activeIndex = (activeIndex - 1 + (filtered.length || 1)) % (filtered.length || 1);
		} else if (e.key === 'Enter') {
			e.preventDefault();
			if (filtered[activeIndex]) selectItem(filtered[activeIndex]);
		} else if (e.key === 'Escape') {
			e.preventDefault();
			onClose();
		}
	}

	$effect(() => {
		window.addEventListener('keydown', handleKeyDown, true);
		return () => window.removeEventListener('keydown', handleKeyDown, true);
	});
</script>

{#if open}
	<div
		class="palette-overlay"
		role="dialog"
		aria-modal="true"
		aria-label="Command palette"
		onclick={onClose}
		onkeydown={(e) => e.key === 'Escape' && onClose()}
		tabindex="-1"
	>
		<div
			class="palette-modal"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="presentation"
		>
			<div class="palette-search">
				<MagnifyingGlassIcon size={18} weight="bold" class="palette-search__icon" />
				<input
					bind:this={inputEl}
					bind:value={query}
					id="admin-command-palette-input"
					name="admin-command-palette"
					type="text"
					class="palette-search__input"
					placeholder="Type a command or search..."
					autocomplete="off"
				/>
				<kbd class="palette-esc">Esc</kbd>
			</div>
			<ul class="palette-list" role="listbox">
				{#if filtered.length === 0}
					<li class="palette-empty">No results for "{query}"</li>
				{:else}
					{#each filtered as item, i (item.label)}
						<li
							class="palette-item"
							class:palette-item--active={i === activeIndex}
							role="option"
							aria-selected={i === activeIndex}
							onmouseenter={() => (activeIndex = i)}
						>
							<button class="palette-item__btn" onclick={() => selectItem(item)}>
								<span class="palette-item__icon"><item.icon size={18} weight="bold" /></span>
								<span class="palette-item__text">
									<span class="palette-item__label">{item.label}</span>
									{#if item.description}
										<span class="palette-item__desc">{item.description}</span>
									{/if}
								</span>
							</button>
						</li>
					{/each}
				{/if}
			</ul>
			<div class="palette-hint">
				<span><kbd>↑↓</kbd> navigate</span>
				<span><kbd>↵</kbd> select</span>
				<span><kbd>Esc</kbd> close</span>
			</div>
		</div>
	</div>
{/if}

<style>
	.palette-overlay {
		position: fixed;
		inset: 0;
		z-index: 10000;
		background: rgba(0, 0, 0, 0.55);
		backdrop-filter: blur(8px);
		-webkit-backdrop-filter: blur(8px);
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding: 10vh 1rem 1rem;
		box-sizing: border-box;
	}

	.palette-modal {
		background: var(--color-navy-deep, #0a0e1a);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-xl);
		width: clamp(320px, 92vw, 560px);
		overflow: hidden;
		box-shadow:
			0 24px 64px rgba(0, 0, 0, 0.5),
			0 1px 0 rgba(255, 255, 255, 0.08) inset;
	}

	.palette-search {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	:global(.palette-search__icon) {
		color: var(--color-grey-500);
		flex-shrink: 0;
	}

	.palette-search__input {
		flex: 1;
		min-width: 0;
		background: transparent;
		border: none;
		outline: none;
		color: var(--color-white, #fff);
		font-size: var(--fs-sm);
	}

	.palette-search__input::placeholder {
		color: var(--color-grey-500, #475569);
	}

	.palette-esc {
		padding: 0.15rem 0.45rem;
		background: rgba(255, 255, 255, 0.07);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-md);
		font-size: var(--fs-2xs);
		color: var(--color-grey-400, #94a3b8);
		flex-shrink: 0;
	}

	.palette-list {
		list-style: none;
		padding: 0.35rem;
		margin: 0;
		max-height: min(340px, 60vh);
		overflow-y: auto;
	}

	.palette-empty {
		padding: 1.5rem;
		text-align: center;
		color: var(--color-grey-500, #475569);
		font-size: var(--fs-sm);
	}

	.palette-item {
		border-radius: var(--radius-md);
		transition: background-color 120ms var(--ease-out);
	}
	.palette-item--active {
		background: rgba(15, 164, 175, 0.14);
	}

	.palette-item + .palette-item {
		margin-top: 1px;
	}

	.palette-item__btn {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		width: 100%;
		padding: 0.55rem 0.75rem;
		background: transparent;
		border: none;
		border-radius: var(--radius-md);
		cursor: pointer;
		text-align: left;
		color: var(--color-grey-200, #e2e8f0);
	}

	.palette-item__btn:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: -2px;
	}

	.palette-item__icon {
		display: flex;
		color: var(--color-teal-light, #15c5d1);
		flex-shrink: 0;
	}

	.palette-item__text {
		display: flex;
		flex-direction: column;
		gap: 0.05rem;
		min-width: 0;
	}

	.palette-item__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
	}

	.palette-item__desc {
		font-size: var(--fs-xs);
		color: var(--color-grey-500, #475569);
	}

	.palette-hint {
		display: none;
		gap: 1rem;
		padding: 0.55rem 1.25rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		font-size: var(--fs-2xs);
		color: var(--color-grey-500, #475569);
	}

	.palette-hint kbd {
		padding: 0.1rem 0.35rem;
		background: rgba(255, 255, 255, 0.07);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-sm);
		font-size: var(--fs-2xs);
	}

	/* Show footer hint and roomier search padding from sm (>=480px) */
	@media (min-width: 480px) {
		.palette-search {
			padding: 1rem 1.25rem;
		}

		.palette-hint {
			display: flex;
		}
	}
</style>
