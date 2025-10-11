<script lang="ts">
	import { Badge } from './common'
	import JobArgs from './JobArgs.svelte'
	import DisplayResult from './DisplayResult.svelte'
	import type { FlowStatusModule } from '$lib/gen'

	interface Props {
		functionName: string
		args: any
		result: any
		type: FlowStatusModule['type']
		workspaceId?: string | undefined
	}

	let { functionName, args, result, type, workspaceId = undefined }: Props = $props()
</script>

<div class="p-2 flex flex-col gap-4">
	<!-- Header -->
	<div class="flex items-center gap-2">
		<span class="font-semibold text-sm">{functionName}</span>
		<Badge color={type === 'Success' ? 'green' : type === 'Failure' ? 'red' : 'gray'}>
			{type}
		</Badge>
	</div>

	<!-- Arguments Section -->
	<div>
		<h3 class="text-sm font-semibold mb-2 text-secondary">Arguments</h3>
		{#if args && typeof args === 'object' && Object.keys(args).length > 0}
			<JobArgs {args} argLabel="Parameter" />
		{:else}
			<p class="text-xs text-tertiary italic">No arguments</p>
		{/if}
	</div>

	<!-- Result Section -->
	<div>
		<h3 class="text-sm font-semibold mb-2 text-secondary">Result</h3>
		<div class="border rounded">
			<DisplayResult {result} {workspaceId} />
		</div>
	</div>
</div>
