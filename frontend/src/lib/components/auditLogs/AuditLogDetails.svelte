<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { type AuditLog } from '$lib/gen'
	import { ArrowRight } from 'lucide-svelte'

	interface Props {
		logs: AuditLog[]
		selectedId?: number | undefined
	}

	let { logs, selectedId = undefined }: Props = $props()

	const ViewFlowOp: AuditLog['operation'][] = ['jobs.run.flow', 'flows.create', 'flows.update']

	const ViewAppOp: AuditLog['operation'][] = ['apps.create', 'apps.update']
</script>

<div class="p-4 flex flex-col gap-2 border-t items-start">
	{#if selectedId}
		{@const log = logs.find((e) => e.id === selectedId)}
		{#if log}
			<div class="flex flex-col gap-6 w-full">
				<div class="flex flex-col gap-1">
					<span class="font-semibold text-xs text-emphasis">ID</span>
					<span class="text-xs">{log.id}</span>
				</div>
				<div class="flex flex-col gap-1">
					<span class="font-semibold text-xs text-emphasis">Parameters</span>
					<div class="text-xs p-2 bg-surface-secondary rounded-md">
						{JSON.stringify(log.parameters, null, 2)}
					</div>
				</div>

				{#if log?.parameters?.uuid}
					<Button
						href={`run/${log.parameters.uuid}`}
						variant="accent"
						unifiedSize="md"
						target="_blank"
						wrapperClasses="w-fit"
					>
						View run
					</Button>
				{/if}

				{#if log.operation === 'jobs.run.script'}
					<Button
						href={`scripts/get/${log.resource}`}
						variant="default"
						unifiedSize="md"
						target="_blank"
						wrapperClasses="w-fit"
					>
						View script
					</Button>
				{/if}

				{#if ViewFlowOp.includes(log.operation)}
					<Button
						href={`flows/get/${log.resource}`}
						variant="default"
						unifiedSize="md"
						target="_blank"
						endIcon={{ icon: ArrowRight }}
						wrapperClasses="w-fit"
					>
						View flow
					</Button>
				{/if}
				{#if ViewAppOp.includes(log.operation)}
					<Button
						href={`apps/get/${log.resource}`}
						variant="default"
						unifiedSize="md"
						target="_blank"
						endIcon={{ icon: ArrowRight }}
						wrapperClasses="w-fit"
					>
						View app
					</Button>
				{/if}
			</div>
		{/if}
	{:else}
		<span class="text-xs text-primary font-normal">No log selected</span>
	{/if}
</div>
