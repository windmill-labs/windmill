<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { AuditLog } from '$lib/gen'
	import { ArrowRight } from 'lucide-svelte'

	export let logs: AuditLog[]
	export let selectedId: number | undefined = undefined
</script>

<div class="p-4 flex flex-col gap-2 border-t items-start">
	{#if selectedId}
		{@const log = logs.find((e) => e.id === selectedId)}
		{#if log}
			<span class="font-semibold text-xs leading-6">ID</span>
			<span class="text-xs">{log.id}</span>
			<span class="font-semibold text-xs leading-6">Parameters</span>
			<div class="text-xs p-2 bg-surface-secondary rounded-sm">
				{JSON.stringify(log.parameters, null, 2)}
			</div>

			{#if log?.parameters?.uuid}
				<Button
					href={`run/${log.parameters.uuid}`}
					color="light"
					variant="border"
					size="xs"
					target="_blank"
				>
					View run
				</Button>
			{/if}

			{#if log.operation === AuditLog.operation.JOBS_RUN_SCRIPT}
				<Button
					href={`scripts/get/${log.resource}`}
					color="dark"
					variant="contained"
					size="xs"
					target="_blank"
				>
					View script
				</Button>
			{/if}

			{#if [AuditLog.operation.JOBS_RUN_FLOW, AuditLog.operation.FLOWS_CREATE, AuditLog.operation.FLOWS_UPDATE].includes(log.operation)}
				<Button
					href={`flows/get/${log.resource}`}
					color="dark"
					variant="contained"
					size="xs"
					target="_blank"
					endIcon={{ icon: ArrowRight }}
				>
					View flow
				</Button>
			{/if}
			{#if [AuditLog.operation.APPS_UPDATE, AuditLog.operation.APPS_CREATE].includes(log.operation)}
				<Button
					href={`apps/get/${log.resource}`}
					color="dark"
					variant="contained"
					size="xs"
					target="_blank"
					endIcon={{ icon: ArrowRight }}
				>
					View app
				</Button>
			{/if}
		{/if}
	{:else}
		<span class="text-xs">No log selected</span>
	{/if}
</div>
