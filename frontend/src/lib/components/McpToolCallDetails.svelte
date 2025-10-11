<script lang="ts">
	import { Badge, Button } from './common'
	import JobArgs from './JobArgs.svelte'
	import DisplayResult from './DisplayResult.svelte'
	import { Copy } from 'lucide-svelte'
	import { copyToClipboard } from '$lib/utils'
	import type { FlowStatusModule } from '$lib/gen'

	interface Props {
		functionName: string
		resourcePath: string
		args: any
		result: any
		type: FlowStatusModule['type']
		workspaceId?: string | undefined
		jobId?: string | undefined
	}

	let {
		functionName,
		resourcePath,
		args,
		result,
		type,
		workspaceId = undefined,
		jobId = undefined
	}: Props = $props()
</script>

<div class="p-2 flex flex-col gap-4">
	<!-- Header -->
	<div class="flex items-center gap-2">
		<span class="font-semibold text-sm">{functionName}</span>
		<Badge color={type === 'Success' ? 'green' : type === 'Failure' ? 'red' : 'gray'}>
			{type}
		</Badge>
	</div>

	<!-- Resource Path Section -->
	<div>
		<h3 class="text-sm font-semibold mb-2 text-secondary">Resource</h3>
		<div class="flex items-center gap-2 bg-surface-secondary rounded p-2">
			<code class="text-xs flex-1 truncate">{resourcePath}</code>
			<Button
				size="xs2"
				color="light"
				variant="border"
				startIcon={{ icon: Copy }}
				on:click={() => copyToClipboard(resourcePath)}
			>
				Copy
			</Button>
		</div>
	</div>

	<!-- Arguments Section -->
	{#if args && typeof args === 'object' && Object.keys(args).length > 0}
		<div>
			<h3 class="text-sm font-semibold mb-2 text-secondary">Arguments</h3>
			<JobArgs {args} argLabel="Parameter" />
		</div>
	{:else}
		<div>
			<h3 class="text-sm font-semibold mb-2 text-secondary">Arguments</h3>
			<p class="text-xs text-tertiary italic">No arguments</p>
		</div>
	{/if}

	<!-- Result Section -->
	<div>
		<h3 class="text-sm font-semibold mb-2 text-secondary">Result</h3>
		<div class="border rounded">
			<DisplayResult {result} {workspaceId} {jobId} />
		</div>
	</div>
</div>
