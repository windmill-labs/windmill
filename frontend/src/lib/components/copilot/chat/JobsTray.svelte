<script lang="ts">
	import { Button } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import JobStatusIcon from '$lib/components/runs/JobStatusIcon.svelte'
	import FlowStatusWaitingForEvents from '$lib/components/FlowStatusWaitingForEvents.svelte'
	import { ChevronRight, ExternalLink, Hourglass, ThumbsUp, TimerOff } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { slide } from 'svelte/transition'
	import { JobService, type Job } from '$lib/gen'
	import { msToReadableTime } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { getAiChatManager } from './aiChatManagerContext'
	import { AIMode } from './AIChatManager.svelte'
	import { deriveChatJobStatus, type ChatJob, type ChatJobStatus } from './shared'

	// Docked just above the chat input (see AIChatDisplay). Surfaces jobs the chat
	// started that detached into the background — their live status (rendered with
	// the same JobStatusIcon the runs page uses), elapsed time, approval, cancel,
	// and open-in-preview. Global/sessions chat only (mode === GLOBAL).
	const aiChatManager = getAiChatManager()

	// Collapsed by default — the tray shows only the "Jobs" header + count badges;
	// the user opens it to see the rows.
	let expanded = $state(false)

	const jobs = $derived(aiChatManager.backgroundJobs)
	const runningCount = $derived(jobs.filter((j) => j.status === 'running').length)
	const approvalCount = $derived(jobs.filter((j) => j.status === 'suspended').length)
	const queuedCount = $derived(jobs.filter((j) => j.status === 'queued').length)
	const hasLive = $derived(jobs.some((j) => !isTerminal(j.status)))

	// Show the most recent jobs first, capped to a page; "Show more" reveals older
	// ones in batches so a long-running session doesn't grow an endless list.
	const PAGE_SIZE = 5
	let visibleCount = $state(PAGE_SIZE)
	const sortedJobs = $derived([...jobs].sort((a, b) => b.createdAt - a.createdAt))
	const visibleJobs = $derived(sortedJobs.slice(0, visibleCount))
	const hiddenCount = $derived(Math.max(0, sortedJobs.length - visibleCount))

	// A 1s clock for elapsed times, running only while a job is live so the tray
	// isn't holding a timer once everything has settled.
	let now = $state(Date.now())
	$effect(() => {
		if (!hasLive) return
		const t = setInterval(() => (now = Date.now()), 1000)
		return () => clearInterval(t)
	})

	function isTerminal(status: ChatJobStatus): boolean {
		return status === 'success' || status === 'failure' || status === 'canceled'
	}

	function elapsedLabel(job: ChatJob): string {
		// A scheduled run's "elapsed since created" is meaningless — suppress it.
		if (job.status === 'scheduled') return ''
		if (isTerminal(job.status)) {
			return job.durationMs !== undefined ? msToReadableTime(job.durationMs, 2) : ''
		}
		return msToReadableTime(now - job.createdAt, 2)
	}

	function runHref(job: ChatJob): string {
		return `${base}/run/${job.jobId}?workspace=${job.workspace}`
	}

	function openRun(job: ChatJob) {
		// In /sessions the run opens in the preview pane; the global side-panel chat
		// has no runtime, so fall back to a new browser tab.
		if (aiChatManager.openRunInPreview) {
			aiChatManager.openRunInPreview({
				jobId: job.jobId,
				workspace: job.workspace,
				label: job.label
			})
		} else {
			window.open(runHref(job), '_blank', 'noreferrer')
		}
	}

	// --- Approval modal (reuses the run page's FlowStatusWaitingForEvents) ---
	let approvalOpen = $state(false)
	let approvalLabel = $state('')
	let approvalJob = $state<Job | undefined>(undefined)

	async function openApproval(job: ChatJob) {
		approvalLabel = job.label
		approvalJob = undefined
		approvalOpen = true
		try {
			// The tray's stored job snapshot is trimmed; the approval form needs the
			// full job (flow_status + raw_flow), so fetch it fresh.
			const full = await JobService.getJob({
				workspace: job.workspace,
				id: job.jobId,
				noLogs: true,
				noCode: true
			})
			if (deriveChatJobStatus(full) === 'suspended') {
				approvalJob = full
			} else {
				approvalOpen = false
				sendUserToast('This job is no longer waiting for approval.')
			}
		} catch (e) {
			approvalOpen = false
			console.error('Failed to load approval details', e)
			sendUserToast('Failed to load approval details', true)
		}
	}

	// When the modal closes, drop the loaded job and force an immediate poll so the
	// tray reflects the approve/reject without waiting for the next poll tick.
	$effect(() => {
		if (!approvalOpen && approvalJob) {
			approvalJob = undefined
			aiChatManager.refreshBackgroundJobs()
		}
	})
</script>

{#if aiChatManager.mode === AIMode.GLOBAL && jobs.length > 0}
	<div class="mb-1 rounded-md border bg-surface-tertiary text-xs">
		<button
			type="button"
			class="flex w-full items-center gap-2 px-3 py-2"
			onclick={() => (expanded = !expanded)}
		>
			<ChevronRight
				size={14}
				class={`text-tertiary transition-transform duration-150 ${expanded ? 'rotate-90' : ''}`}
			/>
			<span class="text-xs font-normal text-primary">Jobs</span>
			<span class="ml-auto flex items-center gap-1">
				{#if runningCount > 0}<Badge color="yellow" small>{runningCount} running</Badge>{/if}
				{#if approvalCount > 0}<Badge color="violet" small>{approvalCount} approval</Badge>{/if}
				{#if queuedCount > 0}<Badge color="orange" small>{queuedCount} queued</Badge>{/if}
			</span>
		</button>

		{#if expanded}
			<div transition:slide={{ duration: 150 }} class="border-t py-1">
				{#each visibleJobs as job (job.jobId)}
					<div class="group flex items-center gap-2.5 px-3 py-2">
						{#if job.status === 'queued' || !job.job}
							<!-- Queued: match the job detail page's orange badge (JobStatus.svelte).
							     JobStatusIcon's default queued badge is gray, so render it here
							     instead. This also covers the pre-first-fetch state (no job.job). -->
							<Badge color="orange" baseClass="!px-1.5" title="Queued"
								><Hourglass size={14} /></Badge
							>
						{:else}
							<JobStatusIcon job={job.job} />
						{/if}
						<span class="min-w-0 grow truncate text-secondary" title={job.label}>{job.label}</span>
						<span class="shrink-0 tabular-nums text-tertiary">{elapsedLabel(job)}</span>
						<div class="flex shrink-0 items-center gap-1.5">
							{#if job.status === 'suspended'}
								<Button
									unifiedSize="xs"
									variant="accent"
									startIcon={{ icon: ThumbsUp }}
									on:click={() => openApproval(job)}>Approve</Button
								>
							{/if}
							{#if !isTerminal(job.status)}
								<Button
									unifiedSize="xs"
									variant="accent"
									destructive
									startIcon={{ icon: TimerOff }}
									on:click={() => aiChatManager.cancelJob(job.jobId)}>Cancel</Button
								>
							{/if}
							<button
								type="button"
								class="text-tertiary hover:text-primary"
								title="Open the run"
								onclick={() => openRun(job)}
							>
								<ExternalLink size={13} />
							</button>
						</div>
					</div>
				{/each}
				{#if hiddenCount > 0}
					<button
						type="button"
						class="flex w-full items-center justify-center border-t px-3 py-1.5 text-tertiary hover:text-primary"
						onclick={() => (visibleCount += PAGE_SIZE)}
					>
						Show more ({hiddenCount})
					</button>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Portal to <body> + a z-index above the editor: opened from deep in the
	     sessions chat column, the modal would otherwise be trapped below the
	     preview-pane flow editor (Monaco at zIndexes.monacoEditor). -->
	<Portal target="body">
		<Modal
			title={`Approval — ${approvalLabel}`}
			bind:open={approvalOpen}
			minZIndex={zIndexes.monacoEditorSuggestions + 1}
		>
			{#if approvalJob}
				<FlowStatusWaitingForEvents
					job={approvalJob}
					workspaceId={approvalJob.workspace_id}
					isOwner={true}
					light
				/>
			{:else}
				<div class="p-6 text-center text-xs text-tertiary">Loading approval…</div>
			{/if}
		</Modal>
	</Portal>
{/if}
