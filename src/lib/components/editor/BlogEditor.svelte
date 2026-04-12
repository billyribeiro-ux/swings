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
		autosaveStatus?: 'idle' | 'pending' | 'saving' | 'saved' | 'error';
		lastSavedAt?: Date | null;
		revisions?: Array<{ id: string; revision_number: number; content: string; created_at: string; author_name: string }>;
		focusKeyword?: string;
		metaTitle?: string;
		metaDescription?: string;
	}

	let {
		content = '',
		contentJson = null,
		onUpdate,
		onWordCount,
		placeholder = 'Start writing your post...',
		onInsertImage,
		autosaveStatus = 'idle',
		lastSavedAt = null,
		revisions = [],
		focusKeyword = '',
		metaTitle = '',
		metaDescription = ''
	}: Props = $props();

	let editorElement: HTMLElement | undefined = $state();
	// Tiptap Editor is a heavy non-reactive instance — `$state.raw` skips the
	// proxy wrapper, which is wasteful (and can confuse ProseMirror identity
	// checks) for an opaque object that is only ever reassigned wholesale.
	let editor: Editor | undefined = $state.raw();
	let isFullscreen = $state(false);
	let isDistractionFree = $state(false);
	let showSource = $state(false);
	let sourceHtml = $state('');
	let showSeoPanel = $state(false);
	let showRevisionDiff = $state(false);
	let selectedRevisionId = $state('');
	let wordCount = $state(0);
	let charCount = $state(0);
	let seoFocusKeyword = $state('');

	// Derived SEO metrics
	const seoTitleLength = $derived(metaTitle.length);
	const seoTitleStatus = $derived(
		seoTitleLength >= 50 && seoTitleLength <= 60 ? 'good' :
		seoTitleLength > 0 ? 'warn' : 'none'
	);
	const seoDescLength = $derived(metaDescription.length);
	const seoDescStatus = $derived(
		seoDescLength >= 150 && seoDescLength <= 160 ? 'good' :
		seoDescLength > 0 ? 'warn' : 'none'
	);

	// Keyword density
	const keywordDensity = $derived.by(() => {
		const kw = (seoFocusKeyword || focusKeyword).toLowerCase().trim();
		if (!kw || wordCount === 0) return 0;
		const text = editor?.getText()?.toLowerCase() || '';
		const matches = text.split(kw).length - 1;
		return Math.round((matches / wordCount) * 100 * 10) / 10;
	});

	// Simple Flesch-Kincaid readability approximation
	const readabilityScore = $derived.by(() => {
		const text = editor?.getText() || '';
		if (!text.trim()) return 0;
		const sentences = text.split(/[.!?]+/).filter(s => s.trim().length > 0);
		const words = text.split(/\s+/).filter(w => w.length > 0);
		if (words.length === 0 || sentences.length === 0) return 0;
		// Count syllables (rough approximation)
		function countSyllables(word: string): number {
			word = word.toLowerCase().replace(/[^a-z]/g, '');
			if (word.length <= 3) return 1;
			const vowelGroups = word.match(/[aeiouy]+/g);
			let count = vowelGroups ? vowelGroups.length : 1;
			if (word.endsWith('e')) count = Math.max(1, count - 1);
			return count;
		}
		const totalSyllables = words.reduce((acc, w) => acc + countSyllables(w), 0);
		// Flesch Reading Ease = 206.835 - 1.015*(words/sentences) - 84.6*(syllables/words)
		const score = 206.835 - 1.015 * (words.length / sentences.length) - 84.6 * (totalSyllables / words.length);
		return Math.round(Math.max(0, Math.min(100, score)));
	});

	const readabilityLabel = $derived(
		readabilityScore >= 80 ? 'Very Easy' :
		readabilityScore >= 60 ? 'Easy' :
		readabilityScore >= 40 ? 'Moderate' :
		readabilityScore >= 20 ? 'Difficult' :
		'Very Difficult'
	);

	const readabilityColor = $derived(
		readabilityScore >= 60 ? '#22c55e' :
		readabilityScore >= 40 ? '#f59e0b' :
		'#ef4444'
	);

	// Autosave time ago
	const autosaveLabel = $derived.by(() => {
		if (autosaveStatus === 'saving') return 'Saving...';
		if (autosaveStatus === 'pending') return 'Unsaved changes';
		if (autosaveStatus === 'error') return 'Save failed';
		if (autosaveStatus === 'saved' && lastSavedAt) {
			const diff = Math.floor((Date.now() - lastSavedAt.getTime()) / 1000);
			if (diff < 10) return 'Saved just now';
			if (diff < 60) return `Saved ${diff}s ago`;
			const mins = Math.floor(diff / 60);
			if (mins < 60) return `Saved ${mins} minute${mins > 1 ? 's' : ''} ago`;
			return `Saved ${Math.floor(mins / 60)}h ago`;
		}
		return '';
	});

	// Revision diff
	const selectedRevision = $derived(revisions.find(r => r.id === selectedRevisionId));
	const diffHtml = $derived.by(() => {
		if (!selectedRevision || !editor) return '';
		const currentText = editor.getText();
		const revText = stripHtml(selectedRevision.content);
		return computeInlineDiff(revText, currentText);
	});

	function stripHtml(html: string): string {
		const div = document.createElement('div');
		div.innerHTML = html;
		return div.textContent || div.innerText || '';
	}

	function computeInlineDiff(oldText: string, newText: string): string {
		const oldWords = oldText.split(/\s+/);
		const newWords = newText.split(/\s+/);
		// Simple word-level LCS diff
		const m = oldWords.length;
		const n = newWords.length;
		// For large texts, limit to first 500 words each to avoid perf issues
		const maxLen = 500;
		const oW = oldWords.slice(0, maxLen);
		const nW = newWords.slice(0, maxLen);
		const ml = oW.length;
		const nl = nW.length;

		// Build LCS table
		const dp: number[][] = Array.from({ length: ml + 1 }, () => new Array(nl + 1).fill(0));
		for (let i = 1; i <= ml; i++) {
			for (let j = 1; j <= nl; j++) {
				if (oW[i - 1] === nW[j - 1]) {
					dp[i][j] = dp[i - 1][j - 1] + 1;
				} else {
					dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
				}
			}
		}

		// Backtrack to build diff
		const parts: string[] = [];
		let i = ml, j = nl;
		const result: Array<{ type: 'same' | 'del' | 'ins'; word: string }> = [];
		while (i > 0 || j > 0) {
			if (i > 0 && j > 0 && oW[i - 1] === nW[j - 1]) {
				result.unshift({ type: 'same', word: oW[i - 1] });
				i--; j--;
			} else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
				result.unshift({ type: 'ins', word: nW[j - 1] });
				j--;
			} else {
				result.unshift({ type: 'del', word: oW[i - 1] });
				i--;
			}
		}

		for (const r of result) {
			if (r.type === 'del') {
				parts.push(`<span class="diff-del">${r.word}</span>`);
			} else if (r.type === 'ins') {
				parts.push(`<span class="diff-ins">${r.word}</span>`);
			} else {
				parts.push(r.word);
			}
		}

		if (m > maxLen || n > maxLen) {
			parts.push(' <em>[... diff truncated for performance]</em>');
		}

		return parts.join(' ');
	}

	function generateToc() {
		if (!editor) return;
		const doc = editor.state.doc;
		const headings: Array<{ level: number; text: string; id: string }> = [];

		doc.descendants((node) => {
			if (node.type.name === 'heading') {
				const text = node.textContent;
				const id = text.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
				headings.push({ level: node.attrs.level, text, id });
			}
		});

		if (headings.length === 0) return;

		let tocHtml = '<div class="toc-block"><h4>Table of Contents</h4><ul>';
		for (const h of headings) {
			const indent = h.level > 2 ? ' style="margin-left: ' + ((h.level - 2) * 1) + 'rem"' : '';
			tocHtml += `<li${indent}><a href="#${h.id}">${h.text}</a></li>`;
		}
		tocHtml += '</ul></div>';

		editor.chain().focus().insertContent(tocHtml).run();
	}

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
				wordCount = storage.words();
				charCount = storage.characters();
				onWordCount?.(wordCount, charCount);
				checkSlash(ed);
			},
			onSelectionUpdate: ({ editor: ed }) => {
				if (slashActive) checkSlash(ed);
			}
		});
		editor = e;

		// Initial word count
		const storage = e.storage.characterCount;
		wordCount = storage.words();
		charCount = storage.characters();
		onWordCount?.(wordCount, charCount);

		return () => {
			e.destroy();
		};
	});

	function toggleFullscreen() {
		isFullscreen = !isFullscreen;
		if (isFullscreen) isDistractionFree = false;
	}

	function toggleDistractionFree() {
		isDistractionFree = !isDistractionFree;
		if (isDistractionFree) {
			isFullscreen = true;
		}
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

<div class="blog-editor" class:blog-editor--fullscreen={isFullscreen} class:blog-editor--distraction-free={isDistractionFree}>
	{#if editor}
		{#if !isDistractionFree}
			<EditorToolbar
				{editor}
				{isFullscreen}
				{showSource}
				{isDistractionFree}
				{wordCount}
				{charCount}
				onToggleFullscreen={toggleFullscreen}
				onToggleSource={toggleSource}
				onToggleDistractionFree={toggleDistractionFree}
				onInsertToc={generateToc}
				{onInsertImage}
				onInsertReadMore={insertReadMore}
			/>
		{:else}
			<!-- Minimal bar in distraction-free mode -->
			<div class="blog-editor__df-bar">
				<span class="blog-editor__df-wc">{wordCount} words</span>
				<button class="blog-editor__df-exit" onclick={toggleDistractionFree}>
					Exit distraction-free mode
				</button>
			</div>
		{/if}
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

	<!-- Autosave indicator -->
	{#if autosaveLabel}
		<div class="blog-editor__autosave-bar" class:blog-editor__autosave-bar--saved={autosaveStatus === 'saved'} class:blog-editor__autosave-bar--pending={autosaveStatus === 'pending'} class:blog-editor__autosave-bar--error={autosaveStatus === 'error'}>
			{#if autosaveStatus === 'saved'}
				<span class="blog-editor__autosave-dot blog-editor__autosave-dot--green"></span>
			{:else if autosaveStatus === 'pending'}
				<span class="blog-editor__autosave-dot blog-editor__autosave-dot--amber"></span>
			{:else if autosaveStatus === 'error'}
				<span class="blog-editor__autosave-dot blog-editor__autosave-dot--red"></span>
			{:else}
				<span class="blog-editor__autosave-dot"></span>
			{/if}
			<span>{autosaveLabel}</span>
		</div>
	{/if}

	<!-- SEO Analysis Panel -->
	{#if !isDistractionFree}
		<div class="blog-editor__seo-toggle">
			<button class="blog-editor__panel-btn" onclick={() => (showSeoPanel = !showSeoPanel)}>
				{showSeoPanel ? 'Hide' : 'Show'} SEO Analysis
			</button>
			{#if revisions.length > 0}
				<button class="blog-editor__panel-btn" onclick={() => (showRevisionDiff = !showRevisionDiff)}>
					{showRevisionDiff ? 'Hide' : 'Show'} Revision Diff
				</button>
			{/if}
		</div>
	{/if}

	{#if showSeoPanel && !isDistractionFree}
		<div class="blog-editor__seo-panel">
			<h4 class="seo-panel__title">SEO Analysis</h4>

			<div class="seo-panel__field">
				<label class="seo-panel__label" for="seo-focus-kw">Focus Keyword</label>
				<input
					id="seo-focus-kw"
					name="seo-focus-kw"
					type="text"
					class="seo-panel__input"
					bind:value={seoFocusKeyword}
					placeholder="Enter focus keyword..."
				/>
			</div>

			<div class="seo-panel__metrics">
				<div class="seo-panel__metric">
					<span class="seo-panel__metric-label">Title Length</span>
					<div class="seo-panel__bar-wrap">
						<div
							class="seo-panel__bar"
							class:seo-panel__bar--good={seoTitleStatus === 'good'}
							class:seo-panel__bar--warn={seoTitleStatus === 'warn'}
							style="width: {Math.min(100, (seoTitleLength / 70) * 100)}%"
						></div>
					</div>
					<span class="seo-panel__metric-value" class:seo-panel__metric-value--good={seoTitleStatus === 'good'} class:seo-panel__metric-value--warn={seoTitleStatus === 'warn'}>
						{seoTitleLength}/60 chars {seoTitleStatus === 'good' ? '(ideal)' : seoTitleLength > 60 ? '(too long)' : seoTitleLength > 0 ? '(too short)' : ''}
					</span>
				</div>

				<div class="seo-panel__metric">
					<span class="seo-panel__metric-label">Meta Description</span>
					<div class="seo-panel__bar-wrap">
						<div
							class="seo-panel__bar"
							class:seo-panel__bar--good={seoDescStatus === 'good'}
							class:seo-panel__bar--warn={seoDescStatus === 'warn'}
							style="width: {Math.min(100, (seoDescLength / 180) * 100)}%"
						></div>
					</div>
					<span class="seo-panel__metric-value" class:seo-panel__metric-value--good={seoDescStatus === 'good'} class:seo-panel__metric-value--warn={seoDescStatus === 'warn'}>
						{seoDescLength}/160 chars {seoDescStatus === 'good' ? '(ideal)' : seoDescLength > 160 ? '(too long)' : seoDescLength > 0 ? '(too short)' : ''}
					</span>
				</div>

				<div class="seo-panel__metric">
					<span class="seo-panel__metric-label">Keyword Density</span>
					<span class="seo-panel__metric-value" class:seo-panel__metric-value--good={keywordDensity >= 1 && keywordDensity <= 3} class:seo-panel__metric-value--warn={keywordDensity > 3}>
						{keywordDensity}% {keywordDensity >= 1 && keywordDensity <= 3 ? '(good)' : keywordDensity > 3 ? '(too high)' : '(add keyword)'}
					</span>
				</div>

				<div class="seo-panel__metric">
					<span class="seo-panel__metric-label">Readability (Flesch)</span>
					<span class="seo-panel__metric-value" style="color: {readabilityColor}">
						{readabilityScore}/100 - {readabilityLabel}
					</span>
				</div>
			</div>
		</div>
	{/if}

	<!-- Revision Diff View -->
	{#if showRevisionDiff && revisions.length > 0 && !isDistractionFree}
		<div class="blog-editor__revision-panel">
			<h4 class="seo-panel__title">Revision Comparison</h4>
			<div class="revision-panel__select">
				<label class="seo-panel__label" for="revision-select">Compare with revision:</label>
				<select id="revision-select" name="revision-select" class="seo-panel__input" bind:value={selectedRevisionId}>
					<option value="">Select a revision...</option>
					{#each revisions as rev (rev.id)}
						<option value={rev.id}>
							#{rev.revision_number} - {rev.author_name} ({new Date(rev.created_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })})
						</option>
					{/each}
				</select>
			</div>
			{#if selectedRevisionId && diffHtml}
				<div class="revision-panel__diff">
					{@html diffHtml}
				</div>
				<div class="revision-panel__legend">
					<span class="revision-panel__legend-item"><span class="diff-del-sample"></span> Removed</span>
					<span class="revision-panel__legend-item"><span class="diff-ins-sample"></span> Added</span>
				</div>
			{:else if selectedRevisionId}
				<p class="revision-panel__empty">No differences or revision content unavailable.</p>
			{/if}
		</div>
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

	.blog-editor--distraction-free {
		position: fixed;
		inset: 0;
		z-index: 9999;
		border-radius: 0;
		background-color: #0a0e1a;
	}

	.blog-editor--distraction-free .blog-editor__wrapper {
		max-width: 750px;
		margin: 0 auto;
		min-height: 0;
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

	/* Distraction-free bar */
	.blog-editor__df-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.5rem 1rem;
		background: rgba(255, 255, 255, 0.02);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}

	.blog-editor__df-wc {
		font-size: 0.75rem;
		color: var(--color-grey-500, #475569);
	}

	.blog-editor__df-exit {
		font-size: 0.75rem;
		color: var(--color-grey-400, #64748b);
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.25rem;
		padding: 0.25rem 0.75rem;
		cursor: pointer;
		transition: all 0.15s;
	}

	.blog-editor__df-exit:hover {
		color: var(--color-white, #fff);
		background: rgba(255, 255, 255, 0.1);
	}

	/* Autosave indicator bar */
	.blog-editor__autosave-bar {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.35rem 0.75rem;
		font-size: 0.7rem;
		color: var(--color-grey-400, #64748b);
		border-top: 1px solid rgba(255, 255, 255, 0.04);
		background: rgba(255, 255, 255, 0.02);
	}

	.blog-editor__autosave-bar--saved {
		color: #22c55e;
	}

	.blog-editor__autosave-bar--pending {
		color: #f59e0b;
	}

	.blog-editor__autosave-bar--error {
		color: #ef4444;
	}

	.blog-editor__autosave-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--color-grey-500, #475569);
		flex-shrink: 0;
	}

	.blog-editor__autosave-dot--green {
		background: #22c55e;
		box-shadow: 0 0 4px rgba(34, 197, 94, 0.5);
	}

	.blog-editor__autosave-dot--amber {
		background: #f59e0b;
		box-shadow: 0 0 4px rgba(245, 158, 11, 0.5);
	}

	.blog-editor__autosave-dot--red {
		background: #ef4444;
		box-shadow: 0 0 4px rgba(239, 68, 68, 0.5);
	}

	/* SEO Toggle / Panel toggle buttons */
	.blog-editor__seo-toggle {
		display: flex;
		gap: 0.5rem;
		padding: 0.4rem 0.75rem;
		border-top: 1px solid rgba(255, 255, 255, 0.04);
		background: rgba(255, 255, 255, 0.02);
	}

	.blog-editor__panel-btn {
		font-size: 0.7rem;
		color: var(--color-teal-light, #15c5d1);
		background: rgba(15, 164, 175, 0.08);
		border: 1px solid rgba(15, 164, 175, 0.2);
		border-radius: 0.25rem;
		padding: 0.25rem 0.6rem;
		cursor: pointer;
		transition: all 0.15s;
	}

	.blog-editor__panel-btn:hover {
		background: rgba(15, 164, 175, 0.15);
	}

	/* SEO Panel */
	.blog-editor__seo-panel {
		padding: 1rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		background: rgba(0, 0, 0, 0.15);
	}

	.seo-panel__title {
		font-size: 0.85rem;
		font-weight: 700;
		color: var(--color-white, #fff);
		margin: 0 0 0.75rem;
	}

	.seo-panel__field {
		margin-bottom: 0.75rem;
	}

	.seo-panel__label {
		display: block;
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--color-grey-400, #64748b);
		margin-bottom: 0.25rem;
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}

	.seo-panel__input {
		width: 100%;
		padding: 0.4rem 0.6rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.25rem;
		background: rgba(0, 0, 0, 0.3);
		color: #fff;
		font-size: 0.8rem;
		outline: none;
	}

	.seo-panel__input:focus {
		border-color: var(--color-teal, #0fa4af);
	}

	.seo-panel__metrics {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}

	.seo-panel__metric {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.seo-panel__metric-label {
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--color-grey-400, #64748b);
	}

	.seo-panel__bar-wrap {
		height: 4px;
		background: rgba(255, 255, 255, 0.08);
		border-radius: 2px;
		overflow: hidden;
	}

	.seo-panel__bar {
		height: 100%;
		background: var(--color-grey-500, #475569);
		border-radius: 2px;
		transition: width 0.3s, background 0.3s;
	}

	.seo-panel__bar--good {
		background: #22c55e;
	}

	.seo-panel__bar--warn {
		background: #f59e0b;
	}

	.seo-panel__metric-value {
		font-size: 0.7rem;
		color: var(--color-grey-400, #64748b);
	}

	.seo-panel__metric-value--good {
		color: #22c55e;
	}

	.seo-panel__metric-value--warn {
		color: #f59e0b;
	}

	/* Revision panel */
	.blog-editor__revision-panel {
		padding: 1rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		background: rgba(0, 0, 0, 0.15);
	}

	.revision-panel__select {
		margin-bottom: 0.75rem;
	}

	.revision-panel__diff {
		padding: 0.75rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: 0.375rem;
		font-size: 0.8rem;
		line-height: 1.6;
		color: var(--color-grey-300, #cbd5e1);
		max-height: 20rem;
		overflow-y: auto;
		white-space: pre-wrap;
		word-wrap: break-word;
	}

	.revision-panel__diff :global(.diff-del) {
		background: rgba(239, 68, 68, 0.2);
		color: #fca5a5;
		text-decoration: line-through;
		border-radius: 0.15rem;
		padding: 0 0.15rem;
	}

	.revision-panel__diff :global(.diff-ins) {
		background: rgba(34, 197, 94, 0.2);
		color: #86efac;
		border-radius: 0.15rem;
		padding: 0 0.15rem;
	}

	.revision-panel__legend {
		display: flex;
		gap: 1rem;
		margin-top: 0.5rem;
		font-size: 0.7rem;
		color: var(--color-grey-400, #64748b);
	}

	.revision-panel__legend-item {
		display: flex;
		align-items: center;
		gap: 0.3rem;
	}

	.revision-panel__legend-item :global(.diff-del-sample) {
		display: inline-block;
		width: 12px;
		height: 12px;
		background: rgba(239, 68, 68, 0.2);
		border: 1px solid rgba(239, 68, 68, 0.4);
		border-radius: 2px;
	}

	.revision-panel__legend-item :global(.diff-ins-sample) {
		display: inline-block;
		width: 12px;
		height: 12px;
		background: rgba(34, 197, 94, 0.2);
		border: 1px solid rgba(34, 197, 94, 0.4);
		border-radius: 2px;
	}

	.revision-panel__empty {
		font-size: 0.8rem;
		color: var(--color-grey-500, #475569);
		margin: 0;
	}

	/* TOC block in editor */
	.blog-editor :global(.blog-editor__content .toc-block) {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.5rem;
		padding: 1rem;
		margin: 1rem 0;
	}

	.blog-editor :global(.blog-editor__content .toc-block h4) {
		font-size: 0.9rem;
		font-weight: 700;
		color: var(--color-white, #fff);
		margin: 0 0 0.5rem;
	}

	.blog-editor :global(.blog-editor__content .toc-block ul) {
		list-style: none;
		padding-left: 0;
		margin: 0;
	}

	.blog-editor :global(.blog-editor__content .toc-block li) {
		padding: 0.15rem 0;
	}

	.blog-editor :global(.blog-editor__content .toc-block a) {
		font-size: 0.85rem;
	}

	/* Shortcode rendered blocks in editor */
	.blog-editor :global(.blog-editor__content .shortcode-alert) {
		padding: 0.75rem 1rem;
		border-radius: 0.375rem;
		margin: 0.75rem 0;
		border-left: 3px solid;
	}

	.blog-editor :global(.blog-editor__content .shortcode-alert[data-type="info"]) {
		background: rgba(59, 130, 246, 0.1);
		border-color: #3b82f6;
		color: #93c5fd;
	}

	.blog-editor :global(.blog-editor__content .shortcode-alert[data-type="warning"]) {
		background: rgba(245, 158, 11, 0.1);
		border-color: #f59e0b;
		color: #fcd34d;
	}

	.blog-editor :global(.blog-editor__content .shortcode-alert[data-type="success"]) {
		background: rgba(34, 197, 94, 0.1);
		border-color: #22c55e;
		color: #86efac;
	}

	.blog-editor :global(.blog-editor__content .shortcode-cta) {
		background: rgba(15, 164, 175, 0.08);
		border: 1px solid rgba(15, 164, 175, 0.2);
		border-radius: 0.5rem;
		padding: 1.25rem;
		text-align: center;
		margin: 1rem 0;
	}

	.blog-editor :global(.blog-editor__content .shortcode-pricing) {
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.5rem;
		padding: 1.25rem;
		text-align: center;
		margin: 1rem 0;
	}

	.blog-editor :global(.blog-editor__content .shortcode-pricing .price) {
		font-size: 1.75rem;
		font-weight: 700;
		color: var(--color-teal-light, #15c5d1);
	}

	.blog-editor :global(.blog-editor__content .cta-button) {
		display: inline-block;
		padding: 0.5rem 1.5rem;
		background: var(--color-teal, #0fa4af);
		color: #fff;
		border-radius: 0.375rem;
		text-decoration: none;
		font-weight: 600;
		margin-top: 0.5rem;
	}

	.blog-editor :global(.blog-editor__content .shortcode-accordion) {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.375rem;
		margin: 0.75rem 0;
	}

	.blog-editor :global(.blog-editor__content .shortcode-accordion summary) {
		padding: 0.6rem 0.75rem;
		cursor: pointer;
		font-weight: 600;
		color: var(--color-white, #fff);
	}

	.blog-editor :global(.blog-editor__content .shortcode-accordion p) {
		padding: 0 0.75rem 0.6rem;
	}
</style>
