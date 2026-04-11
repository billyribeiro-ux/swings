<script lang="ts">
	import { Editor, Node } from '@tiptap/core';
	import StarterKit from '@tiptap/starter-kit';
	import Image from '@tiptap/extension-image';
	import { Table } from '@tiptap/extension-table';
	import TableRow from '@tiptap/extension-table-row';
	import TableCell from '@tiptap/extension-table-cell';
	import TableHeader from '@tiptap/extension-table-header';
	import Link from '@tiptap/extension-link';
	import TextAlign from '@tiptap/extension-text-align';
	import Underline from '@tiptap/extension-underline';
	import Color from '@tiptap/extension-color';
	import { TextStyle } from '@tiptap/extension-text-style';
	import Highlight from '@tiptap/extension-highlight';
	import Subscript from '@tiptap/extension-subscript';
	import Superscript from '@tiptap/extension-superscript';
	import TaskList from '@tiptap/extension-task-list';
	import TaskItem from '@tiptap/extension-task-item';
	import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight';
	import CharacterCount from '@tiptap/extension-character-count';
	import Placeholder from '@tiptap/extension-placeholder';
	import Typography from '@tiptap/extension-typography';
	import Youtube from '@tiptap/extension-youtube';
	import { common, createLowlight } from 'lowlight';

	import EditorToolbar from './EditorToolbar.svelte';
	import SlashMenu from './SlashMenu.svelte';

	const ResizableImage = Image.extend({
		addAttributes() {
			return {
				...this.parent?.(),
				width: {
					default: null,
					renderHTML: (attrs: Record<string, unknown>) =>
						attrs.width ? { style: `width:${attrs.width}` } : {}
				}
			};
		},
		addNodeView() {
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			return (({
				node,
				updateAttributes
			}: {
				node: { attrs: Record<string, string> };
				updateAttributes: (a: Record<string, string>) => void;
			}) => {
				const container = document.createElement('div');
				container.className = 'resizable-image-wrap';

				const img = document.createElement('img');
				img.src = node.attrs.src || '';
				img.alt = node.attrs.alt || '';
				img.className = 'resizable-image';
				if (node.attrs.width) img.style.width = node.attrs.width;

				const handle = document.createElement('div');
				handle.className = 'resize-handle';
				handle.contentEditable = 'false';

				let startX = 0;
				let startW = 0;

				handle.addEventListener('mousedown', (e: MouseEvent) => {
					e.preventDefault();
					startX = e.clientX;
					startW = img.offsetWidth;
					const onMove = (ev: MouseEvent) => {
						img.style.width = `${Math.max(48, startW + ev.clientX - startX)}px`;
					};
					const onUp = () => {
						updateAttributes({ width: img.style.width });
						document.removeEventListener('mousemove', onMove);
						document.removeEventListener('mouseup', onUp);
					};
					document.addEventListener('mousemove', onMove);
					document.addEventListener('mouseup', onUp);
				});

				container.appendChild(img);
				container.appendChild(handle);

				return {
					dom: container,
					update(updated: { type: unknown; attrs: Record<string, string> }) {
						if (updated.type !== (node as unknown as { type: unknown }).type) return false;
						img.src = updated.attrs.src || '';
						img.alt = updated.attrs.alt || '';
						if (updated.attrs.width) img.style.width = updated.attrs.width;
						return true;
					}
				};
			}) as unknown as any;
		}
	});

	const ReadMore = Node.create({
		name: 'readMore',
		group: 'block',
		atom: true,
		parseHTML() {
			return [{ tag: 'div[data-read-more]' }];
		},
		renderHTML() {
			return ['div', { 'data-read-more': 'true', class: 'read-more-break' }];
		}
	});

	interface Props {
		content?: string;
		contentJson?: Record<string, unknown> | null;
		onUpdate?: (html: string, json: Record<string, unknown>) => void;
		onWordCount?: (words: number, chars: number) => void;
		placeholder?: string;
		onInsertImage?: () => void;
	}

	let {
		content = '',
		contentJson = null,
		onUpdate,
		onWordCount,
		placeholder = 'Start writing your post...',
		onInsertImage
	}: Props = $props();

	let editorElement: HTMLElement | undefined = $state();
	// Tiptap Editor is a heavy non-reactive instance — `$state.raw` skips the
	// proxy wrapper, which is wasteful (and can confuse ProseMirror identity
	// checks) for an opaque object that is only ever reassigned wholesale.
	let editor: Editor | undefined = $state.raw();
	let isFullscreen = $state(false);
	let showSource = $state(false);
	let sourceHtml = $state('');

	// Slash command state
	let slashActive = $state(false);
	let slashX = $state(0);
	let slashY = $state(0);
	let slashQuery = $state('');
	let slashAnchorPos = $state(0);

	function checkSlash(e: Editor) {
		const { from } = e.state.selection;
		const resolvedFrom = e.state.doc.resolve(from);
		const nodeStart = from - resolvedFrom.parentOffset;
		const textBeforeCursor = e.state.doc.textBetween(nodeStart, from, '\n', '\0');

		const match = textBeforeCursor.match(/(?:^|\s)(\/(\w*)$)/);
		if (match) {
			const slashPart = match[1];
			slashQuery = match[2];
			slashAnchorPos = from - slashPart.length;
			try {
				const coords = e.view.coordsAtPos(from);
				slashX = coords.left;
				slashY = coords.bottom + 6;
				slashActive = true;
			} catch {
				slashActive = false;
			}
		} else {
			slashActive = false;
		}
	}

	const lowlight = createLowlight(common);

	$effect(() => {
		if (!editorElement) return;

		const e = new Editor({
			element: editorElement,
			extensions: [
				StarterKit.configure({
					codeBlock: false,
					heading: { levels: [1, 2, 3, 4, 5, 6] },
					link: false,
					underline: false
				}),
				ResizableImage.configure({ inline: false, allowBase64: true }),
				Table.configure({ resizable: true }),
				TableRow,
				TableCell,
				TableHeader,
				Link.configure({
					openOnClick: false,
					HTMLAttributes: { rel: 'noopener noreferrer' }
				}),
				TextAlign.configure({ types: ['heading', 'paragraph'] }),
				Underline,
				TextStyle,
				Color,
				Highlight.configure({ multicolor: true }),
				Subscript,
				Superscript,
				TaskList,
				TaskItem.configure({ nested: true }),
				CodeBlockLowlight.configure({ lowlight }),
				CharacterCount,
				Placeholder.configure({ placeholder }),
				Typography,
				Youtube.configure({ inline: false, ccLanguage: 'en' }),
				ReadMore
			],
			content: contentJson || content || '',
			editorProps: {
				attributes: {
					class: 'blog-editor__content'
				}
			},
			onUpdate: ({ editor: ed }) => {
				const html = ed.getHTML();
				const json = ed.getJSON();
				onUpdate?.(html, json as Record<string, unknown>);
				const storage = ed.storage.characterCount;
				onWordCount?.(storage.words(), storage.characters());
				checkSlash(ed);
			},
			onSelectionUpdate: ({ editor: ed }) => {
				if (slashActive) checkSlash(ed);
			}
		});
		editor = e;

		// Initial word count
		const storage = e.storage.characterCount;
		onWordCount?.(storage.words(), storage.characters());

		return () => {
			e.destroy();
		};
	});

	function toggleFullscreen() {
		isFullscreen = !isFullscreen;
	}

	function toggleSource() {
		if (!editor) return;
		if (showSource) {
			editor.commands.setContent(sourceHtml);
			showSource = false;
		} else {
			sourceHtml = editor.getHTML();
			showSource = true;
		}
	}

	function handleSourceInput(e: Event) {
		sourceHtml = (e.target as HTMLTextAreaElement).value;
	}

	export function getEditor(): Editor | undefined {
		return editor;
	}

	export function setContent(html: string) {
		editor?.commands.setContent(html);
	}

	export function insertImage(src: string, alt: string = '') {
		editor?.chain().focus().setImage({ src, alt }).run();
	}

	export function insertReadMore() {
		editor?.chain().focus().insertContent({ type: 'readMore' }).run();
	}
</script>

<div class="blog-editor" class:blog-editor--fullscreen={isFullscreen}>
	{#if editor}
		<EditorToolbar
			{editor}
			{isFullscreen}
			{showSource}
			onToggleFullscreen={toggleFullscreen}
			onToggleSource={toggleSource}
			{onInsertImage}
			onInsertReadMore={insertReadMore}
		/>
	{/if}

	{#if showSource}
		<textarea
			id="blog-source-editor"
			name="blog-source"
			class="blog-editor__source"
			value={sourceHtml}
			oninput={handleSourceInput}
			spellcheck="false"
		></textarea>
	{:else}
		<div class="blog-editor__wrapper" bind:this={editorElement}></div>
	{/if}
</div>

{#if editor && slashActive}
	<SlashMenu
		{editor}
		visible={slashActive}
		x={slashX}
		y={slashY}
		query={slashQuery}
		anchorPos={slashAnchorPos}
		onClose={() => (slashActive = false)}
		{onInsertImage}
	/>
{/if}

<style>
	.blog-editor {
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.5rem;
		background-color: rgba(255, 255, 255, 0.03);
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.blog-editor--fullscreen {
		position: fixed;
		inset: 0;
		z-index: 9999;
		border-radius: 0;
		background-color: var(--color-navy-deep, #0a0e1a);
	}

	.blog-editor__wrapper {
		flex: 1;
		overflow-y: auto;
		min-height: 400px;
	}

	.blog-editor--fullscreen .blog-editor__wrapper {
		min-height: 0;
	}

	.blog-editor__source {
		flex: 1;
		min-height: 400px;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.3);
		color: #a5d6ff;
		border: none;
		font-family: 'Courier New', Courier, monospace;
		font-size: 0.875rem;
		line-height: 1.6;
		resize: none;
		outline: none;
		white-space: pre-wrap;
		word-wrap: break-word;
	}

	/* Editor content styles */
	.blog-editor :global(.blog-editor__content) {
		padding: 1.5rem;
		min-height: 400px;
		color: var(--color-grey-200, #e2e8f0);
		font-size: 1rem;
		line-height: 1.75;
		outline: none;
	}

	.blog-editor :global(.blog-editor__content > *:first-child) {
		margin-top: 0;
	}

	.blog-editor :global(.blog-editor__content h1) {
		font-size: 2rem;
		font-weight: 700;
		margin: 1.5rem 0 0.75rem;
		color: var(--color-white, #fff);
	}

	.blog-editor :global(.blog-editor__content h2) {
		font-size: 1.5rem;
		font-weight: 700;
		margin: 1.25rem 0 0.5rem;
		color: var(--color-white, #fff);
	}

	.blog-editor :global(.blog-editor__content h3) {
		font-size: 1.25rem;
		font-weight: 600;
		margin: 1rem 0 0.5rem;
		color: var(--color-white, #fff);
	}

	.blog-editor :global(.blog-editor__content h4),
	.blog-editor :global(.blog-editor__content h5),
	.blog-editor :global(.blog-editor__content h6) {
		font-size: 1.1rem;
		font-weight: 600;
		margin: 0.75rem 0 0.5rem;
		color: var(--color-white, #fff);
	}

	.blog-editor :global(.blog-editor__content p) {
		margin: 0 0 0.75rem;
	}

	.blog-editor :global(.blog-editor__content a) {
		color: var(--color-teal-light, #15c5d1);
		text-decoration: underline;
	}

	.blog-editor :global(.blog-editor__content blockquote) {
		border-left: 3px solid var(--color-teal, #0fa4af);
		padding-left: 1rem;
		margin: 1rem 0;
		color: var(--color-grey-300, #94a3b8);
		font-style: italic;
	}

	.blog-editor :global(.blog-editor__content ul),
	.blog-editor :global(.blog-editor__content ol) {
		padding-left: 1.5rem;
		margin: 0.5rem 0;
	}

	.blog-editor :global(.blog-editor__content li) {
		margin: 0.25rem 0;
	}

	.blog-editor :global(.blog-editor__content ul[data-type='taskList']) {
		list-style: none;
		padding-left: 0;
	}

	.blog-editor :global(.blog-editor__content ul[data-type='taskList'] li) {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.blog-editor :global(.blog-editor__content ul[data-type='taskList'] li label) {
		margin-top: 0.15rem;
	}

	.blog-editor :global(.blog-editor__content code) {
		background: rgba(255, 255, 255, 0.08);
		border-radius: 0.25rem;
		padding: 0.15rem 0.4rem;
		font-size: 0.875em;
		color: #e2e8f0;
	}

	.blog-editor :global(.blog-editor__content pre) {
		background: rgba(0, 0, 0, 0.4);
		border-radius: 0.5rem;
		padding: 1rem;
		margin: 1rem 0;
		overflow-x: auto;
	}

	.blog-editor :global(.blog-editor__content pre code) {
		background: none;
		padding: 0;
		font-size: 0.875rem;
		line-height: 1.5;
	}

	.blog-editor :global(.blog-editor__content hr) {
		border: none;
		border-top: 1px solid rgba(255, 255, 255, 0.1);
		margin: 1.5rem 0;
	}

	.blog-editor :global(.blog-editor__content .read-more-break) {
		position: relative;
		border: 1.5px dashed rgba(255, 193, 7, 0.6);
		border-radius: 0.25rem;
		margin: 1.5rem 0;
		text-align: center;
		padding: 0.25rem 0;
		color: rgba(255, 193, 7, 0.8);
		font-size: 0.75rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		cursor: default;
		user-select: none;
	}

	.blog-editor :global(.blog-editor__content .read-more-break::before) {
		content: '✂ Read More Break';
	}

	.blog-editor :global(.blog-editor__content img) {
		max-width: 100%;
		height: auto;
		border-radius: 0.5rem;
		margin: 1rem 0;
	}

	.blog-editor :global(.resizable-image-wrap) {
		position: relative;
		display: inline-block;
		max-width: 100%;
	}

	.blog-editor :global(.resizable-image) {
		display: block;
		max-width: 100%;
		height: auto;
		border-radius: 0.5rem;
	}

	.blog-editor :global(.resize-handle) {
		position: absolute;
		bottom: 4px;
		right: 4px;
		width: 12px;
		height: 12px;
		background: rgba(15, 164, 175, 0.85);
		border-radius: 2px;
		cursor: se-resize;
		opacity: 0;
		transition: opacity 0.15s;
	}

	.blog-editor :global(.resizable-image-wrap:hover .resize-handle) {
		opacity: 1;
	}

	.blog-editor :global(.blog-editor__content table) {
		border-collapse: collapse;
		width: 100%;
		margin: 1rem 0;
	}

	.blog-editor :global(.blog-editor__content th),
	.blog-editor :global(.blog-editor__content td) {
		border: 1px solid rgba(255, 255, 255, 0.15);
		padding: 0.5rem 0.75rem;
		text-align: left;
	}

	.blog-editor :global(.blog-editor__content th) {
		background: rgba(255, 255, 255, 0.06);
		font-weight: 600;
	}

	.blog-editor :global(.blog-editor__content .ProseMirror-selectednode) {
		outline: 2px solid var(--color-teal, #0fa4af);
		outline-offset: 2px;
	}

	/* Placeholder */
	.blog-editor :global(.blog-editor__content p.is-editor-empty:first-child::before) {
		content: attr(data-placeholder);
		float: left;
		color: rgba(255, 255, 255, 0.25);
		pointer-events: none;
		height: 0;
	}

	/* YouTube embed */
	.blog-editor :global(.blog-editor__content div[data-youtube-video]) {
		position: relative;
		padding-bottom: 56.25%;
		height: 0;
		overflow: hidden;
		margin: 1rem 0;
		border-radius: 0.5rem;
	}

	.blog-editor :global(.blog-editor__content div[data-youtube-video] iframe) {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
	}

	/* Mark styles */
	.blog-editor :global(.blog-editor__content mark) {
		background-color: rgba(255, 255, 0, 0.3);
		border-radius: 0.15rem;
		padding: 0 0.1rem;
	}

	.blog-editor :global(.blog-editor__content sub) {
		font-size: 0.75em;
	}

	.blog-editor :global(.blog-editor__content sup) {
		font-size: 0.75em;
	}

	.blog-editor :global(.blog-editor__content s) {
		text-decoration: line-through;
	}

	.blog-editor :global(.blog-editor__content u) {
		text-decoration: underline;
	}
</style>
