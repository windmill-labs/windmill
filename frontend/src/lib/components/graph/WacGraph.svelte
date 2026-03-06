<script lang="ts">
	import { base } from '$lib/base'
	import type { WorkflowStatus } from '$lib/gen'
	import {
		CheckCircle2,
		Circle,
		ExternalLink,
		GitBranch,
		Loader2,
		XCircle
	} from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import type { WacParseResult } from './wacToFlow'

	interface Props {
		wacResult?: WacParseResult
		wacStatus?: Record<string, any>
		flowDone?: boolean
	}

	let { wacResult, wacStatus, flowDone = false }: Props = $props()

	let checkpoint = $derived(wacStatus?._checkpoint ?? {})
	let completedSteps: Record<string, any> = $derived(checkpoint?.completed_steps ?? {})
	let pendingSteps = $derived(checkpoint?.pending_steps)

	let childJobs: Record<string, WorkflowStatus> = $derived.by(() => {
		if (!wacStatus || typeof wacStatus !== 'object') return {}
		const result: Record<string, any> = {}
		for (const [k, v] of Object.entries(wacStatus)) {
			if (!k.startsWith('_')) result[k] = v
		}
		return result
	})

	interface StepInfo {
		key: string
		name: string
		status: 'completed' | 'running' | 'pending' | 'failed'
		jobId?: string
		result?: any
		parallelGroup?: number
	}

	let jobIdsByKey: Record<string, string> = $derived.by(() => {
		const ids: Record<string, string> = {}
		for (const [key, val] of Object.entries(pendingSteps?.job_ids ?? {})) {
			if (typeof val === 'string') ids[key] = val
		}
		return ids
	})

	function stepName(key: string, jobId?: string): string {
		return jobId ? (childJobs[jobId]?.name ?? key) : key
	}

	let steps: StepInfo[] = $derived.by(() => {
		const result: StepInfo[] = []

		for (const [key, value] of Object.entries(completedSteps)) {
			const jobId = jobIdsByKey[key]
			result.push({ key, name: stepName(key, jobId), status: 'completed', jobId, result: value })
		}

		if (pendingSteps) {
			for (const key of pendingSteps.keys ?? []) {
				if (key in completedSteps) continue
				const jobId = jobIdsByKey[key]
				const isDone = jobId && childJobs[jobId]?.duration_ms != null
				result.push({
					key,
					name: stepName(key, jobId),
					status: isDone ? 'completed' : 'running',
					jobId,
					parallelGroup: pendingSteps.mode === 'parallel' ? 1 : undefined
				})
			}
		}

		return result
	})

	interface StepRound {
		mode: 'sequential' | 'parallel'
		steps: StepInfo[]
	}

	let rounds: StepRound[] = $derived.by(() => {
		if (steps.length === 0) return []

		const result: StepRound[] = []
		let currentSeq: StepInfo[] = []

		for (const step of steps) {
			if (step.parallelGroup != null) {
				if (currentSeq.length > 0) {
					result.push({ mode: 'sequential', steps: currentSeq })
					currentSeq = []
				}
				const lastRound = result[result.length - 1]
				if (lastRound?.mode === 'parallel') {
					lastRound.steps.push(step)
				} else {
					result.push({ mode: 'parallel', steps: [step] })
				}
			} else {
				currentSeq.push(step)
			}
		}

		if (currentSeq.length > 0) {
			result.push({ mode: 'sequential', steps: currentSeq })
		}

		return result
	})

	let hasRuntimeData = $derived(
		Object.keys(completedSteps).length > 0 ||
			pendingSteps != null ||
			Object.keys(childJobs).length > 0
	)

	function stepClasses(status: string): string {
		switch (status) {
			case 'completed':
				return 'bg-green-50 dark:bg-green-900/20 border-green-300 dark:border-green-700'
			case 'running':
				return 'bg-blue-50 dark:bg-blue-900/20 border-blue-300 dark:border-blue-700'
			case 'failed':
				return 'bg-red-50 dark:bg-red-900/20 border-red-300 dark:border-red-700'
			default:
				return 'bg-surface-secondary'
		}
	}
</script>

<div class="h-full w-full overflow-auto p-4">
	{#if hasRuntimeData}
		<div class="flex flex-col gap-3">
			{#each rounds as round, ri (ri)}
				{#if round.mode === 'parallel'}
					<div class="flex items-center gap-1 text-xs text-secondary">
						<GitBranch size={14} />
						<span>Parallel</span>
					</div>
					<div class="flex gap-3 flex-wrap">
						{#each round.steps as step (step.key)}
							{@render stepNode(step)}
						{/each}
					</div>
				{:else}
					{#each round.steps as step (step.key)}
						{@render stepNode(step)}
					{/each}
				{/if}
				{#if ri < rounds.length - 1}
					<div class="flex justify-center">
						<div class="w-px h-4 bg-border"></div>
					</div>
				{/if}
			{/each}

			{#if !flowDone && pendingSteps == null && Object.keys(completedSteps).length > 0}
				<div class="flex items-center gap-2 text-xs text-secondary">
					<Loader2 size={14} class="animate-spin" />
					<span>Replaying checkpoint...</span>
				</div>
			{/if}
		</div>
	{:else if wacResult && !wacResult.error}
		<div class="flex flex-col gap-2">
			<div class="text-xs text-secondary mb-2">Workflow structure (from code)</div>
			{#each wacResult.modules as mod (mod.id)}
				<div class="border rounded-md px-3 py-2 text-sm flex items-center gap-2 bg-surface-secondary">
					<Circle size={14} class="text-tertiary" />
					<span>{mod.summary || mod.id}</span>
				</div>
			{/each}
		</div>
	{:else}
		<div class="text-sm text-secondary p-4">Run the workflow to see execution graph.</div>
	{/if}
</div>

{#snippet stepNode(step: StepInfo)}
	<div class="border rounded-md px-3 py-2 text-sm flex items-center gap-2 min-w-[160px] {stepClasses(step.status)}">
		{#if step.status === 'completed'}
			<CheckCircle2 size={14} class="text-green-600 dark:text-green-400 shrink-0" />
		{:else if step.status === 'running'}
			<Loader2 size={14} class="animate-spin text-blue-600 dark:text-blue-400 shrink-0" />
		{:else if step.status === 'failed'}
			<XCircle size={14} class="text-red-600 dark:text-red-400 shrink-0" />
		{:else}
			<Circle size={14} class="text-tertiary shrink-0" />
		{/if}
		<span class="truncate">{step.name}</span>
		{#if step.jobId}
			<Popover notClickable>
				<a
					href="{base}/run/{step.jobId}"
					target="_blank"
					class="ml-auto text-secondary hover:text-primary shrink-0"
				>
					<ExternalLink size={12} />
				</a>
				{#snippet text()}View child job{/snippet}
			</Popover>
		{/if}
	</div>
{/snippet}
