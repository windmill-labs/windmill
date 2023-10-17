<script lang="ts">
	import FlowStatusViewerInner from "./FlowStatusViewerInner.svelte"

	export let jobId: string
	export let workspaceId: string | undefined = undefined
	export let flowState: FlowState | undefined = undefined
	export let flowJobIds:
		| {
				moduleId: string
				flowJobs: string[]
		  }
		| undefined = undefined
	export let job: Job | undefined = undefined

	export let flowModuleStates: Record<string, GraphModuleState> = {}

	export let retry_status: Record<string, number> = {}
	export let suspend_status: number | undefined = undefined
	export let render = true

	export let is_owner = false
</script>

<FlowStatusViewerInner on:jobsLoaded {jobId} {workspaceId} {flowState}