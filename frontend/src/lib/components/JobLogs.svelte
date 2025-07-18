<script lang="ts">
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import LogViewer from './LogViewer.svelte'

	export let jobId: string
	export let tagLabel: string | undefined = undefined

	let logs: string | undefined = undefined

	$: jobId && loadLogs()
	async function loadLogs() {
		logs = await JobService.getJobLogs({ workspace: $workspaceStore!, id: jobId })
	}
</script>

<LogViewer content={logs} isLoading={false} tag={undefined} {jobId} {tagLabel} />
