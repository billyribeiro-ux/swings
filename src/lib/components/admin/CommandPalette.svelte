<script lang="ts">
	import { tick } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		MagnifyingGlass,
		ChartBar,
		PresentationChart,
		Users,
		Article,
		PlusCircle,
		Tag,
		FolderOpen,
		Image,
		ListChecks,
		UserCircle
	} from 'phosphor-svelte';

	interface PaletteItem {
		label: string;
		description?: string;
		icon: typeof MagnifyingGlass;
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
			icon: ChartBar,
			action: () => goto('/admin'),
			keywords: ['home', 'dash', 'overview', 'stats']
		},
		{
			label: 'Analytics',
			description: 'Traffic, pages, CTR',
			icon: PresentationChart,
			action: () => goto('/admin/analytics'),
			keywords: ['analytics', 'traffic', 'views', 'ctr', 'charts', 'three']
		},
		{
			label: 'Posts',
			description: 'Manage blog posts',
			icon: Article,
			action: () => goto('/admin/blog'),
			keywords: ['post', 'blog', 'articles']
		},
		{
			label: 'New Post',
			description: 'Create a new blog post',
			icon: PlusCircle,
			action: () => goto('/admin/blog/new'),
			keywords: ['new', 'create', 'write', 'post']
		},
		{
			label: 'Categories',
			description: 'Manage blog categories',
			icon: FolderOpen,
			action: () => goto('/admin/blog/categories'),
			keywords: ['category', 'categories', 'folder']
		},
		{
			label: 'Tags',
			description: 'Manage post tags',
			icon: Tag,
			action: () => goto('/admin/blog/tags'),
			keywords: ['tag', 'tags', 'label']
		},
		{
			label: 'Media',
			description: 'Media library',
			icon: Image,
			action: () => goto('/admin/blog/media'),
			keywords: ['media', 'image', 'upload', 'library']
		},
		{
			label: 'Members',
			description: 'Manage members',
			icon: Users,
			action: () => goto('/admin/members'),
			keywords: ['member', 'user', 'people']
		},
		{
			label: 'Watchlists',
			description: 'Manage watchlists',
			icon: ListChecks,
			action: () => goto('/admin/watchlists'),
			keywords: ['watchlist', 'watch', 'list']
		},
		{
			label: 'Author Profile',
			description: 'Edit your author profile',
			icon: UserCircle,
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
				<MagnifyingGlass size={18} weight="bold" class="palette-search__icon" />
				<input
					bind:this={inputEl}
					bind:value={query}
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
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding-top: 15vh;
	}

	.palette-modal {
		background: var(--color-navy-deep, #0a0e1a);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.75rem;
		width: 100%;
		max-width: 520px;
		overflow: hidden;
		box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6);
	}

	.palette-search {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 1rem 1.25rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	.palette-search__input {
		flex: 1;
		background: transparent;
		border: none;
		outline: none;
		color: var(--color-white, #fff);
		font-size: 1rem;
	}

	.palette-search__input::placeholder {
		color: var(--color-grey-500, #475569);
	}

	.palette-esc {
		padding: 0.2rem 0.45rem;
		background: rgba(255, 255, 255, 0.07);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.3rem;
		font-size: 0.7rem;
		color: var(--color-grey-400, #94a3b8);
	}

	.palette-list {
		list-style: none;
		padding: 0.35rem;
		margin: 0;
		max-height: 340px;
		overflow-y: auto;
	}

	.palette-empty {
		padding: 1.5rem;
		text-align: center;
		color: var(--color-grey-500, #475569);
		font-size: 0.85rem;
	}

	.palette-item {
		border-radius: 0.4rem;
	}
	.palette-item--active {
		background: rgba(15, 164, 175, 0.12);
	}

	.palette-item__btn {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		width: 100%;
		padding: 0.6rem 0.75rem;
		background: transparent;
		border: none;
		border-radius: 0.4rem;
		cursor: pointer;
		text-align: left;
		color: var(--color-grey-200, #e2e8f0);
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
	}

	.palette-item__label {
		font-size: 0.875rem;
		font-weight: 600;
	}

	.palette-item__desc {
		font-size: 0.7rem;
		color: var(--color-grey-500, #475569);
	}

	.palette-hint {
		display: flex;
		gap: 1rem;
		padding: 0.5rem 1.25rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		font-size: 0.7rem;
		color: var(--color-grey-500, #475569);
	}

	.palette-hint kbd {
		padding: 0.1rem 0.35rem;
		background: rgba(255, 255, 255, 0.07);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.2rem;
		font-size: 0.65rem;
	}
</style>
