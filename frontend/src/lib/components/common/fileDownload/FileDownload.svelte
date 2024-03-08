<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { Download } from 'lucide-svelte'

	export let s3object: any
</script>

<a
	class="relative center-center flex w-full text-center font-normal text-tertiary text-sm
border border-dashed border-gray-400 hover:border-blue-500
focus-within:border-blue-500 hover:bg-blue-50 dark:hover:bg-frost-900 focus-within:bg-blue-50
duration-200 rounded-lg p-1 gap-2"
	href={`/api/w/${$workspaceStore}/job_helpers/download_s3_file?file_key=${s3object?.s3}`}
	download={s3object?.s3.split('/').pop() ?? 'unnamed_download.file'}
>
	<Download />
	<span>s3://{s3object.s3}</span>
</a>
