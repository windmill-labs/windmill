<script lang="ts">
	import { emptyString } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import { HelpersService } from '$lib/gen'
	import { Download } from 'lucide-svelte'

	export let s3object: any

	async function downloadS3File(fileKey: string | undefined) {
		if (emptyString(fileKey)) {
			return
		}
		const downloadUrl = await HelpersService.generateDownloadUrl({
			workspace: $workspaceStore!,
			fileKey: fileKey!
		})
		console.log('download URL ', downloadUrl.download_url)
		window.location.assign(downloadUrl.download_url)
		// window.open(downloadUrl.download_url, '_blank')
	}
</script>

<button
	class="relative center-center flex w-full text-center font-medium text-tertiary
					border-2 border-dashed border-gray-400 hover:border-blue-500
					focus-within:border-blue-500 hover:bg-blue-50 dark:hover:bg-frost-900 focus-within:bg-blue-50
					duration-200 rounded-lg p-1 gap-2"
	on:click={() => {
		downloadS3File(s3object?.s3)
	}}
>
	<Download />
	<span>s3://{s3object.s3}</span>
</button>
