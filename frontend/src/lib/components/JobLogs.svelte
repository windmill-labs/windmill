<script lang="ts">
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import LogViewer from './LogViewer.svelte'

	interface Props {
		jobId: string
		tagLabel?: string | undefined
	}

	let { jobId, tagLabel = undefined }: Props = $props()

	let logs: string | undefined = $state(undefined)

	async function loadLogs() {
		logs = await JobService.getJobLogs({ workspace: $workspaceStore!, id: jobId })
	}
	$effect(() => {
		jobId && loadLogs()
	})
</script>

<LogViewer content={logs} isLoading={false} tag={undefined} {jobId} {tagLabel} />
