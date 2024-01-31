<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { Download } from 'lucide-svelte'

	export let s3object: any
</script>

<a
	href={`/api/w/${$workspaceStore}/job_helpers/download_s3_file?file_key=${s3object?.s3}`}
	download={s3object?.s3.split('/').pop() ?? 'unnamed_download.file'}
>
	<Download />
	<span>s3://{s3object.s3}</span>
</a>
