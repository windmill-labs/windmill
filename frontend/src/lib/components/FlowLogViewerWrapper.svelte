<script lang="ts">
	import type { FlowStatusModule, Job } from '$lib/gen'
	import type { Writable } from 'svelte/store'
	import type { GraphModuleState } from './graph'
	import FlowLogViewer from './FlowLogViewer.svelte'

	interface Props {
		innerModules: FlowStatusModule[]
		job: Job
		localModuleStates: Writable<Record<string, GraphModuleState>>
		workspaceId: string | undefined
		render: boolean
		prefix?: string
		level?: number
	}

	let { innerModules, job, localModuleStates, workspaceId, render }: Props = $props()
</script>

<div class="w-full rounded-md overflow-hidden border">
	<FlowLogViewer {innerModules} {job} {localModuleStates} {workspaceId} {render} />
</div>
