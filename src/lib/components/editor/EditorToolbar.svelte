<script lang="ts">
	import type { Editor } from '@tiptap/core';
	import {
		TextB,
		TextItalic,
		TextUnderline,
		TextStrikethrough,
		TextSubscript,
		TextSuperscript,
		ListBullets,
		ListNumbers,
		CheckSquare,
		TextAlignLeft,
		TextAlignCenter,
		TextAlignRight,
		TextAlignJustify,
		ArrowCounterClockwise,
		ArrowClockwise,
		Link,
		Image,
		YoutubeLogo,
		PaintBrush,
		HighlighterCircle,
		Quotes,
		Code,
		Minus,
		Table,
		MathOperations,
		Eraser,
		TextOutdent,
		TextIndent,
		CodeBlock,
		CornersIn,
		CornersOut,
		X,
		ClipboardText,
		Scissors,
		Question,
		Eye,
		ListDashes,
		BracketsCurly
	} from 'phosphor-svelte';

	interface Props {
		editor: Editor;
		isFullscreen: boolean;
		showSource: boolean;
		isDistractionFree: boolean;
		wordCount?: number;
		charCount?: number;
		onToggleFullscreen: () => void;
		onToggleSource: () => void;
		onToggleDistractionFree: () => void;
		onInsertToc?: () => void;
		onInsertImage?: () => void;
		onInsertReadMore?: () => void;
	}

	let {
		editor,
		isFullscreen,
		showSource,
		isDistractionFree = false,
		wordCount = 0,
		charCount: _charCount = 0,
		onToggleFullscreen,
		onToggleSource,
		onToggleDistractionFree,
		onInsertToc,
		onInsertImage,
		onInsertReadMore
	}: Props = $props();

	let showLinkModal = $state(false);
	let linkUrl = $state('');
	let linkTarget = $state('_blank');
	let showColorPicker = $state(false);
	let showHighlightPicker = $state(false);
	let showHeadingMenu = $state(false);
	let showTableMenu = $state(false);
	let showSpecialChars = $state(false);
	let showYoutubeModal = $state(false);
	let youtubeUrl = $state('');

	const colors = [
		'#ffffff',
		'#94a3b8',
		'#ef4444',
		'#f97316',
		'#eab308',
		'#22c55e',
		'#0fa4af',
		'#3b82f6',
		'#8b5cf6',
		'#ec4899'
	];

	const specialChars = [
		'&amp;',
		'&copy;',
		'&reg;',
		'&trade;',
		'&euro;',
		'&pound;',
		'&yen;',
		'&deg;',
		'&plusmn;',
		'&times;',
		'&divide;',
		'&frac12;',
		'&frac14;',
		'&frac34;',
		'&laquo;',
		'&raquo;',
		'&bull;',
		'&mdash;',
		'&ndash;',
		'&hellip;',
		'&larr;',
		'&rarr;',
		'&uarr;',
		'&darr;'
	];

	const specialCharDisplay: Record<string, string> = {
		'&amp;': '&',
		'&copy;': '©',
		'&reg;': '®',
		'&trade;': '™',
		'&euro;': '€',
		'&pound;': '£',
		'&yen;': '¥',
		'&deg;': '°',
		'&plusmn;': '±',
		'&times;': '×',
		'&divide;': '÷',
		'&frac12;': '½',
		'&frac14;': '¼',
		'&frac34;': '¾',
		'&laquo;': '«',
		'&raquo;': '»',
		'&bull;': '•',
		'&mdash;': '—',
		'&ndash;': '–',
		'&hellip;': '…',
		'&larr;': '←',
		'&rarr;': '→',
		'&uarr;': '↑',
		'&darr;': '↓'
	};

	function setLink() {
		if (linkUrl) {
			editor
				.chain()
				.focus()
				.extendMarkRange('link')
				.setLink({
					href: linkUrl,
					target: linkTarget
				})
				.run();
		}
		showLinkModal = false;
		linkUrl = '';
	}

	function removeLink() {
		editor.chain().focus().unsetLink().run();
		showLinkModal = false;
	}

	function openLinkModal() {
		const attrs = editor.getAttributes('link');
		linkUrl = attrs.href || '';
		linkTarget = attrs.target || '_blank';
		showLinkModal = true;
	}

	function setHeading(level: 1 | 2 | 3 | 4 | 5 | 6) {
		editor.chain().focus().toggleHeading({ level }).run();
		showHeadingMenu = false;
	}

	function setParagraph() {
		editor.chain().focus().setParagraph().run();
		showHeadingMenu = false;
	}

	function insertTable() {
		editor.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run();
		showTableMenu = false;
	}

	function insertSpecialChar(html: string) {
		editor.chain().focus().insertContent(html).run();
		showSpecialChars = false;
	}

	function insertYoutube() {
		if (youtubeUrl) {
			editor.commands.setYoutubeVideo({ src: youtubeUrl });
		}
		showYoutubeModal = false;
		youtubeUrl = '';
	}

	function clearFormatting() {
		editor.chain().focus().unsetAllMarks().clearNodes().run();
	}

	function getCurrentHeadingLabel(): string {
		for (let i = 1; i <= 6; i++) {
			if (editor.isActive('heading', { level: i })) return `H${i}`;
		}
		return 'P';
	}

	function closeAllDropdowns() {
		showHeadingMenu = false;
		showColorPicker = false;
		showHighlightPicker = false;
		showTableMenu = false;
		showSpecialChars = false;
		showShortcodeMenu = false;
	}

	let showShortcodeMenu = $state(false);

	const readingTime = $derived(Math.max(1, Math.ceil(wordCount / 238)));

	const shortcodes = [
		{
			label: 'Alert Box',
			code: '<div class="shortcode-alert" data-type="info">\n  <strong>Note:</strong> Your message here.\n</div>'
		},
		{
			label: 'Call to Action',
			code: '<div class="shortcode-cta">\n  <h3>Ready to get started?</h3>\n  <p>Sign up today and start your journey.</p>\n  <a href="#" class="cta-button">Get Started</a>\n</div>'
		},
		{
			label: 'Pricing Card',
			code: '<div class="shortcode-pricing">\n  <h4>Pro Plan</h4>\n  <p class="price">$29/mo</p>\n  <ul>\n    <li>Feature one</li>\n    <li>Feature two</li>\n    <li>Feature three</li>\n  </ul>\n  <a href="#" class="cta-button">Choose Plan</a>\n</div>'
		},
		{
			label: 'Warning Box',
			code: '<div class="shortcode-alert" data-type="warning">\n  <strong>Warning:</strong> Please be aware of this.\n</div>'
		},
		{
			label: 'Success Box',
			code: '<div class="shortcode-alert" data-type="success">\n  <strong>Success!</strong> Operation completed.\n</div>'
		},
		{
			label: 'Accordion',
			code: '<details class="shortcode-accordion">\n  <summary>Click to expand</summary>\n  <p>Accordion content goes here.</p>\n</details>'
		}
	];

	function insertShortcode(html: string) {
		editor.chain().focus().insertContent(html).run();
		showShortcodeMenu = false;
	}

	let showShortcuts = $state(false);

	const shortcuts = [
		{ keys: 'Ctrl+B', label: 'Bold' },
		{ keys: 'Ctrl+I', label: 'Italic' },
		{ keys: 'Ctrl+U', label: 'Underline' },
		{ keys: 'Ctrl+K', label: 'Insert / Edit link' },
		{ keys: 'Ctrl+Z', label: 'Undo' },
		{ keys: 'Ctrl+Shift+Z', label: 'Redo' },
		{ keys: 'Ctrl+Alt+1–6', label: 'Heading 1–6' },
		{ keys: 'Ctrl+Shift+8', label: 'Bullet list' },
		{ keys: 'Ctrl+Shift+9', label: 'Ordered list' },
		{ keys: 'Ctrl+Shift+B', label: 'Blockquote' },
		{ keys: 'Ctrl+`', label: 'Inline code' },
		{ keys: 'Ctrl+Alt+C', label: 'Code block' },
		{ keys: 'Tab', label: 'Indent list item' },
		{ keys: 'Shift+Tab', label: 'Outdent list item' },
		{ keys: 'Enter (in list)', label: 'New list item' },
		{ keys: 'Backspace (empty item)', label: 'Exit list' }
	];

	async function pasteAsPlainText() {
		try {
			const text = await navigator.clipboard.readText();
			editor.chain().focus().insertContent({ type: 'text', text }).run();
		} catch (e) {
			console.warn('Clipboard access denied', e);
		}
	}
</script>

<div class="toolbar" role="toolbar" aria-label="Editor toolbar">
	<!-- Row 1: Primary formatting -->
	<div class="toolbar__row">
		<!-- Heading dropdown -->
		<div class="toolbar__group">
			<div class="toolbar__dropdown-wrap">
				<button
					class="toolbar__btn toolbar__btn--dropdown"
					title="Block type"
					onclick={() => {
						closeAllDropdowns();
						showHeadingMenu = !showHeadingMenu;
					}}
				>
					{getCurrentHeadingLabel()} ▾
				</button>
				{#if showHeadingMenu}
					<div class="toolbar__dropdown toolbar__dropdown--headings">
						<button
							class="toolbar__dropdown-item toolbar__heading-preview toolbar__heading-preview--p"
							onclick={setParagraph}>Paragraph</button
						>
						<button
							class="toolbar__dropdown-item toolbar__heading-preview toolbar__heading-preview--h1"
							onclick={() => setHeading(1)}>Heading 1</button
						>
						<button
							class="toolbar__dropdown-item toolbar__heading-preview toolbar__heading-preview--h2"
							onclick={() => setHeading(2)}>Heading 2</button
						>
						<button
							class="toolbar__dropdown-item toolbar__heading-preview toolbar__heading-preview--h3"
							onclick={() => setHeading(3)}>Heading 3</button
						>
						<button
							class="toolbar__dropdown-item toolbar__heading-preview toolbar__heading-preview--h4"
							onclick={() => setHeading(4)}>Heading 4</button
						>
						<button
							class="toolbar__dropdown-item toolbar__heading-preview toolbar__heading-preview--h5"
							onclick={() => setHeading(5)}>Heading 5</button
						>
						<button
							class="toolbar__dropdown-item toolbar__heading-preview toolbar__heading-preview--h6"
							onclick={() => setHeading(6)}>Heading 6</button
						>
					</div>
				{/if}
			</div>
		</div>

		<div class="toolbar__sep"></div>

		<!-- Text formatting -->
		<div class="toolbar__group">
			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive('bold')}
				title="Bold (Ctrl+B)"
				onclick={() => editor.chain().focus().toggleBold().run()}
			>
				<TextB size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive('italic')}
				title="Italic (Ctrl+I)"
				onclick={() => editor.chain().focus().toggleItalic().run()}
			>
				<TextItalic size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive('underline')}
				title="Underline (Ctrl+U)"
				onclick={() => editor.chain().focus().toggleUnderline().run()}
			>
				<TextUnderline size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive('strike')}
				title="Strikethrough"
				onclick={() => editor.chain().focus().toggleStrike().run()}
			>
				<TextStrikethrough size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive('subscript')}
				title="Subscript"
				onclick={() => editor.chain().focus().toggleSubscript().run()}
			>
				<TextSubscript size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive('superscript')}
				title="Superscript"
				onclick={() => editor.chain().focus().toggleSuperscript().run()}
			>
				<TextSuperscript size={18} weight="bold" />
			</button>
		</div>

		<div class="toolbar__sep"></div>

		<!-- Lists -->
		<div class="toolbar__group">
			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive('bulletList')}
				title="Bullet list"
				onclick={() => editor.chain().focus().toggleBulletList().run()}
			>
				<ListBullets size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive('orderedList')}
				title="Numbered list"
				onclick={() => editor.chain().focus().toggleOrderedList().run()}
			>
				<ListNumbers size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive('taskList')}
				title="Task list"
				onclick={() => editor.chain().focus().toggleTaskList().run()}
			>
				<CheckSquare size={18} weight="bold" />
			</button>
		</div>

		<div class="toolbar__sep"></div>

		<!-- Alignment -->
		<div class="toolbar__group">
			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive({ textAlign: 'left' })}
				title="Align left"
				onclick={() => editor.chain().focus().setTextAlign('left').run()}
			>
				<TextAlignLeft size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive({ textAlign: 'center' })}
				title="Align center"
				onclick={() => editor.chain().focus().setTextAlign('center').run()}
			>
				<TextAlignCenter size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive({ textAlign: 'right' })}
				title="Align right"
				onclick={() => editor.chain().focus().setTextAlign('right').run()}
			>
				<TextAlignRight size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive({ textAlign: 'justify' })}
				title="Justify"
				onclick={() => editor.chain().focus().setTextAlign('justify').run()}
			>
				<TextAlignJustify size={18} weight="bold" />
			</button>
		</div>

		<div class="toolbar__sep"></div>

		<!-- Undo / Redo -->
		<div class="toolbar__group">
			<button
				class="toolbar__btn"
				title="Undo (Ctrl+Z)"
				disabled={!editor.can().undo()}
				onclick={() => editor.chain().focus().undo().run()}
			>
				<ArrowCounterClockwise size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				title="Redo (Ctrl+Shift+Z)"
				disabled={!editor.can().redo()}
				onclick={() => editor.chain().focus().redo().run()}
			>
				<ArrowClockwise size={18} weight="bold" />
			</button>
		</div>
	</div>

	<!-- Row 2: Advanced formatting -->
	<div class="toolbar__row">
		<!-- Link -->
		<div class="toolbar__group">
			<div class="toolbar__dropdown-wrap">
				<button
					class="toolbar__btn"
					class:toolbar__btn--active={editor.isActive('link')}
					title="Insert/Edit link (Ctrl+K)"
					onclick={openLinkModal}
				>
					<Link size={18} weight="bold" />
				</button>
				{#if showLinkModal}
					<div class="toolbar__dropdown toolbar__dropdown--wide">
						<label class="toolbar__dropdown-label">
							URL
							<input
								id="toolbar-link-url"
								name="link-url"
								type="url"
								class="toolbar__dropdown-input"
								bind:value={linkUrl}
								placeholder="https://..."
							/>
						</label>
						<label class="toolbar__dropdown-label">
							Target
							<select
								id="toolbar-link-target"
								name="link-target"
								class="toolbar__dropdown-input"
								bind:value={linkTarget}
							>
								<option value="_blank">New tab</option>
								<option value="_self">Same tab</option>
							</select>
						</label>
						<div class="toolbar__dropdown-actions">
							<button class="toolbar__dropdown-btn" onclick={setLink}>Apply</button>
							{#if editor.isActive('link')}
								<button
									class="toolbar__dropdown-btn toolbar__dropdown-btn--danger"
									onclick={removeLink}>Remove</button
								>
							{/if}
							<button
								class="toolbar__dropdown-btn"
								onclick={() => (showLinkModal = false)}>Cancel</button
							>
						</div>
					</div>
				{/if}
			</div>
		</div>

		<div class="toolbar__sep"></div>

		<!-- Media -->
		<div class="toolbar__group">
			{#if onInsertImage}
				<button class="toolbar__btn" title="Add media" onclick={onInsertImage}>
					<Image size={18} weight="bold" />
				</button>
			{/if}

			<div class="toolbar__dropdown-wrap">
				<button
					class="toolbar__btn"
					title="Embed YouTube"
					onclick={() => {
						closeAllDropdowns();
						showYoutubeModal = !showYoutubeModal;
					}}
				>
					<YoutubeLogo size={18} weight="bold" />
				</button>
				{#if showYoutubeModal}
					<div class="toolbar__dropdown toolbar__dropdown--wide">
						<label class="toolbar__dropdown-label">
							YouTube URL
							<input
								id="toolbar-youtube-url"
								name="youtube-url"
								type="url"
								class="toolbar__dropdown-input"
								bind:value={youtubeUrl}
								placeholder="https://youtube.com/watch?v=..."
							/>
						</label>
						<div class="toolbar__dropdown-actions">
							<button class="toolbar__dropdown-btn" onclick={insertYoutube}
								>Embed</button
							>
							<button
								class="toolbar__dropdown-btn"
								onclick={() => (showYoutubeModal = false)}>Cancel</button
							>
						</div>
					</div>
				{/if}
			</div>
		</div>

		<div class="toolbar__sep"></div>

		<!-- Colors -->
		<div class="toolbar__group">
			<div class="toolbar__dropdown-wrap">
				<button
					class="toolbar__btn"
					title="Text color"
					onclick={() => {
						closeAllDropdowns();
						showColorPicker = !showColorPicker;
					}}
				>
					<PaintBrush size={18} weight="bold" />
				</button>
				{#if showColorPicker}
					<div class="toolbar__dropdown toolbar__dropdown--colors">
						{#each colors as c (c)}
							<button
								class="toolbar__color-swatch"
								style="background: {c}"
								title={c}
								onclick={() => {
									editor.chain().focus().setColor(c).run();
									showColorPicker = false;
								}}
							></button>
						{/each}
						<button
							class="toolbar__color-swatch toolbar__color-swatch--clear"
							title="Remove color"
							onclick={() => {
								editor.chain().focus().unsetColor().run();
								showColorPicker = false;
							}}
						>
							<X size={14} weight="bold" />
						</button>
					</div>
				{/if}
			</div>

			<div class="toolbar__dropdown-wrap">
				<button
					class="toolbar__btn"
					class:toolbar__btn--active={editor.isActive('highlight')}
					title="Highlight"
					onclick={() => {
						closeAllDropdowns();
						showHighlightPicker = !showHighlightPicker;
					}}
				>
					<HighlighterCircle size={18} weight="bold" />
				</button>
				{#if showHighlightPicker}
					<div class="toolbar__dropdown toolbar__dropdown--colors">
						{#each colors as c (c)}
							<button
								class="toolbar__color-swatch"
								style="background: {c}"
								title={c}
								onclick={() => {
									editor.chain().focus().toggleHighlight({ color: c }).run();
									showHighlightPicker = false;
								}}
							></button>
						{/each}
						<button
							class="toolbar__color-swatch toolbar__color-swatch--clear"
							title="Remove highlight"
							onclick={() => {
								editor.chain().focus().unsetHighlight().run();
								showHighlightPicker = false;
							}}
						>
							<X size={14} weight="bold" />
						</button>
					</div>
				{/if}
			</div>
		</div>

		<div class="toolbar__sep"></div>

		<!-- Block types -->
		<div class="toolbar__group">
			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive('blockquote')}
				title="Blockquote"
				onclick={() => editor.chain().focus().toggleBlockquote().run()}
			>
				<Quotes size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={editor.isActive('codeBlock')}
				title="Code block"
				onclick={() => editor.chain().focus().toggleCodeBlock().run()}
			>
				<Code size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				title="Horizontal rule"
				onclick={() => editor.chain().focus().setHorizontalRule().run()}
			>
				<Minus size={18} weight="bold" />
			</button>
		</div>

		<div class="toolbar__sep"></div>

		<!-- Table -->
		<div class="toolbar__group">
			<div class="toolbar__dropdown-wrap">
				<button
					class="toolbar__btn"
					title="Table"
					onclick={() => {
						closeAllDropdowns();
						showTableMenu = !showTableMenu;
					}}
				>
					<Table size={18} weight="bold" />
				</button>
				{#if showTableMenu}
					<div class="toolbar__dropdown">
						<button class="toolbar__dropdown-item" onclick={insertTable}
							>Insert 3×3 table</button
						>
						{#if editor.isActive('table')}
							<button
								class="toolbar__dropdown-item"
								onclick={() => {
									editor.chain().focus().addColumnAfter().run();
									showTableMenu = false;
								}}>Add column after</button
							>
							<button
								class="toolbar__dropdown-item"
								onclick={() => {
									editor.chain().focus().addRowAfter().run();
									showTableMenu = false;
								}}>Add row after</button
							>
							<button
								class="toolbar__dropdown-item"
								onclick={() => {
									editor.chain().focus().deleteColumn().run();
									showTableMenu = false;
								}}>Delete column</button
							>
							<button
								class="toolbar__dropdown-item"
								onclick={() => {
									editor.chain().focus().deleteRow().run();
									showTableMenu = false;
								}}>Delete row</button
							>
							<button
								class="toolbar__dropdown-item"
								onclick={() => {
									editor.chain().focus().deleteTable().run();
									showTableMenu = false;
								}}>Delete table</button
							>
						{/if}
					</div>
				{/if}
			</div>
		</div>

		<div class="toolbar__sep"></div>

		<!-- Special characters -->
		<div class="toolbar__group">
			<div class="toolbar__dropdown-wrap">
				<button
					class="toolbar__btn"
					title="Special characters"
					onclick={() => {
						closeAllDropdowns();
						showSpecialChars = !showSpecialChars;
					}}
				>
					<MathOperations size={18} weight="bold" />
				</button>
				{#if showSpecialChars}
					<div class="toolbar__dropdown toolbar__dropdown--chars">
						{#each specialChars as ch (ch)}
							<button
								class="toolbar__char-btn"
								title={ch}
								onclick={() => insertSpecialChar(ch)}
							>
								{specialCharDisplay[ch] || ch}
							</button>
						{/each}
					</div>
				{/if}
			</div>
		</div>

		<div class="toolbar__sep"></div>

		<!-- Paste / Read More -->
		<div class="toolbar__group">
			<button class="toolbar__btn" title="Paste as plain text" onclick={pasteAsPlainText}>
				<ClipboardText size={18} weight="bold" />
			</button>
			{#if onInsertReadMore}
				<button
					class="toolbar__btn"
					title="Insert Read More break"
					onclick={onInsertReadMore}
				>
					<Scissors size={18} weight="bold" />
				</button>
			{/if}
		</div>

		<div class="toolbar__sep"></div>

		<!-- Clear formatting -->
		<div class="toolbar__group">
			<button class="toolbar__btn" title="Clear formatting" onclick={clearFormatting}>
				<Eraser size={18} weight="bold" />
			</button>

			<!-- Indent / Outdent for lists -->
			<button
				class="toolbar__btn"
				title="Decrease indent"
				onclick={() => editor.chain().focus().liftListItem('listItem').run()}
				disabled={!editor.can().liftListItem('listItem')}
			>
				<TextOutdent size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				title="Increase indent"
				onclick={() => editor.chain().focus().sinkListItem('listItem').run()}
				disabled={!editor.can().sinkListItem('listItem')}
			>
				<TextIndent size={18} weight="bold" />
			</button>
		</div>

		<div class="toolbar__sep"></div>

		<!-- TOC + Shortcodes -->
		<div class="toolbar__group">
			{#if onInsertToc}
				<button class="toolbar__btn" title="Insert Table of Contents" onclick={onInsertToc}>
					<ListDashes size={18} weight="bold" />
				</button>
			{/if}

			<div class="toolbar__dropdown-wrap">
				<button
					class="toolbar__btn"
					title="Insert Shortcode"
					onclick={() => {
						closeAllDropdowns();
						showShortcodeMenu = !showShortcodeMenu;
					}}
				>
					<BracketsCurly size={18} weight="bold" />
				</button>
				{#if showShortcodeMenu}
					<div class="toolbar__dropdown">
						{#each shortcodes as sc (sc.label)}
							<button
								class="toolbar__dropdown-item"
								onclick={() => insertShortcode(sc.code)}
							>
								{sc.label}
							</button>
						{/each}
					</div>
				{/if}
			</div>
		</div>

		<!-- Right-side: view controls -->
		<div class="toolbar__spacer"></div>

		<!-- Word count / reading time -->
		<div class="toolbar__group">
			<span class="toolbar__stats">
				{wordCount} words &middot; {readingTime} min read
			</span>
		</div>

		<div class="toolbar__sep"></div>

		<div class="toolbar__group">
			<button
				class="toolbar__btn"
				class:toolbar__btn--active={isDistractionFree}
				title="Distraction-free mode"
				onclick={onToggleDistractionFree}
			>
				<Eye size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={showSource}
				title="Toggle HTML source"
				onclick={onToggleSource}
			>
				<CodeBlock size={18} weight="bold" />
			</button>

			<button
				class="toolbar__btn"
				class:toolbar__btn--active={isFullscreen}
				title="Toggle fullscreen"
				onclick={onToggleFullscreen}
			>
				{#if isFullscreen}
					<CornersIn size={18} weight="bold" />
				{:else}
					<CornersOut size={18} weight="bold" />
				{/if}
			</button>

			<button
				class="toolbar__btn"
				title="Keyboard shortcuts"
				onclick={() => (showShortcuts = !showShortcuts)}
			>
				<Question size={18} weight="bold" />
			</button>
		</div>
	</div>
</div>

{#if showShortcuts}
	<div
		class="shortcuts-overlay"
		role="dialog"
		aria-modal="true"
		aria-label="Keyboard shortcuts"
		onclick={() => (showShortcuts = false)}
		onkeydown={(e) => e.key === 'Escape' && (showShortcuts = false)}
		tabindex="-1"
	>
		<div
			class="shortcuts-modal"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="presentation"
		>
			<div class="shortcuts-modal__header">
				<h3 class="shortcuts-modal__title">Keyboard Shortcuts</h3>
				<button class="shortcuts-modal__close" onclick={() => (showShortcuts = false)}>
					<X size={16} weight="bold" />
				</button>
			</div>
			<ul class="shortcuts-modal__list">
				{#each shortcuts as s (s.keys)}
					<li class="shortcuts-modal__item">
						<kbd class="shortcuts-modal__kbd">{s.keys}</kbd>
						<span class="shortcuts-modal__label">{s.label}</span>
					</li>
				{/each}
			</ul>
		</div>
	</div>
{/if}

<style>
	.toolbar {
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		background: rgba(255, 255, 255, 0.04);
		user-select: none;
	}

	.toolbar__row {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0;
		padding: 0.25rem 0.5rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}

	.toolbar__row:last-child {
		border-bottom: none;
	}

	.toolbar__group {
		display: flex;
		align-items: center;
		gap: 1px;
	}

	.toolbar__sep {
		width: 1px;
		height: 1.5rem;
		background: rgba(255, 255, 255, 0.08);
		margin: 0 0.35rem;
	}

	.toolbar__spacer {
		flex: 1;
	}

	.toolbar__btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 2rem;
		height: 2rem;
		padding: 0 0.4rem;
		border: none;
		border-radius: 0.25rem;
		background: transparent;
		color: var(--color-grey-300, #94a3b8);
		font-size: 0.8rem;
		cursor: pointer;
		transition:
			background 0.15s,
			color 0.15s;
		white-space: nowrap;
	}

	.toolbar__btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-white, #fff);
	}

	.toolbar__btn--active {
		background: rgba(15, 164, 175, 0.2);
		color: var(--color-teal-light, #15c5d1);
	}

	.toolbar__btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.toolbar__btn--dropdown {
		font-weight: 600;
		min-width: 3rem;
	}

	/* Dropdown */
	.toolbar__dropdown-wrap {
		position: relative;
	}

	.toolbar__dropdown {
		position: absolute;
		top: 100%;
		left: 0;
		z-index: 100;
		background: var(--color-navy-deep, #0d1321);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.5rem;
		padding: 0.25rem;
		min-width: 10rem;
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
		margin-top: 0.25rem;
	}

	.toolbar__dropdown--wide {
		min-width: 16rem;
		padding: 0.75rem;
	}

	.toolbar__dropdown--colors {
		display: grid;
		grid-template-columns: repeat(5, 1fr);
		gap: 0.25rem;
		min-width: auto;
		padding: 0.5rem;
	}

	.toolbar__dropdown--chars {
		display: grid;
		grid-template-columns: repeat(6, 1fr);
		gap: 0.25rem;
		min-width: auto;
		padding: 0.5rem;
	}

	.toolbar__dropdown-item {
		display: block;
		width: 100%;
		padding: 0.4rem 0.75rem;
		border: none;
		border-radius: 0.25rem;
		background: transparent;
		color: var(--color-grey-200, #e2e8f0);
		font-size: 0.8rem;
		text-align: left;
		cursor: pointer;
	}

	.toolbar__dropdown-item:hover {
		background: rgba(255, 255, 255, 0.08);
	}

	.toolbar__dropdown-label {
		display: block;
		font-size: 0.75rem;
		color: var(--color-grey-400, #64748b);
		margin-bottom: 0.5rem;
	}

	.toolbar__dropdown-input {
		display: block;
		width: 100%;
		padding: 0.4rem 0.5rem;
		margin-top: 0.25rem;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.25rem;
		background: rgba(0, 0, 0, 0.3);
		color: #fff;
		font-size: 0.8rem;
		outline: none;
	}

	.toolbar__dropdown-input:focus {
		border-color: var(--color-teal, #0fa4af);
	}

	.toolbar__dropdown-actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}

	.toolbar__dropdown-btn {
		padding: 0.35rem 0.75rem;
		border: none;
		border-radius: 0.25rem;
		background: var(--color-teal, #0fa4af);
		color: #fff;
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
	}

	.toolbar__dropdown-btn:hover {
		opacity: 0.9;
	}

	.toolbar__dropdown-btn--danger {
		background: #ef4444;
	}

	/* Color swatches */
	.toolbar__color-swatch {
		width: 1.75rem;
		height: 1.75rem;
		border: 2px solid rgba(255, 255, 255, 0.1);
		border-radius: 0.25rem;
		cursor: pointer;
		transition: transform 0.1s;
	}

	.toolbar__color-swatch:hover {
		transform: scale(1.15);
		border-color: rgba(255, 255, 255, 0.4);
	}

	.toolbar__color-swatch--clear {
		background: transparent;
		color: var(--color-grey-400, #64748b);
		font-size: 0.7rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	/* Color icon underline indicator — applied dynamically by JS */
	:global(.toolbar__color-icon) {
		border-bottom: 2px solid;
		padding-bottom: 1px;
		font-weight: 700;
	}

	:global(.toolbar__highlight-icon) {
		background: rgba(255, 255, 0, 0.25);
		padding: 0 0.2rem;
		border-radius: 0.15rem;
		font-weight: 700;
	}

	/* Special char button */
	.toolbar__char-btn {
		width: 2rem;
		height: 2rem;
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 0.25rem;
		background: transparent;
		color: var(--color-grey-200, #e2e8f0);
		font-size: 0.9rem;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.toolbar__char-btn:hover {
		background: rgba(255, 255, 255, 0.08);
	}

	/* Keyboard shortcuts modal */
	.shortcuts-overlay {
		position: fixed;
		inset: 0;
		z-index: 10000;
		background: rgba(0, 0, 0, 0.55);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}

	.shortcuts-modal {
		background: var(--color-navy-deep, #0a0e1a);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 0.75rem;
		padding: 1.5rem;
		width: 100%;
		max-width: 420px;
		max-height: 80vh;
		overflow-y: auto;
	}

	.shortcuts-modal__header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1.25rem;
	}

	.shortcuts-modal__title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white, #fff);
		margin: 0;
	}

	.shortcuts-modal__close {
		background: transparent;
		border: none;
		color: var(--color-grey-400, #94a3b8);
		cursor: pointer;
		padding: 0.25rem;
		display: flex;
		border-radius: 0.25rem;
	}

	.shortcuts-modal__close:hover {
		color: var(--color-white, #fff);
	}

	.shortcuts-modal__list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.shortcuts-modal__item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.shortcuts-modal__kbd {
		display: inline-block;
		min-width: 9rem;
		padding: 0.2rem 0.5rem;
		background: rgba(255, 255, 255, 0.07);
		border: 1px solid rgba(255, 255, 255, 0.14);
		border-radius: 0.3rem;
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		color: var(--color-teal-light, #15c5d1);
		white-space: nowrap;
	}

	.shortcuts-modal__label {
		font-size: 0.85rem;
		color: var(--color-grey-300, #cbd5e1);
	}

	/* Word count / reading time stats */
	.toolbar__stats {
		font-size: 0.7rem;
		color: var(--color-grey-400, #64748b);
		white-space: nowrap;
		padding: 0 0.35rem;
		user-select: none;
	}

	/* Heading preview styles in dropdown */
	.toolbar__dropdown--headings {
		min-width: 14rem;
	}

	.toolbar__heading-preview {
		font-weight: 400 !important;
		line-height: 1.3;
	}

	.toolbar__heading-preview--p {
		font-size: 0.85rem;
		color: var(--color-grey-300, #cbd5e1);
	}

	.toolbar__heading-preview--h1 {
		font-size: 1.4rem;
		font-weight: 700 !important;
		color: var(--color-white, #fff);
	}

	.toolbar__heading-preview--h2 {
		font-size: 1.15rem;
		font-weight: 700 !important;
		color: var(--color-white, #fff);
	}

	.toolbar__heading-preview--h3 {
		font-size: 1rem;
		font-weight: 600 !important;
		color: var(--color-white, #fff);
	}

	.toolbar__heading-preview--h4 {
		font-size: 0.9rem;
		font-weight: 600 !important;
		color: var(--color-grey-200, #e2e8f0);
	}

	.toolbar__heading-preview--h5 {
		font-size: 0.85rem;
		font-weight: 600 !important;
		color: var(--color-grey-300, #cbd5e1);
	}

	.toolbar__heading-preview--h6 {
		font-size: 0.8rem;
		font-weight: 600 !important;
		color: var(--color-grey-400, #94a3b8);
	}
</style>
