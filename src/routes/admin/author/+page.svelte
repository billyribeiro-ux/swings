<script lang="ts">
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { UserResponse } from '$lib/api/types';
	import UserCircleIcon from 'phosphor-svelte/lib/UserCircleIcon';
	import ImageIcon from 'phosphor-svelte/lib/ImageIcon';
	import TwitterLogoIcon from 'phosphor-svelte/lib/TwitterLogoIcon';
	import LinkedinLogoIcon from 'phosphor-svelte/lib/LinkedinLogoIcon';
	import YoutubeLogoIcon from 'phosphor-svelte/lib/YoutubeLogoIcon';
	import InstagramLogoIcon from 'phosphor-svelte/lib/InstagramLogoIcon';
	import GlobeIcon from 'phosphor-svelte/lib/GlobeIcon';
	import FloppyDiskIcon from 'phosphor-svelte/lib/FloppyDiskIcon';

	let profile: UserResponse | null = $state(null);
	let loading = $state(true);
	let saving = $state(false);
	let saved = $state(false);
	let error = $state('');
	let uploadingAvatar = $state(false);

	let name = $state('');
	let position = $state('');
	let bio = $state('');
	let avatarUrl = $state('');
	let websiteUrl = $state('');
	let twitterUrl = $state('');
	let linkedinUrl = $state('');
	let youtubeUrl = $state('');
	let instagramUrl = $state('');

	onMount(async () => {
		try {
			profile = await api.get<UserResponse>('/api/member/profile');
			name = profile.name;
			position = profile.position || '';
			bio = profile.bio || '';
			avatarUrl = profile.avatar_url || '';
			websiteUrl = profile.website_url || '';
			twitterUrl = profile.twitter_url || '';
			linkedinUrl = profile.linkedin_url || '';
			youtubeUrl = profile.youtube_url || '';
			instagramUrl = profile.instagram_url || '';
		} catch {
			error = 'Failed to load profile.';
		} finally {
			loading = false;
		}
	});

	async function handleSave(e: Event) {
		e.preventDefault();
		saving = true;
		saved = false;
		error = '';
		try {
			const updated = await api.put<UserResponse>('/api/member/profile', {
				name: name || undefined,
				position: position || undefined,
				bio: bio || undefined,
				avatar_url: avatarUrl || undefined,
				website_url: websiteUrl || undefined,
				twitter_url: twitterUrl || undefined,
				linkedin_url: linkedinUrl || undefined,
				youtube_url: youtubeUrl || undefined,
				instagram_url: instagramUrl || undefined
			});
			auth.setUser({ ...auth.user!, name: updated.name });
			saved = true;
			setTimeout(() => (saved = false), 3000);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to save profile.';
		} finally {
			saving = false;
		}
	}

	async function handleAvatarUpload(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		uploadingAvatar = true;
		error = '';
		try {
			const formData = new FormData();
			formData.append('file', file);
			formData.append('title', `${name || 'Author'} profile photo`);
			const media = await api.upload<{ url: string }>('/api/admin/blog/media/upload', formData);
			avatarUrl = media.url;
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Upload failed.';
		} finally {
			uploadingAvatar = false;
			input.value = '';
		}
	}
</script>

<svelte:head>
	<title>Author Profile — Admin</title>
</svelte:head>

<div class="author-profile">
	<div class="author-profile__header">
		<span class="author-profile__eyebrow">Profile</span>
		<h1 class="author-profile__title">Author Profile</h1>
		<p class="author-profile__subtitle">This information appears on your published blog posts.</p>
	</div>

	{#if loading}
		<div class="author-profile__loading">Loading profile...</div>
	{:else}
		<form onsubmit={handleSave} class="author-profile__form">
			<!-- Left column: avatar + meta -->
			<aside class="author-profile__aside">
				<section class="author-section author-section--aside">
					<h2 class="author-section__title">Profile Photo</h2>
					<div class="author-section__content">
						<div class="avatar-editor">
							<div class="avatar-editor__preview">
								{#if avatarUrl}
									<img src={avatarUrl} alt="Author profile" class="avatar-editor__img" />
								{:else}
									<div class="avatar-editor__placeholder">
										<UserCircleIcon size={64} weight="thin" />
									</div>
								{/if}
							</div>
							<div class="avatar-editor__controls">
								<label
									class="btn-upload-avatar"
									class:btn-upload-avatar--loading={uploadingAvatar}
								>
									<ImageIcon size={16} weight="bold" />
									{uploadingAvatar ? 'Uploading...' : 'Upload photo'}
									<input
										id="avatar-upload"
										name="avatar-upload"
										type="file"
										accept="image/*"
										hidden
										onchange={handleAvatarUpload}
										disabled={uploadingAvatar}
									/>
								</label>
								<div class="author-field">
									<label for="avatar-url" class="author-field__label">Or paste a URL</label>
									<input
										id="avatar-url"
										name="avatar-url"
										type="url"
										class="author-field__input"
										bind:value={avatarUrl}
										placeholder="https://..."
									/>
								</div>
							</div>
						</div>
					</div>
				</section>
			</aside>

			<!-- Right column: form sections -->
			<div class="author-profile__main">
				<!-- Personal Info -->
				<section class="author-section">
					<h2 class="author-section__title">Personal Info</h2>
					<div class="author-section__content author-section__content--grid">
						<div class="author-field">
							<label for="author-name" class="author-field__label">Display Name</label>
							<input
								id="author-name"
								name="author-name"
								type="text"
								class="author-field__input"
								bind:value={name}
								required
								placeholder="Billy Ribeiro"
							/>
						</div>
						<div class="author-field">
							<label for="author-position" class="author-field__label">Position / Title</label>
							<input
								id="author-position"
								name="author-position"
								type="text"
								class="author-field__input"
								bind:value={position}
								placeholder="Founder & Head Trader"
							/>
						</div>
						<div class="author-field author-field--full">
							<label for="author-bio" class="author-field__label">
								<span>Biographical Info</span>
								<span class="author-field__hint"
									>Appears in the author box at the bottom of each post.</span
								>
							</label>
							<textarea
								id="author-bio"
								name="author-bio"
								class="author-field__textarea"
								bind:value={bio}
								placeholder="Tell readers a bit about yourself..."
								rows="4"
							></textarea>
						</div>
					</div>
				</section>

				<!-- Contact & Web -->
				<section class="author-section">
					<h2 class="author-section__title">Contact & Web</h2>
					<div class="author-section__content author-section__content--grid">
						<div class="author-field">
							<label for="author-website" class="author-field__label">
								<GlobeIcon size={14} weight="bold" />
								<span>Website</span>
							</label>
							<input
								id="author-website"
								name="author-website"
								type="url"
								class="author-field__input"
								bind:value={websiteUrl}
								placeholder="https://yoursite.com"
							/>
						</div>
						<div class="author-field">
							<label for="author-twitter" class="author-field__label">
								<TwitterLogoIcon size={14} weight="bold" />
								<span>Twitter / X</span>
							</label>
							<input
								id="author-twitter"
								name="author-twitter"
								type="url"
								class="author-field__input"
								bind:value={twitterUrl}
								placeholder="https://x.com/yourusername"
							/>
						</div>
						<div class="author-field">
							<label for="author-linkedin" class="author-field__label">
								<LinkedinLogoIcon size={14} weight="bold" />
								<span>LinkedIn</span>
							</label>
							<input
								id="author-linkedin"
								name="author-linkedin"
								type="url"
								class="author-field__input"
								bind:value={linkedinUrl}
								placeholder="https://linkedin.com/in/yourprofile"
							/>
						</div>
						<div class="author-field">
							<label for="author-youtube" class="author-field__label">
								<YoutubeLogoIcon size={14} weight="bold" />
								<span>YouTube</span>
							</label>
							<input
								id="author-youtube"
								name="author-youtube"
								type="url"
								class="author-field__input"
								bind:value={youtubeUrl}
								placeholder="https://youtube.com/@yourchannel"
							/>
						</div>
						<div class="author-field">
							<label for="author-instagram" class="author-field__label">
								<InstagramLogoIcon size={14} weight="bold" />
								<span>Instagram</span>
							</label>
							<input
								id="author-instagram"
								name="author-instagram"
								type="url"
								class="author-field__input"
								bind:value={instagramUrl}
								placeholder="https://instagram.com/yourhandle"
							/>
						</div>
					</div>
				</section>

				<!-- Sticky Save Bar -->
				<div class="author-save-bar">
					<div class="author-save-bar__status">
						{#if error}
							<span class="author-save-bar__error">{error}</span>
						{/if}
						{#if saved}
							<span class="author-save-bar__success">Profile saved!</span>
						{/if}
					</div>
					<button type="submit" disabled={saving} class="author-save-bar__btn">
						<FloppyDiskIcon size={16} weight="bold" />
						<span>{saving ? 'Saving...' : 'Save Profile'}</span>
					</button>
				</div>
			</div>
		</form>
	{/if}
</div>

<style>
	.author-profile {
		max-width: 1200px;
	}

	.author-profile__header {
		margin-bottom: 2rem;
	}

	.author-profile__eyebrow {
		display: inline-block;
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		margin-bottom: 0.5rem;
	}

	.author-profile__title {
		font-size: 1.5rem;
		font-weight: 700;
		font-family: var(--font-heading);
		color: var(--color-white);
		line-height: 1.15;
		letter-spacing: -0.01em;
		margin: 0 0 0.35rem;
	}

	.author-profile__subtitle {
		color: var(--color-grey-400);
		font-size: 0.875rem;
		line-height: 1.55;
		margin: 0;
	}

	.author-profile__loading {
		color: var(--color-grey-400);
		padding: 2rem 0;
	}

	.author-profile__form {
		display: grid;
		grid-template-columns: 1fr;
		gap: 1rem;
	}

	.author-profile__aside {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		min-width: 0;
	}

	.author-profile__main {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		min-width: 0;
	}

	/* Sections */
	.author-section {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		overflow: hidden;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}

	.author-section__title {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		padding: 0.875rem 1.25rem;
		margin: 0;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		background: rgba(255, 255, 255, 0.02);
	}

	.author-section__content {
		padding: 1.25rem;
	}

	.author-section__content--grid {
		display: grid;
		grid-template-columns: 1fr;
		gap: 1.25rem;
	}

	/* Avatar */
	.avatar-editor {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1.25rem;
		text-align: center;
	}

	.avatar-editor__preview {
		width: 112px;
		height: 112px;
		border-radius: var(--radius-full);
		overflow: hidden;
		border: 1px solid rgba(255, 255, 255, 0.1);
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: rgba(255, 255, 255, 0.05);
		color: var(--color-grey-400);
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
	}

	.avatar-editor__img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.avatar-editor__placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.avatar-editor__controls {
		flex: 1;
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		align-items: stretch;
		text-align: left;
	}

	.btn-upload-avatar {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		min-height: 2.5rem;
		padding: 0 1rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		border-radius: var(--radius-lg);
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out);
	}

	.btn-upload-avatar:hover {
		background-color: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.18);
	}

	.btn-upload-avatar--loading {
		opacity: 0.6;
		cursor: not-allowed;
	}

	/* Fields */
	.author-field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}

	.author-field--full {
		grid-column: 1 / -1;
	}

	.author-field__label {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.author-field__hint {
		font-size: 0.75rem;
		font-weight: var(--w-regular);
		color: var(--color-grey-500);
		text-transform: none;
		letter-spacing: 0;
		margin-left: auto;
	}

	.author-field__input {
		min-height: 2.5rem;
		padding: 0.65rem 0.875rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		font-size: 0.875rem;
		color: var(--color-white);
		transition:
			border-color 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out);
		width: 100%;
		box-sizing: border-box;
	}

	.author-field__input::placeholder {
		color: var(--color-grey-500);
	}

	.author-field__input:focus {
		outline: none;
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	.author-field__textarea {
		padding: 0.65rem 0.875rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		font-size: 0.875rem;
		color: var(--color-white);
		resize: vertical;
		font-family: inherit;
		line-height: 1.55;
		width: 100%;
		box-sizing: border-box;
		transition:
			border-color 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out);
	}

	.author-field__textarea::placeholder {
		color: var(--color-grey-500);
	}

	.author-field__textarea:focus {
		outline: none;
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	/* Sticky Save Bar */
	.author-save-bar {
		position: sticky;
		bottom: 0;
		z-index: 5;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.875rem 1rem;
		margin-top: 0.5rem;
		background-color: rgba(15, 23, 42, 0.85);
		backdrop-filter: blur(16px);
		-webkit-backdrop-filter: blur(16px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.3);
	}

	.author-save-bar__status {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-width: 0;
		flex: 1;
	}

	.author-save-bar__error {
		font-size: 0.875rem;
		color: #fca5a5;
	}

	.author-save-bar__success {
		font-size: 0.875rem;
		color: #4ade80;
		font-weight: 500;
	}

	.author-save-bar__btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 2.5rem;
		padding: 0 1.25rem;
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94));
		color: var(--color-white);
		border: none;
		border-radius: var(--radius-lg);
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45);
		transition:
			opacity 200ms var(--ease-out),
			transform 200ms var(--ease-out),
			box-shadow 200ms var(--ease-out);
	}

	.author-save-bar__btn:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 10px 24px -6px rgba(15, 164, 175, 0.55);
	}

	.author-save-bar__btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* Tablet (>=768px) — sections breathe a bit, two-column form fields */
	@media (min-width: 768px) {
		.author-profile__title {
			font-size: 1.5rem;
		}

		.author-profile__form {
			gap: 1.25rem;
		}

		.author-profile__aside,
		.author-profile__main {
			gap: 1.25rem;
		}

		.author-section {
			border-radius: var(--radius-2xl);
		}

		.author-section__content {
			padding: 1.75rem;
		}

		.author-section__content--grid {
			grid-template-columns: 1fr 1fr;
		}

		.avatar-editor {
			flex-direction: row;
			align-items: flex-start;
			text-align: left;
			gap: 1.25rem;
		}

		.avatar-editor__preview {
			width: 96px;
			height: 96px;
		}
	}

	/* Desktop (>=1024px) — true two-column layout */
	@media (min-width: 1024px) {
		.author-profile__form {
			grid-template-columns: 320px minmax(0, 1fr);
			align-items: start;
			gap: 2rem;
		}

		.author-profile__aside {
			position: sticky;
			top: 1rem;
		}

		.avatar-editor {
			flex-direction: column;
			align-items: center;
			text-align: center;
		}

		.avatar-editor__preview {
			width: 128px;
			height: 128px;
		}

		.avatar-editor__controls {
			text-align: left;
		}
	}
</style>
