<script lang="ts">
	// Standalone preview for an S3-backed object. Extracted from
	// S3FilePickerInner so the asset detail pane (and any other surface that
	// wants to peek at S3 contents) can render the same image/PDF/CSV/text
	// renderings without dragging in the whole picker UI.
	//
	// The component owns its own metadata + preview loads — callers only
	// need to hand it a `fileKey`. CSV separator/header are local state so
	// the user can re-preview the same file with different parsing flags.
	import { FileX2, Loader2 } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		HelpersService,
		type CancelablePromise,
		type LoadFileMetadataData,
		type LoadFileMetadataResponse,
		type LoadFilePreviewData,
		type LoadFilePreviewResponse
	} from '$lib/gen'
	import { displayDate, displaySize, emptyString } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import ExpandableImage from '$lib/components/common/image/ExpandableImage.svelte'

	interface Props {
		fileKey: string | undefined
		// Optional override of the storage backend (matches the picker's
		// `storage` prop). Empty/undefined uses the workspace default.
		storage?: string | undefined
		// Browse this object storage resource directly instead of the
		// workspace storage (matches the picker's `s3ResourcePath` prop).
		s3ResourcePath?: string | undefined
		// Override hooks for non-default backends (e.g. workspace settings
		// preview before the storage is committed). Default to the standard
		// helpers service — same defaults as S3FilePickerInner.
		loadFilePreviewRequest?: (d: LoadFilePreviewData) => CancelablePromise<LoadFilePreviewResponse>
		loadFileMetadataRequest?: (
			d: LoadFileMetadataData
		) => CancelablePromise<LoadFileMetadataResponse>
		// When true, surface a small metadata strip above the preview
		// (size, last-modified, mime). Off by default so callers can lay
		// metadata out themselves (the picker already has its own).
		showMetadata?: boolean
		class?: string
		// Bump this to force a re-fetch (metadata + preview). Used by the
		// asset detail pane after an upstream producer run completes —
		// without it, the "Asset not yet materialized" empty state stays
		// pinned until the user re-selects the asset.
		refreshKey?: any
	}

	let {
		fileKey,
		storage = undefined,
		s3ResourcePath = undefined,
		loadFilePreviewRequest = HelpersService.loadFilePreview,
		loadFileMetadataRequest = HelpersService.loadFileMetadata,
		showMetadata = false,
		class: className = '',
		refreshKey
	}: Props = $props()

	let csvSeparatorChar: string = $state(',')
	let csvHasHeader: boolean = $state(true)

	let fileMetadata = $state<
		| {
				fileKey: string
				mimeType: string | undefined
				size: number | undefined
				sizeStr: string | undefined
				lastModified: string | undefined
		  }
		| undefined
	>(undefined)
	let filePreview = $state<
		| {
				fileKey: string
				contentPreview: string | undefined
				contentType: string | undefined
		  }
		| undefined
	>(undefined)
	let filePreviewLoading = $state(false)
	let fileInfoLoading = $state(false)
	// Distinct error states. `notFound` is the common case for assets that
	// have been declared but never actually materialized (e.g. a fresh
	// pipeline whose producer hasn't run yet) — surfaced as a calm empty
	// state rather than a scary HTTP error. `loadError` is everything else
	// (auth failures, transient S3 hiccups, malformed keys) and shows the
	// raw message so users can debug.
	let notFound = $state(false)
	let loadError = $state<string | undefined>(undefined)

	function isNotFoundError(err: any): boolean {
		// HelpersService surfaces backend errors as ApiError with a `status`
		// field plus a serialized body. We accept either a 404 status or a
		// "not found" substring (case-insensitive) to be robust against
		// future error wrapping changes.
		const status = err?.status ?? err?.response?.status
		if (status === 404) return true
		const body = String(err?.body ?? err?.message ?? err ?? '').toLowerCase()
		return body.includes('not found') || body.includes('404')
	}

	// Reload whenever the file key, workspace, or external refreshKey
	// changes. The refreshKey path is what lets the asset pane re-check
	// existence after an upstream run completes — moving from the
	// "not yet materialized" empty state to the actual preview without
	// requiring the user to re-click the asset.
	$effect(() => {
		const key = fileKey
		const ws = $workspaceStore
		void refreshKey
		if (!key || !ws) {
			fileMetadata = undefined
			filePreview = undefined
			notFound = false
			loadError = undefined
			return
		}
		void loadAll(key)
	})

	async function loadAll(key: string) {
		if (emptyString(key)) {
			fileInfoLoading = false
			return
		}
		fileInfoLoading = true
		filePreview = undefined
		fileMetadata = undefined
		notFound = false
		loadError = undefined
		try {
			const meta = await loadFileMetadataRequest({
				workspace: $workspaceStore!,
				fileKey: key,
				storage,
				s3ResourcePath
			})
			if (meta !== undefined) {
				fileMetadata = {
					fileKey: key,
					size: meta.size_in_bytes,
					sizeStr: displaySize(meta.size_in_bytes),
					mimeType: meta.mime_type,
					lastModified: displayDate(meta.last_modified)
				}
			}
			await reloadPreview(key, meta?.size_in_bytes, meta?.mime_type)
		} catch (err: any) {
			if (isNotFoundError(err)) {
				notFound = true
			} else {
				loadError = err?.body ?? err?.message ?? String(err)
			}
		} finally {
			fileInfoLoading = false
		}
	}

	async function reloadPreview(key: string, size?: number, mimeType?: string) {
		filePreviewLoading = true
		try {
			const raw = await loadFilePreviewRequest({
				workspace: $workspaceStore!,
				fileKey: key,
				fileSizeInBytes: size,
				fileMimeType: mimeType,
				csvSeparator: csvSeparatorChar,
				csvHasHeader: csvHasHeader,
				readBytesFrom: 0,
				readBytesLength: 128 * 1024,
				storage,
				s3ResourcePath
			})
			let content = raw.content
			if (content !== null && content !== undefined && content.length >= 128 * 1024) {
				content = content.substring(0, 128 * 1024 - 35) + '\n\n ... FILE CONTENT TRUNCATED ...\n\n'
			}
			if (raw !== undefined) {
				filePreview = { fileKey: key, contentPreview: content, contentType: raw.content_type }
				if (fileMetadata) {
					fileMetadata.mimeType =
						((key.endsWith('.png') ||
							key.endsWith('.jpg') ||
							key.endsWith('.jpeg') ||
							key.endsWith('.webp')) &&
							'Image') ||
						(key.endsWith('.pdf') && 'PDF') ||
						filePreview.contentType
				}
			}
		} finally {
			filePreviewLoading = false
		}
	}

	// `storage` is keyed by the workspace's S3 storage config name — used as
	// a query-string suffix on the image/PDF preview URLs, together with the
	// optional custom resource override.
	let storageQS = $derived(
		(storage ? `&storage=${storage}` : '') +
			(s3ResourcePath ? `&s3_resource_path=${encodeURIComponent(s3ResourcePath)}` : '')
	)

	function onCsvControlsChanged() {
		if (fileMetadata?.fileKey) {
			void reloadPreview(fileMetadata.fileKey, fileMetadata.size, fileMetadata.mimeType)
		}
	}
</script>

<div class={twMerge('flex flex-col h-full w-full overflow-auto text-xs', className)}>
	{#if showMetadata && fileMetadata}
		<div class="text-2xs text-tertiary px-3 py-1.5 border-b flex flex-wrap gap-x-3 gap-y-0.5">
			{#if fileMetadata.sizeStr}<span>{fileMetadata.sizeStr}</span>{/if}
			{#if fileMetadata.mimeType}<span>{fileMetadata.mimeType}</span>{/if}
			{#if fileMetadata.lastModified}<span>{fileMetadata.lastModified}</span>{/if}
		</div>
	{/if}
	<div class="flex-1 min-h-0 overflow-auto p-4 bg-surface-secondary">
		{#if !fileKey}
			<div class="text-tertiary text-xs">No file selected.</div>
		{:else if fileInfoLoading && !fileMetadata && !notFound && !loadError}
			<div class="flex items-center text-primary gap-1">
				<Loader2 size={12} class="animate-spin" /> Loading…
			</div>
		{:else if notFound}
			<!-- Calm empty state for assets that have been declared (e.g. by a
			     `// out s3://...` annotation or parsed write) but never
			     actually written. Common during pipeline authoring before
			     any producer has run. We surface the file key so users can
			     verify they're looking at the right place. -->
			<div class="flex flex-col items-center justify-center text-center gap-2 py-8">
				<div
					class="flex h-10 w-10 items-center justify-center rounded-full bg-surface-tertiary text-tertiary"
				>
					<FileX2 size={18} />
				</div>
				<div class="text-sm font-medium text-primary">Asset not yet materialized</div>
				<div class="text-2xs text-tertiary max-w-sm">
					This asset has been declared in the pipeline but no producer has written to it yet. Run an
					upstream script to populate it.
				</div>
				<div class="text-3xs font-mono text-tertiary mt-1 break-all max-w-md">{fileKey}</div>
			</div>
		{:else if loadError}
			<!-- Non-404 errors: still structured but show the message so
			     users can debug auth / connectivity / malformed-key issues. -->
			<div class="flex flex-col items-start gap-1 py-2">
				<div class="text-sm font-medium text-red-700 dark:text-red-400">Could not load asset</div>
				<div class="text-2xs text-tertiary break-all">{loadError}</div>
			</div>
		{:else if fileMetadata?.fileKey.endsWith('.png') || fileMetadata?.fileKey.endsWith('.jpg') || fileMetadata?.fileKey.endsWith('.jpeg') || fileMetadata?.fileKey.endsWith('.webp')}
			<div>
				<ExpandableImage
					src={`/api/w/${$workspaceStore}/job_helpers/load_image_preview?file_key=${encodeURIComponent(
						fileMetadata.fileKey
					)}${storageQS}`}
					alt="S3 preview"
					title={fileMetadata.fileKey}
				/>
			</div>
		{:else if fileMetadata?.fileKey.endsWith('.pdf')}
			<div class="w-full h-[950px] border">
				{#await import('$lib/components/display/PdfViewer.svelte')}
					<Loader2 class="animate-spin" />
				{:then Module}
					<Module.default
						source={`/api/w/${$workspaceStore}/job_helpers/load_image_preview?file_key=${encodeURIComponent(
							fileMetadata.fileKey
						)}${storageQS}`}
					/>
				{/await}
			</div>
		{:else if filePreviewLoading}
			<div class="flex h-6 items-center text-primary mb-4">
				<Loader2 size={12} class="animate-spin mr-1" /> File preview loading
			</div>
		{:else if fileMetadata !== undefined && filePreview !== undefined}
			<div class="flex items-center text-primary mb-4">
				{#if filePreview.contentType === 'Unknown'}
					Type of file not supported for preview.
				{:else if filePreview.contentType === 'Csv'}
					Previewing a {filePreview.contentType?.toLowerCase()} file. Separator character:
					<div class="inline-flex w-12 ml-2 mr-2">
						<select class="h-8" bind:value={csvSeparatorChar} onchange={onCsvControlsChanged}>
							<option value=",">,</option>
							<option value=";">;</option>
							<option value="\t">\t</option>
							<option value="|">|</option>
						</select>
					</div>
					Header row:
					<div class="inline-flex item-center w-4 ml-2 mr-2">
						<input
							type="checkbox"
							class="h-5"
							bind:checked={csvHasHeader}
							onchange={onCsvControlsChanged}
						/>
					</div>
				{:else}
					Previewing a {filePreview.contentType?.toLowerCase()} file.
				{/if}
			</div>
			<pre class="grow whitespace-no-wrap break-words"
				>{#if !emptyString(filePreview.contentPreview)}{filePreview.contentPreview}{:else if filePreview.contentType !== undefined}Preview impossible.{/if}</pre
			>
		{/if}
	</div>
</div>
