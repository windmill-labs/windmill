<script lang="ts">
	import { HelpersService } from '$lib/gen'
	import type { WindmillFilePreview } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Skeleton from './common/skeleton/Skeleton.svelte'

	export let fileKey: string
	export let storage: string | undefined
	export let workspaceId: string | undefined

	let filePreview: WindmillFilePreview | undefined = undefined
	let tooBig = false

	async function loadImagePreview(
		fileKey: string,
		storage: string | undefined,
		workspaceId: string | undefined
	) {
		filePreview = undefined
		tooBig = false
		const fileMetadata = await HelpersService.loadFileMetadata({
			fileKey,
			storage,
			workspace: workspaceId ?? $workspaceStore!
		})

		if (fileMetadata.size_in_bytes && fileMetadata.size_in_bytes > 8 * 1024 * 1024) {
			tooBig = true
			return
		}

		filePreview = await HelpersService.loadFilePreview({
			workspace: workspaceId ?? $workspaceStore!,
			fileKey: fileKey,
			fileSizeInBytes: fileMetadata.size_in_bytes,
			fileMimeType: fileMetadata.mime_type,
			readBytesFrom: 0,
			readBytesLength: 8 * 1024 * 1024,
			storage: storage
		})
	}

	$: loadImagePreview(fileKey, storage, workspaceId)
</script>

<div class="h-full mt-4">
	{#if filePreview}
		<img alt="preview rendered" class="w-auto h-full" src={filePreview.content} />
	{:else if tooBig}
		<div class="text-sm">Image too large to preview (&gt 8MB) </div>
	{:else}
		<Skeleton layout={[[20]]} />
	{/if}
</div>
