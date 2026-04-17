<!--
  FORM-10: File upload field. The heavy work (chunked upload + MIME sniff)
  lives in FORM-05's upload pipeline; this component emits a list of
  `{ file_id, filename, mime_type, size, sha256 }` descriptors as the field
  value once the backend has persisted the parts.

  For v1 we POST the file to `/api/forms/uploads` as a single multipart body
  and surface progress via `aria-busy` on the control. The `files` array on
  `SubmitRequest` is populated from the current value when submitting.
-->
<script lang="ts">
	import FieldFrame from './FieldFrame.svelte';
	import type { FieldProps, FieldSchema } from '../types.ts';
	import { api } from '$lib/api/client';
	import Paperclip from 'phosphor-svelte/lib/Paperclip';
	import X from 'phosphor-svelte/lib/X';

	interface FileDescriptor {
		readonly field_key: string;
		readonly file_id: string;
		readonly filename: string;
		readonly mime_type: string;
		readonly size: number;
		readonly sha256: string;
	}

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const fu = $derived(field as Extract<FieldSchema, { type: 'file_upload' }>);
	const current = $derived<readonly FileDescriptor[]>(
		Array.isArray(value) ? (value as FileDescriptor[]) : []
	);
	const controlId = $derived(`form-field-${field.key}`);
	const accept = $derived((fu.allowed_mime_types ?? []).join(','));
	const maxFiles = $derived(fu.max_files ?? Number.MAX_SAFE_INTEGER);

	let uploading = $state(false);
	let localError = $state('');

	async function handlePick(e: Event) {
		const target = e.currentTarget as HTMLInputElement;
		const picked = target.files ? Array.from(target.files) : [];
		if (picked.length === 0) return;

		uploading = true;
		localError = '';
		try {
			const uploaded: FileDescriptor[] = [...current];
			for (const f of picked) {
				if (uploaded.length >= maxFiles) break;
				const fd = new FormData();
				fd.append('file', f);
				fd.append('field_key', field.key);
				const res = await api.upload<{
					file_id: string;
					filename: string;
					mime_type: string;
					size: number;
					sha256: string;
				}>('/api/forms/uploads', fd, { skipAuth: true });
				uploaded.push({
					field_key: field.key,
					file_id: res.file_id,
					filename: res.filename,
					mime_type: res.mime_type,
					size: res.size,
					sha256: res.sha256
				});
			}
			onChange(field.key, uploaded);
			target.value = '';
		} catch (err) {
			localError = err instanceof Error ? err.message : 'Upload failed.';
		} finally {
			uploading = false;
		}
	}

	function removeAt(idx: number) {
		const next = current.filter((_, i) => i !== idx);
		onChange(field.key, next);
	}
</script>

<FieldFrame
	{controlId}
	label={field.label ?? field.key}
	helpText={field.helpText}
	error={error ?? (localError || undefined)}
	required={field.required ?? false}
>
	{#snippet children({ describedBy, invalid, required })}
		<div class="fm-upload">
			<label class="fm-btn fm-btn--ghost fm-upload__trigger" for={controlId}>
				<Paperclip size={18} />
				<span>{uploading ? 'Uploading…' : 'Choose file(s)'}</span>
			</label>
			<input
				id={controlId}
				type="file"
				class="fm-upload__input"
				multiple={maxFiles > 1}
				{accept}
				disabled={disabled || uploading}
				aria-describedby={describedBy}
				aria-invalid={invalid}
				aria-required={required}
				aria-busy={uploading}
				onchange={handlePick}
			/>
			{#if current.length > 0}
				<ul class="fm-upload__list">
					{#each current as f, i (f.file_id)}
						<li class="fm-upload__item">
							<span class="fm-upload__filename" title={f.filename}>{f.filename}</span>
							<button
								type="button"
								class="fm-upload__remove"
								onclick={() => removeAt(i)}
								aria-label={`Remove ${f.filename}`}
								disabled={disabled || uploading}
							>
								<X size={16} />
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/snippet}
</FieldFrame>
