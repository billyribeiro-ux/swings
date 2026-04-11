<script lang="ts">
	import type { Editor } from '@tiptap/core';
	import {
		TextT,
		TextHOne,
		TextHTwo,
		TextHThree,
		ListBullets,
		ListNumbers,
		CheckSquare,
		Quotes,
		Code,
		Minus,
		Image,
		YoutubeLogo
	} from 'phosphor-svelte';

	interface SlashCommand {
		label: string;
		description: string;
		keywords: string[];
		icon: typeof TextT;
		action: (editor: Editor) => void;
	}

	interface Props {
		editor: Editor;
		visible: boolean;
		x: number;
		y: number;
		query: string;
		anchorPos: number;
		onClose: () => void;
		onInsertImage?: () => void;
	}

	let { editor, visible, x, y, query, anchorPos, onClose, onInsertImage }: Props = $props();

	const COMMANDS: SlashCommand[] = [
		{
			label: 'Paragraph',
			description: 'Plain paragraph text',
			keywords: ['p', 'text', 'paragraph', 'normal'],
			icon: TextT,
			action: (e) => e.chain().focus().setParagraph().run()
		},
		{
			label: 'Heading 1',
			description: 'Large section heading',
			keywords: ['h1', 'heading', 'title'],
			icon: TextHOne,
			action: (e) => e.chain().focus().toggleHeading({ level: 1 }).run()
		},
		{
			label: 'Heading 2',
			description: 'Medium section heading',
			keywords: ['h2', 'heading', 'subtitle'],
			icon: TextHTwo,
			action: (e) => e.chain().focus().toggleHeading({ level: 2 }).run()
		},
		{
			label: 'Heading 3',
			description: 'Small section heading',
			keywords: ['h3', 'heading'],
			icon: TextHThree,
			action: (e) => e.chain().focus().toggleHeading({ level: 3 }).run()
		},
		{
			label: 'Bullet List',
			description: 'Unordered list',
			keywords: ['bullet', 'list', 'ul', 'unordered'],
			icon: ListBullets,
			action: (e) => e.chain().focus().toggleBulletList().run()
		},
		{
			label: 'Numbered List',
			description: 'Ordered list',
			keywords: ['numbered', 'ordered', 'list', 'ol'],
			icon: ListNumbers,
			action: (e) => e.chain().focus().toggleOrderedList().run()
		},
		{
			label: 'Task List',
			description: 'Checklist with checkboxes',
			keywords: ['task', 'todo', 'checklist', 'checkbox'],
			icon: CheckSquare,
			action: (e) => e.chain().focus().toggleTaskList().run()
		},
		{
			label: 'Blockquote',
			description: 'Pull quote or citation',
			keywords: ['quote', 'blockquote', 'citation'],
			icon: Quotes,
			action: (e) => e.chain().focus().toggleBlockquote().run()
		},
		{
			label: 'Code Block',
			description: 'Syntax-highlighted code',
			keywords: ['code', 'pre', 'codeblock', 'block'],
			icon: Code,
			action: (e) => e.chain().focus().toggleCodeBlock().run()
		},
		{
			label: 'Divider',
			description: 'Horizontal rule',
			keywords: ['hr', 'divider', 'rule', 'separator'],
			icon: Minus,
			action: (e) => e.chain().focus().setHorizontalRule().run()
		},
		{
			label: 'Image',
			description: 'Insert image from media library',
			keywords: ['image', 'photo', 'picture', 'media'],
			icon: Image,
			action: () => onInsertImage?.()
		},
		{
			label: 'YouTube',
			description: 'Embed a YouTube video',
			keywords: ['youtube', 'video', 'embed'],
			icon: YoutubeLogo,
			action: () => {}
		}
	];

	let activeIndex = $state(0);
	let menuEl: HTMLElement | undefined = $state();

	let filtered = $derived(
		query.trim() === ''
			? COMMANDS
			: COMMANDS.filter((c) => {
					const q = query.toLowerCase();
					return c.label.toLowerCase().includes(q) || c.keywords.some((k) => k.startsWith(q));
				})
	);

	$effect(() => {
		// Reset highlight whenever the query changes — read it explicitly so the
		// effect actually depends on it (the assignment alone is not a dep).
		query;
		activeIndex = 0;
	});

	function selectCommand(cmd: SlashCommand) {
		const { from } = editor.state.selection;
		const deleteFrom = anchorPos;
		const deleteTo = from;

		editor.chain().focus().deleteRange({ from: deleteFrom, to: deleteTo }).run();

		if (cmd.label === 'Image') {
			onInsertImage?.();
		} else {
			cmd.action(editor);
		}
		onClose();
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (!visible) return;
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			e.stopPropagation();
			activeIndex = (activeIndex + 1) % filtered.length;
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			e.stopPropagation();
			activeIndex = (activeIndex - 1 + filtered.length) % filtered.length;
		} else if (e.key === 'Enter') {
			e.preventDefault();
			e.stopPropagation();
			if (filtered[activeIndex]) selectCommand(filtered[activeIndex]);
		} else if (e.key === 'Escape') {
			e.preventDefault();
			e.stopPropagation();
			onClose();
		}
	}

	$effect(() => {
		window.addEventListener('keydown', handleKeyDown, true);
		return () => window.removeEventListener('keydown', handleKeyDown, true);
	});

	$effect(() => {
		// Re-run when activeIndex or visible change — DOM queries inside an effect are
		// not tracked, so we read activeIndex explicitly to register the dep.
		activeIndex;
		if (visible && menuEl) {
			const item = menuEl.querySelector<HTMLElement>('.slash-item--active');
			item?.scrollIntoView({ block: 'nearest' });
		}
	});
</script>

{#if visible && filtered.length > 0}
	<div
		class="slash-menu"
		style="left:{x}px;top:{y}px"
		bind:this={menuEl}
		role="listbox"
		aria-label="Slash commands"
	>
		{#each filtered as cmd, i (cmd.label)}
			<button
				class="slash-item"
				class:slash-item--active={i === activeIndex}
				role="option"
				aria-selected={i === activeIndex}
				onclick={() => selectCommand(cmd)}
				onmouseenter={() => (activeIndex = i)}
			>
				<span class="slash-item__icon"><cmd.icon size={18} weight="bold" /></span>
				<span class="slash-item__text">
					<span class="slash-item__label">{cmd.label}</span>
					<span class="slash-item__desc">{cmd.description}</span>
				</span>
			</button>
		{/each}
	</div>
{/if}

<style>
	.slash-menu {
		position: fixed;
		z-index: 9999;
		background: var(--color-navy-deep, #0a0e1a);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.6rem;
		padding: 0.35rem;
		min-width: 220px;
		max-height: 320px;
		overflow-y: auto;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
	}

	.slash-item {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		width: 100%;
		padding: 0.45rem 0.6rem;
		background: transparent;
		border: none;
		border-radius: 0.4rem;
		cursor: pointer;
		text-align: left;
		color: var(--color-grey-200, #e2e8f0);
	}

	.slash-item--active,
	.slash-item:hover {
		background: rgba(15, 164, 175, 0.15);
	}

	.slash-item__icon {
		display: flex;
		align-items: center;
		color: var(--color-teal-light, #15c5d1);
		flex-shrink: 0;
	}

	.slash-item__text {
		display: flex;
		flex-direction: column;
		gap: 0.05rem;
	}

	.slash-item__label {
		font-size: 0.85rem;
		font-weight: 600;
	}

	.slash-item__desc {
		font-size: 0.7rem;
		color: var(--color-grey-500, #475569);
	}
</style>
