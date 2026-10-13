<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { type AuditLog } from '$lib/gen'
	import { displayDate } from '$lib/utils'
	import { ExternalLink, X } from 'lucide-svelte'

	interface Props {
		logs: AuditLog[]
		selectedId?: number | undefined
		onClose?: () => void
	}

	let { logs, selectedId = undefined, onClose }: Props = $props()

	// `span` holds the caller's token prefix, except for job-minted worker tokens, which
	// stamp the job they run for instead.
	const JOB_SPAN_PREFIX = 'job-span-'

	const ViewFlowOp: AuditLog['operation'][] = ['jobs.run.flow', 'flows.create', 'flows.update']

	const ViewAppOp: AuditLog['operation'][] = ['apps.create', 'apps.update']
</script>

<div class="flex flex-col items-start">
	{#if selectedId}
		{@const log = logs.find((e) => e.id === selectedId)}
		{#if log}
			<div class="flex flex-row items-center justify-between gap-2 w-full px-4 py-2 border-b">
				<div class="flex flex-col min-w-0">
					<span class="text-xs font-semibold text-emphasis truncate">{log.operation}</span>
					<span class="text-2xs text-secondary">{displayDate(log.timestamp)}</span>
				</div>
				{#if onClose}
					<Button
						variant="subtle"
						unifiedSize="sm"
						iconOnly
						startIcon={{ icon: X }}
						onClick={onClose}
						aria-label="Close log details"
					/>
				{/if}
			</div>
			<div class="flex flex-col gap-6 w-full p-4">
				<div class="flex flex-col gap-1">
					<span class="font-semibold text-xs text-emphasis">ID</span>
					<span class="text-xs">{log.id}</span>
				</div>
				{#if log.span}
					{@const isJobSpan = log.span.startsWith(JOB_SPAN_PREFIX)}
					<div class="flex flex-col gap-1">
						<span class="font-semibold text-xs text-emphasis">
							{isJobSpan ? 'Job' : 'Token prefix'}
						</span>
						<span class="text-xs break-all">
							{isJobSpan ? log.span.slice(JOB_SPAN_PREFIX.length) : log.span}
						</span>
					</div>
				{/if}
				<div class="flex flex-col gap-1">
					<span class="font-semibold text-xs text-emphasis">Parameters</span>
					<div class="text-xs p-2 bg-surface-secondary rounded-md">
						{JSON.stringify(log.parameters, null, 2)}
					</div>
				</div>

				{#if log?.parameters?.uuid}
					<Button
						href={`run/${log.parameters.uuid}`}
						variant="default"
						unifiedSize="md"
						target="_blank"
						endIcon={{ icon: ExternalLink }}
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
						endIcon={{ icon: ExternalLink }}
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
						endIcon={{ icon: ExternalLink }}
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
						endIcon={{ icon: ExternalLink }}
						wrapperClasses="w-fit"
					>
						View app
					</Button>
				{/if}
			</div>
		{/if}
	{:else}
		<span class="p-4 text-xs text-primary font-normal">No log selected</span>
	{/if}
</div>
