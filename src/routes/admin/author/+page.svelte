<script lang="ts">
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { UserResponse } from '$lib/api/types';
	import UserCircle from 'phosphor-svelte/lib/UserCircle';
	import Image from 'phosphor-svelte/lib/Image';
	import TwitterLogo from 'phosphor-svelte/lib/TwitterLogo';
	import LinkedinLogo from 'phosphor-svelte/lib/LinkedinLogo';
	import YoutubeLogo from 'phosphor-svelte/lib/YoutubeLogo';
	import InstagramLogo from 'phosphor-svelte/lib/InstagramLogo';
	import Globe from 'phosphor-svelte/lib/Globe';
	import FloppyDisk from 'phosphor-svelte/lib/FloppyDisk';

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
		<h1 class="author-profile__title">Author Profile</h1>
		<p class="author-profile__subtitle">This information appears on your published blog posts.</p>
	</div>

	{#if loading}
		<div class="author-profile__loading">Loading profile...</div>
	{:else}
		<form onsubmit={handleSave} class="author-profile__form">
			<!-- Profile Photo -->
			<section class="author-section">
				<h2 class="author-section__title">Profile Photo</h2>
				<div class="author-section__content">
					<div class="avatar-editor">
						<div class="avatar-editor__preview">
							{#if avatarUrl}
								<img src={avatarUrl} alt="Author profile" class="avatar-editor__img" />
							{:else}
								<div class="avatar-editor__placeholder">
									<UserCircle size={64} weight="thin" />
								</div>
							{/if}
						</div>
						<div class="avatar-editor__controls">
							<label class="btn-upload-avatar" class:btn-upload-avatar--loading={uploadingAvatar}>
								<Image size={16} weight="bold" />
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
							Biographical Info
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
							<Globe size={14} weight="bold" />
							Website
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
							<TwitterLogo size={14} weight="bold" />
							Twitter / X
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
							<LinkedinLogo size={14} weight="bold" />
							LinkedIn
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
							<YoutubeLogo size={14} weight="bold" />
							YouTube
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
							<InstagramLogo size={14} weight="bold" />
							Instagram
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

			<!-- Save Bar -->
			<div class="author-save-bar">
				{#if error}
					<span class="author-save-bar__error">{error}</span>
				{/if}
				{#if saved}
					<span class="author-save-bar__success">Profile saved!</span>
				{/if}
				<button type="submit" disabled={saving} class="author-save-bar__btn">
					<FloppyDisk size={16} weight="bold" />
					{saving ? 'Saving...' : 'Save Profile'}
				</button>
			</div>
		</form>
	{/if}
</div>

<style>
	.author-profile {
		max-width: 760px;
		padding: 2rem;
	}

	.author-profile__header {
		margin-bottom: 2rem;
	}

	.author-profile__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-navy);
		margin: 0 0 0.25rem;
	}

	.author-profile__subtitle {
		color: var(--color-grey-500);
		font-size: var(--fs-sm);
		margin: 0;
	}

	.author-profile__loading {
		color: var(--color-grey-500);
		padding: 2rem 0;
	}

	.author-profile__form {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	/* Sections */
	.author-section {
		background: #fff;
		border: 1px solid var(--color-grey-200);
		border-radius: 0.75rem;
		overflow: hidden;
	}

	.author-section__title {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-navy);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 0.875rem 1.25rem;
		margin: 0;
		background: var(--color-grey-50, #f8fafc);
		border-bottom: 1px solid var(--color-grey-200);
	}

	.author-section__content {
		padding: 1.5rem 1.25rem;
	}

	.author-section__content--grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.25rem;
	}

	/* Avatar */
	.avatar-editor {
		display: flex;
		align-items: flex-start;
		gap: 1.5rem;
	}

	.avatar-editor__preview {
		width: 96px;
		height: 96px;
		border-radius: 50%;
		overflow: hidden;
		border: 2px solid var(--color-grey-200);
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-grey-100, #f1f5f9);
		color: var(--color-grey-400);
	}

	.avatar-editor__img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.avatar-editor__controls {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.btn-upload-avatar {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		background: var(--color-navy);
		color: #fff;
		border-radius: 0.5rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		cursor: pointer;
		width: fit-content;
		transition: opacity 0.15s;
	}

	.btn-upload-avatar--loading {
		opacity: 0.6;
		cursor: not-allowed;
	}

	/* Fields */
	.author-field {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.author-field--full {
		grid-column: 1 / -1;
	}

	.author-field__label {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-navy);
	}

	.author-field__hint {
		font-size: var(--fs-xs);
		font-weight: var(--w-normal);
		color: var(--color-grey-500);
		margin-left: auto;
	}

	.author-field__input {
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--color-grey-300);
		border-radius: 0.5rem;
		font-size: var(--fs-sm);
		color: var(--color-navy);
		background: #fff;
		transition: border-color 0.15s;
		width: 100%;
		box-sizing: border-box;
	}

	.author-field__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.author-field__textarea {
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--color-grey-300);
		border-radius: 0.5rem;
		font-size: var(--fs-sm);
		color: var(--color-navy);
		background: #fff;
		resize: vertical;
		font-family: inherit;
		line-height: 1.6;
		width: 100%;
		box-sizing: border-box;
		transition: border-color 0.15s;
	}

	.author-field__textarea:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	/* Save bar */
	.author-save-bar {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 1rem;
		padding: 1rem 0;
		border-top: 1px solid var(--color-grey-200);
	}

	.author-save-bar__error {
		font-size: var(--fs-sm);
		color: #ef4444;
	}

	.author-save-bar__success {
		font-size: var(--fs-sm);
		color: #22c55e;
		font-weight: var(--w-medium);
	}

	.author-save-bar__btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.625rem 1.25rem;
		background: var(--color-teal);
		color: #fff;
		border: none;
		border-radius: 0.5rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: opacity 0.15s;
	}

	.author-save-bar__btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	@media (max-width: 600px) {
		.author-section__content--grid {
			grid-template-columns: 1fr;
		}

		.avatar-editor {
			flex-direction: column;
		}
	}
</style>
