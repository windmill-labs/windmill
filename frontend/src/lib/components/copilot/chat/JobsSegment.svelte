<script lang="ts">
	import { Button } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import JobStatusIcon from '$lib/components/runs/JobStatusIcon.svelte'
	import FlowStatusWaitingForEvents from '$lib/components/FlowStatusWaitingForEvents.svelte'
	import { ChevronUp, Hourglass, ThumbsUp, TimerOff } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { slide } from 'svelte/transition'
	import { JobService, type Job } from '$lib/gen'
	import { msToReadableTime } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { getAiChatManager } from './aiChatManagerContext'
	import { deriveChatJobStatus, type ChatJob, type ChatJobStatus } from './shared'
	import { TOKEN_TRIGGER_CLASS } from '$lib/components/sessions/SessionStatusToken.svelte'

	// The "Jobs" segment of the session bar: a compact status chip that summarizes
	// the background jobs the chat started, opening a popover with the full list
	// (live status via the runs-page JobStatusIcon, approval, cancel, open-in-run).
	// Renders as a standalone slim bar in the global side-panel chat, and as the
	// right segment inside SessionChangesBar's session bar (standalone={false}).
	let { standalone = false }: { standalone?: boolean } = $props()

	const aiChatManager = getAiChatManager()
	const jobs = $derived(aiChatManager.backgroundJobs)

	const runningCount = $derived(jobs.filter((j) => j.status === 'running').length)
	const approvalCount = $derived(jobs.filter((j) => j.status === 'suspended').length)
	const queuedCount = $derived(
		jobs.filter((j) => j.status === 'queued' || j.status === 'scheduled').length
	)
	const failureCount = $derived(jobs.filter((j) => j.status === 'failure').length)
	const successCount = $derived(jobs.filter((j) => j.status === 'success').length)
	const liveCount = $derived(jobs.filter((j) => !isTerminal(j.status)).length)
	const hasLive = $derived(liveCount > 0)

	function isTerminal(status: ChatJobStatus): boolean {
		return status === 'success' || status === 'failure' || status === 'canceled'
	}

	// bg color for a status dot, keyed off the same vocabulary as the runs page:
	// blue running, violet approval, orange queued/scheduled, green ok, red fail.
	function dotClass(status: ChatJobStatus): string {
		switch (status) {
			case 'running':
				return 'bg-blue-500'
			case 'suspended':
				return 'bg-violet-500'
			case 'queued':
			case 'scheduled':
				return 'bg-orange-500'
			case 'success':
				return 'bg-green-500'
			case 'failure':
				return 'bg-red-500'
			default:
				return 'bg-gray-400'
		}
	}

	// The single discriminating segment of a path ("u/admin/wait" → "wait"); the
	// full label stays in the popover row + the chip's title.
	function shortName(label: string): string {
		const parts = label.split('/')
		return parts[parts.length - 1] || label
	}

	// Aggregate chip readout, priority-ordered so the most action-worthy state
	// wins the dot: approval > running > queued > failed > succeeded. A live run
	// takes the dot even if an earlier job failed (failure resurfaces once idle).
	const segment = $derived.by(
		(): { dot: string; pulse: boolean; text: string; danger: boolean } => {
			if (approvalCount > 0)
				return {
					dot: dotClass('suspended'),
					pulse: true,
					text: `${approvalCount} approval`,
					danger: false
				}
			if (runningCount > 0) {
				if (runningCount === 1 && liveCount === 1) {
					const job = jobs.find((j) => j.status === 'running')
					return {
						dot: dotClass('running'),
						pulse: true,
						text: job ? shortName(job.label) : '1 running',
						danger: false
					}
				}
				return {
					dot: dotClass('running'),
					pulse: true,
					text: `${runningCount} running`,
					danger: false
				}
			}
			if (queuedCount > 0)
				return {
					dot: dotClass('queued'),
					pulse: false,
					text: `${queuedCount} queued`,
					danger: false
				}
			if (failureCount > 0)
				return {
					dot: dotClass('failure'),
					pulse: false,
					text: `${failureCount} failed`,
					danger: true
				}
			// All terminal, none failed: green if anything actually succeeded, else gray
			// (only canceled jobs left — a cancel isn't a success, so don't show green).
			if (successCount > 0)
				return {
					dot: dotClass('success'),
					pulse: false,
					text: `${successCount} succeeded`,
					danger: false
				}
			return {
				dot: dotClass('canceled'),
				pulse: false,
				text: `${jobs.length} canceled`,
				danger: false
			}
		}
	)

	// Elapsed label for the running-single chip; a 1s clock, live only.
	let now = $state(Date.now())
	$effect(() => {
		if (!hasLive) return
		const t = setInterval(() => (now = Date.now()), 1000)
		return () => clearInterval(t)
	})

	// Screen-reader announcement when a background job finishes — once per job (on
	// its terminal transition), never per elapsed tick (the clock above updates
	// `now`, not `jobs`, so this effect doesn't fire for it). Seed the seen-set on
	// the first run so pre-existing terminal jobs aren't announced on mount.
	let announcement = $state('')
	const announcedTerminal = new Set<string>()
	let announceInitialized = false
	$effect(() => {
		const terminal = jobs.filter((j) => isTerminal(j.status))
		if (!announceInitialized) {
			for (const j of terminal) announcedTerminal.add(j.jobId)
			announceInitialized = true
			return
		}
		// Announce every job that became terminal this tick in one utterance —
		// setting `announcement` per job would drop all but the last when several
		// finish together.
		const newlyDone = terminal.filter((j) => !announcedTerminal.has(j.jobId))
		for (const j of newlyDone) announcedTerminal.add(j.jobId)
		if (newlyDone.length > 0) {
			const parts = newlyDone.map((j) => {
				const verb =
					j.status === 'success' ? 'succeeded' : j.status === 'failure' ? 'failed' : 'was canceled'
				return `${shortName(j.label)} ${verb}`
			})
			announcement = `Background job${newlyDone.length > 1 ? 's' : ''}: ${parts.join('; ')}.`
		}
	})
	const runningJob = $derived(
		runningCount === 1 && liveCount === 1 ? jobs.find((j) => j.status === 'running') : undefined
	)
	const chipElapsed = $derived(runningJob ? msToReadableTime(now - runningJob.createdAt, 2) : '')

	// Popover rows: live jobs pinned to the top, then newest first.
	const sortedJobs = $derived(
		[...jobs].sort((a, b) => {
			const al = isTerminal(a.status) ? 1 : 0
			const bl = isTerminal(b.status) ? 1 : 0
			if (al !== bl) return al - bl
			return b.createdAt - a.createdAt
		})
	)

	function elapsedLabel(job: ChatJob): string {
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
		open = false
	}

	// --- Popover open state + auto-open on approval ---
	let popover: Popover | undefined = $state()
	let open = $state(false)

	// A job entering the approval state needs attention, so open the popover to
	// surface its Approve action. Rising-edge only (a newly suspended job), so the
	// user can still dismiss it while the approval stays pending.
	let prevApprovalCount = 0
	$effect(() => {
		const count = approvalCount
		if (count > prevApprovalCount) popover?.open()
		prevApprovalCount = count
	})

	// --- Approval modal (reuses the run page's FlowStatusWaitingForEvents) ---
	let approvalOpen = $state(false)
	let approvalLabel = $state('')
	let approvalJob = $state<Job | undefined>(undefined)

	async function openApproval(job: ChatJob) {
		approvalLabel = job.label
		approvalJob = undefined
		approvalOpen = true
		try {
			// The stored snapshot is trimmed; the approval form needs the full job
			// (flow_status + raw_flow), so fetch it fresh.
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
	// segment reflects the approve/reject without waiting for the next poll tick.
	$effect(() => {
		if (!approvalOpen && approvalJob) {
			approvalJob = undefined
			aiChatManager.refreshBackgroundJobs()
		}
	})

	const ariaLabel = $derived(
		hasLive
			? `Jobs, ${segment.text}${chipElapsed ? `, ${chipElapsed}` : ''}`
			: `Jobs, ${jobs.length} finished${failureCount > 0 ? `, ${failureCount} failed` : ''}`
	)
</script>

{#if jobs.length > 0}
	<!-- Live region so a background-job completion is announced to screen readers
	     even when the chip's visual change alone wouldn't be. role="status" already
	     implies aria-live="polite". -->
	<div class="sr-only" role="status">{announcement}</div>
	<Popover
		bind:this={popover}
		bind:isOpen={open}
		placement={standalone ? 'top-end' : 'top-start'}
		enableFlyTransition
		usePointerDownOutside
		closeOnOtherPopoverOpen={!standalone}
		class={standalone
			? 'flex h-[34px] w-full items-center rounded-md border bg-surface-tertiary px-3 hover:bg-surface-hover'
			: TOKEN_TRIGGER_CLASS}
		triggerAttrs={{ 'aria-label': ariaLabel, 'aria-haspopup': 'dialog' }}
		contentClasses="!bg-surface"
	>
		{#snippet trigger()}
			<span class="flex min-w-0 items-center gap-2 text-xs">
				{#if standalone}
					<span class="shrink-0 font-normal text-primary">Jobs</span>
				{/if}
				<span
					class={`size-[7px] shrink-0 rounded-full ${segment.dot} ${segment.pulse ? 'motion-safe:animate-pulse' : ''}`}
				></span>
				<span
					class={`min-w-0 truncate font-normal ${runningJob ? 'text-2xs' : 'text-xs'} ${segment.danger ? 'text-red-500' : standalone ? 'text-primary' : 'text-secondary'}`}
					title={runningJob?.label ?? undefined}
					dir={runningJob ? 'rtl' : undefined}
				>
					<!-- Running single: show the FULL path, truncated with a LEADING ellipsis
					     (dir=rtl clips the start) so the disambiguating tail stays visible;
					     <bdi> keeps the path itself rendering left-to-right. Counts render as-is. -->
					{#if runningJob}<bdi>{runningJob.label}</bdi>{:else}{segment.text}{/if}
				</span>
				{#if chipElapsed}
					<span
						transition:slide={{ axis: 'x', duration: 200 }}
						class="shrink-0 tabular-nums text-2xs text-tertiary"
					>
						{chipElapsed}
					</span>
				{/if}
				<ChevronUp
					size={14}
					class={`shrink-0 ${standalone ? 'text-secondary' : 'text-tertiary'} transition-transform duration-150 ${open ? 'rotate-180' : ''}`}
				/>
			</span>
		{/snippet}

		{#snippet content()}
			<div class="flex w-80 flex-col text-xs">
				<div class="border-b px-3 py-2 text-tertiary">Jobs this session</div>
				<div
					class={`overflow-y-auto py-1 ${standalone ? 'max-h-[50vh]' : 'max-h-[min(12rem,50vh)]'}`}
				>
					{#each sortedJobs as job (job.jobId)}
						<div class="flex items-center gap-2.5 px-3 py-1.5 hover:bg-surface-hover">
							<button
								type="button"
								class="flex min-w-0 grow items-center gap-2.5 text-left font-normal"
								title={job.label}
								onclick={() => openRun(job)}
							>
								{#if job.status === 'queued' || !job.job}
									<!-- Queued: match the job detail page's orange badge (JobStatusIcon's
									     default queued badge is gray). Also covers the pre-first-fetch state. -->
									<Badge color="orange" baseClass="!px-1.5" title="Queued"
										><Hourglass size={13} /></Badge
									>
								{:else}
									<JobStatusIcon job={job.job} />
								{/if}
								<span class="min-w-0 grow truncate text-secondary">{job.label}</span>
								<span class="shrink-0 tabular-nums text-tertiary">{elapsedLabel(job)}</span>
							</button>
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
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/snippet}
	</Popover>

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
					onAction={(approved) => {
						// Approving resumes the flow (back to running); optimistically drop the
						// suspended status so the segment updates instantly. Closing the modal
						// triggers the on-close effect above, which re-polls to reconcile.
						if (approved && approvalJob) {
							aiChatManager.updateJob(approvalJob.id, { status: 'running' })
						}
						approvalOpen = false
					}}
				/>
			{:else}
				<div class="p-6 text-center text-xs text-tertiary">Loading approval…</div>
			{/if}
		</Modal>
	</Portal>
{/if}
