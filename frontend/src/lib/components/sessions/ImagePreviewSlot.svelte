<script lang="ts">
	import { Button } from '$lib/components/common'
	import ImageViewer from '$lib/components/common/image/ImageViewer.svelte'
	import type { AIChatManager } from '../copilot/chat/AIChatManager.svelte'
	import { asImageBlob, imageContentKey } from '../copilot/chat/files/imageFiles'
	import type { ImageSource } from './previewRouter'
	import { resource } from 'runed'
	import { JobService } from '$lib/gen'

	interface Props {
		manager: AIChatManager
		source: ImageSource
		label: string
	}

	let { manager, source, label }: Props = $props()

	// A session link, never a message attachment: those share the namespace only by name.
	const file = $derived(
		source.kind === 'file'
			? manager.attachedFiles.files.find(
					(f) => f.name === source.name && !f.messageScoped && !f.isFolderRoot
				)
			: undefined
	)
	// A folder whose permission lapsed after a reload keeps only its placeholder row.
	const folderLocked = $derived(
		source.kind === 'file' &&
			!file &&
			manager.attachedFiles.folders.some(
				(f) => f.status === 'locked' && source.name.startsWith(`${f.name}/`)
			)
	)

	const messageImage = $derived.by(() => {
		if (source.kind !== 'message') return undefined
		// A user's attachment, or an image a tool produced (a screenshot).
		for (const m of manager.displayMessages) {
			if (m.role === 'user') {
				for (const image of m.images ?? []) {
					if (imageContentKey(image.dataUrl) === source.hash) return image
				}
			} else if (m.role === 'tool' && m.imageUrl?.startsWith('data:')) {
				if (imageContentKey(m.imageUrl) === source.hash)
					return { dataUrl: m.imageUrl, name: undefined }
			}
		}
		return undefined
	})

	const blob = $derived(
		file && file.status === 'ready'
			? asImageBlob(file.file, file.sourceName ?? file.name)
			: undefined
	)

	// The object URL pins the blob in memory until revoked, so it lives exactly as long as
	// the blob it was made from is the one shown.
	const blobUrl = $derived(blob ? URL.createObjectURL(blob) : undefined)
	$effect(() => {
		const url = blobUrl
		return () => {
			if (url) URL.revokeObjectURL(url)
		}
	})

	// An inline result is fetched again from its job: the tab keeps only the job's address.
	const resultImage = resource(
		() => (source.kind === 'result' ? source : undefined),
		async (s) => {
			if (!s) return undefined
			const result = (
				s.nodeId
					? await JobService.resultById({
							workspace: s.workspace,
							flowJobId: s.jobId,
							nodeId: s.nodeId
						})
					: await JobService.getCompletedJobResult({ workspace: s.workspace, id: s.jobId })
			) as Record<string, unknown> | undefined
			// DisplayResult takes either the base64 string or `{ content }`.
			const value = result?.[s.format]
			const b64 =
				typeof value === 'string'
					? value
					: typeof (value as { content?: unknown })?.content === 'string'
						? (value as { content: string }).content
						: undefined
			return b64 ? `data:image/${s.format};base64,${b64}` : null
		}
	)

	const src = $derived(
		source.kind === 'message'
			? messageImage?.dataUrl
			: source.kind === 'result'
				? (resultImage.current ?? undefined)
				: source.kind === 'url'
					? source.url
					: blobUrl
	)
	const name = $derived(
		source.kind === 'file'
			? source.name.split('/').pop() || source.name
			: source.kind === 'message'
				? messageImage?.name || label || 'Image'
				: source.kind === 'result'
					? `${label || 'result'}.${source.format === 'jpeg' ? 'jpg' : source.format}`
					: label || 'Image'
	)
</script>

{#if src}
	<ImageViewer {src} {name} bytes={file?.size} />
{:else if folderLocked}
	<div class="flex flex-col items-start gap-2 p-4 text-xs text-secondary">
		<span>Your browser needs permission again to read this folder.</span>
		<Button
			unifiedSize="sm"
			variant="default"
			onClick={() => void manager.attachedFiles.regrantLocked()}
		>
			Restore access
		</Button>
	</div>
{:else if source.kind === 'result'}
	<div class="p-4 text-xs text-secondary">
		{resultImage.loading
			? 'Loading the result…'
			: resultImage.error
				? 'The result of this run could not be loaded.'
				: 'This run’s result no longer holds an image.'}
	</div>
{:else if file && file.status !== 'ready'}
	<div class="p-4 text-xs text-secondary">
		{file.status === 'indexing' ? 'Loading…' : 'This image is no longer available.'}
	</div>
{:else}
	<div class="p-4 text-xs text-secondary">
		{source.kind === 'file'
			? 'This image is no longer linked to the session.'
			: source.kind === 'message'
				? 'This image is not in the current chat.'
				: 'This image could not be loaded.'}
	</div>
{/if}
