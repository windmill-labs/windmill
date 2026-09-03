<script lang="ts">
	import { onMount } from 'svelte'
	import { cubicOut } from 'svelte/easing'
	import { prefersReducedMotion } from 'svelte/motion'
	import { Ban, Loader2, TimerOff } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Button, Tab, Tabs } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { msToReadableTime } from '$lib/utils'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import { base } from '$lib/base'
	import { getAiChatManager } from './aiChatManagerContext'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import RunArgsFormDisplay from './RunArgsFormDisplay.svelte'
	import ToolContentDisplay from './ToolContentDisplay.svelte'
	import ToolPreviewCard from './ToolPreviewCard.svelte'
	import { scrollFades } from './scrollFades.svelte'
	import { isActiveRunForm, MAX_LOG_LENGTH, type ToolDisplayMessage } from './shared'

	const aiChatManager = getAiChatManager()

	interface Props {
		message: ToolDisplayMessage
	}

	let { message }: Props = $props()

	const runForm = $derived(message.runForm!)
	// The loop is parked on the form and nothing has run yet: the card is the form.
	const pending = $derived(isActiveRunForm(message))

	const chatJob = $derived(
		aiChatManager.backgroundJobs.find((j) => j.toolCallId === message.tool_call_id)
	)
	// Declining the form, stopping the turn and cancelling the job all land here, and none
	// of them is a failure: the run stopped because someone said so.
	const canceled = $derived(
		Boolean(message.declinedByUser) || Boolean(runForm.canceled) || chatJob?.status === 'canceled'
	)
	const failed = $derived(Boolean(message.error) && !canceled)
	// A cancelled form never reached a job, so it has no logs and no outcome to offer.
	const ran = $derived(
		Boolean(runForm.started) || Boolean(message.logs) || message.result !== undefined || !!chatJob
	)
	// A run can outlive the turn that started it, so "the tool call returned" is not
	// "the run finished": a detached job keeps the card in its running state until
	// the background poller lands an outcome on it or the tray sees the job end.
	const settled = $derived(
		!pending &&
			!message.isLoading &&
			(message.result !== undefined ||
				failed ||
				canceled ||
				(chatJob !== undefined && ['success', 'failure', 'canceled'].includes(chatJob.status)))
	)
	const running = $derived(!pending && !settled)

	const parameters = $derived(
		message.parameters && typeof message.parameters === 'object' ? message.parameters : {}
	)
	const logs = $derived(typeof message.logs === 'string' ? message.logs : '')
	const logLineCount = $derived(logs.trim() ? logs.trimEnd().split('\n').length : 0)
	// What the job has streamed of its result so far. Only ever set while it runs: the
	// terminal patch clears it, so a settled card reads its outcome off `result` alone.
	const resultStream = $derived(
		typeof message.resultStream === 'string' ? message.resultStream : ''
	)
	const streaming = $derived(running && resultStream.length > 0)

	// The card stores its result as text (see formatResult), so read it back into a
	// value DisplayResult can render: a markdown, table or image result is what the
	// pretty view buys. A string that happens to be JSON parses back as JSON, and the
	// text it was stored as is one toggle away in the raw view.
	const resultValue = $derived.by(() => {
		if (message.result === undefined) return undefined
		if (typeof message.result !== 'string') return message.result
		try {
			return JSON.parse(message.result)
		} catch {
			return message.result
		}
	})

	// The row is the card's whole heading, in the tense the call is in: a run cancelled
	// before it started never ran, so it is still the thing that was going to be run. A
	// test says so, since what it ran is the draft rather than what is deployed.
	const verbs = $derived(
		runForm.kind === 'test'
			? { present: 'Testing', past: 'Tested', future: 'Test' }
			: { present: 'Running', past: 'Ran', future: 'Run' }
	)
	// What the script is called on its own page and in the picker, so the row names the thing
	// that ran rather than where it is filed. Not every script has one, so the path stays the
	// fallback — and stays on the preview chip either way, since two folders can hold one name.
	const runnableName = $derived(runForm.summary || runForm.path)
	const verb = $derived(running ? verbs.present : settled && ran ? verbs.past : verbs.future)

	// Being cancelled is an outcome like any other, and it is the one the card has to say out
	// loud: nothing came back, so no other tab can carry it.
	const outcomeTab = $derived(
		failed ? 'Error' : canceled ? (ran ? 'Cancelled' : 'Not run') : 'Result'
	)
	const cancelReason = $derived(
		ran
			? 'This run was cancelled while the script was running.'
			: 'This run was cancelled before the script started.'
	)
	// Streaming opens the tab early: the result is already arriving, and one that appeared
	// only at the end would hide the thing the user is waiting to read.
	const hasOutcome = $derived(settled || streaming)
	const tabs = $derived([
		{ value: 'input', label: 'Inputs' },
		...(ran ? [{ value: 'logs', label: 'Logs' }] : []),
		...(hasOutcome ? [{ value: 'outcome', label: outcomeTab }] : [])
	])

	// Keyed by call id, like `toggled` below: this instance is reused when the message at its
	// index changes, and a bare flag would hand one card's raw view, or the tab its user
	// picked, to whichever run lands in the slot next.
	// Undefined until a tab is clicked, and never cleared after: the run follows itself
	// only for as long as nobody has steered the card, and then it stops taking it back.
	let userTab = $state<{ id: string; value: string } | undefined>(undefined)
	let jsonView = $state<{ id: string; on: boolean } | undefined>(undefined)
	const steered = $derived(userTab?.id === message.tool_call_id ? userTab.value : undefined)
	const rawView = $derived(jsonView?.id === message.tool_call_id ? jsonView.on : false)

	// However the run landed, that is what the card opens on.
	const autoTab = $derived(hasOutcome ? 'outcome' : ran ? 'logs' : 'input')
	const activeTab = $derived(steered && tabs.some((t) => t.value === steered) ? steered : autoTab)

	// Keyed by call id: a bare flag would carry one card's collapse onto the next message
	// reusing this instance. Open by default, since the run is what was asked for.
	let toggled = $state<{ id: string; open: boolean } | undefined>(undefined)
	const expanded = $derived(toggled?.id === message.tool_call_id ? toggled.open : true)

	// The panel mounts the chat's own form on this call, so the card must not mount a second
	// one: two views binding the one draft would each reorder the schema SchemaForm edits in
	// place. Only the form is exclusive. A run shows in both, and collapsing the row or
	// closing the tab is the user's own way out of seeing it twice.
	const formInPreview = $derived(
		pending && (aiChatManager.isRunFormInPreview?.(message.tool_call_id) ?? false)
	)

	// The three moments a run moves the card — a tab arrives, the selection follows it, the body
	// changes under it — used to land in one frame each, which is why the card was hard to follow.
	// A tab already on screen when the card mounts never arrived, so a settled call restored from
	// history renders its strip at rest.
	let liveStrip = $state(false)
	onMount(() => (liveStrip = true))

	function growIn(node: HTMLElement, { live }: { live: boolean }) {
		const width = node.getBoundingClientRect().width
		return {
			duration: live && !prefersReducedMotion.current ? 170 : 0,
			easing: cubicOut,
			css: (t: number) => `width:${t * width}px; opacity:${t}`
		}
	}

	/** The body fades in over the time the bar takes to travel, so the two halves of a tab change
	 * land together rather than one instantly and one over 200ms. No shift with it: the logs open
	 * pinned to the end of their own scroll, which clips one, and a pane that moves on two tabs
	 * out of three reads as a glitch rather than as direction. */
	function enterPane(_node: HTMLElement) {
		return {
			duration: liveStrip && !prefersReducedMotion.current ? 200 : 0,
			easing: cubicOut,
			css: (t: number) => `opacity:${t}`
		}
	}

	let bodyEl: HTMLDivElement | undefined = $state()

	const fades = scrollFades()
	// The arguments table brings its own surface, so a fade ending in the card's would seam
	// against it; logs and a result stand on the body's own ground and take it cleanly.
	const fadeBody = $derived(!rawView && (activeTab === 'logs' || activeTab === 'outcome'))

	// One scroll region serves every tab, so a switch has to place it: logs open on their
	// end, which is where a run is read from, and everything else on its start — otherwise
	// the tab opened after the logs would begin part-way down its own content.
	$effect(() => {
		void rawView
		if (!bodyEl) return
		bodyEl.scrollTop = activeTab === 'logs' && !rawView ? bodyEl.scrollHeight : 0
	})

	// And stay on the end while the job writes: a log stream the user has to scroll to
	// read is not following the run.
	$effect(() => {
		void logs
		if (!bodyEl || rawView || activeTab !== 'logs' || !running) return
		bodyEl.scrollTop = bodyEl.scrollHeight
	})

	// Ticks only while this card has a job in flight, so a transcript of settled runs
	// keeps no timers at all.
	let now = $state(Date.now())
	$effect(() => {
		if (!running || !chatJob) return
		const timer = setInterval(() => (now = Date.now()), 500)
		return () => clearInterval(timer)
	})

	const elapsed = $derived(chatJob ? msToReadableTime(now - chatJob.createdAt, 2) : '')
	const duration = $derived(
		chatJob?.durationMs !== undefined ? msToReadableTime(chatJob.durationMs, 2) : ''
	)

	// The hue of the status badge the jobs bar shows for the same job — blue running, violet
	// approval, orange queued, green ok, red fail — kept under the name it annotates, since the
	// runnable is the subject of the row and this is metadata about it. Not the badge's ink:
	// drawn for a tinted ground, it lands brighter than the name on this transparent one. The
	// step differs per hue because the ramp is not uniform, and ok borrows emerald because the
	// green one skips straight past that weight.
	const statusClass = $derived.by(() => {
		// The card outlives its job and sometimes precedes it, so the states only it knows about
		// read off its own flags rather than off a status no job is there to report.
		if (canceled) return 'text-tertiary'
		if (failed) return 'text-red-800 dark:text-red-300'
		if (!ran) return 'text-tertiary'
		switch (chatJob?.status) {
			case 'running':
				return 'text-blue-800 dark:text-blue-200'
			case 'suspended':
				return 'text-violet-800 dark:text-violet-300'
			case 'queued':
			case 'scheduled':
				return 'text-orange-800 dark:text-orange-300'
			case 'failure':
				return 'text-red-800 dark:text-red-300'
			case 'success':
				return 'text-green-700 dark:text-emerald-400'
			default:
				return settled ? 'text-green-700 dark:text-emerald-400' : 'text-blue-800 dark:text-blue-200'
		}
	})
	// How long it took, which is the one thing the colour cannot say. A run that never started
	// has no time to give, so its outcome takes the slot — as a word, never "Not run", which
	// stutters against the "Run <name>" label beside it.
	const outcome = $derived(failed ? 'Failed' : canceled ? 'Cancelled' : 'Done')
	const statusTime = $derived(running ? elapsed : duration || outcome)

	// What the preview button opens changes with the card: the form while the call is still
	// waiting on one, the run once a job exists. Neither, and there is nothing to open, so
	// the button is not drawn at all — a form has nowhere to go outside a session, and a
	// call cancelled before Run never became a run.
	const previewTarget = $derived(
		pending
			? aiChatManager.openRunForm
				? ('form' as const)
				: undefined
			: chatJob
				? ('run' as const)
				: undefined
	)
	const previewTitle = $derived(
		previewTarget === 'form'
			? `Open this form in the preview panel: ${runForm.path}`
			: aiChatManager.openRunInPreview
				? `Open this run in the preview panel: ${runForm.path}`
				: `Open this run in a new tab: ${runForm.path}`
	)

	function openPreview() {
		const label = runnableName
		if (previewTarget === 'form') {
			aiChatManager.openRunForm?.({ toolCallId: message.tool_call_id, label })
			return
		}
		if (!chatJob) return
		// Outside a session there is no panel, so the run opens where the jobs tray sends it.
		if (aiChatManager.openRunInPreview) {
			aiChatManager.openRunInPreview({ jobId: chatJob.jobId, workspace: chatJob.workspace, label })
		} else {
			window.open(
				`${base}/run/${chatJob.jobId}?workspace=${chatJob.workspace}`,
				'_blank',
				'noreferrer'
			)
		}
	}
</script>

<!-- One readout rather than a badge beside a number: the colour says how the run went and the
     text how long it took, which is how the rest of the chat states a status. While it runs
     that number is still moving. `font-medium` because the row is a button and the base layer
     sets those semibold, which would leave this the one bold word in the header. -->
{#snippet status()}
	{#if !pending}
		<span class={twMerge('shrink-0 whitespace-nowrap text-2xs font-medium', statusClass)}>
			{statusTime}
		</span>
	{/if}
{/snippet}

<!-- The chip every other tool row opens its preview with, pointed at this call: the form
     on its way in, the run on its way out. Not a toggle — pressing it again focuses the
     tab it already opened. The row's only control, as on every other tool call. -->
{#snippet previewChip()}
	<ToolPreviewCard
		card={{ kind: 'script', path: runForm.path }}
		title={previewTitle}
		onOpen={openPreview}
		kindIcon={false}
	/>
{/snippet}

<!-- scroll-mb clears the chat's sticky "Waiting for your input" chip so the mount
     scrollIntoView of the form below leaves the Run button uncovered. -->
<!-- The runnable's name is the subject of the row and the verb is grammar, so the name takes
     the weight the selected tab has while the verb keeps the row's ordinary one. font-main
     because a name is UI text: the row around it is font-mono, right for a path, wrong here. -->
<ChatCollapsibleCard
	label={runnableName}
	labelPrefix={verb}
	{expanded}
	onToggle={() => (toggled = { id: message.tool_call_id, open: !expanded })}
	headerLeft={status}
	headerRight={previewTarget ? previewChip : undefined}
	class="scroll-mb-8"
	labelClass="font-main font-semibold text-emphasis"
	contentClass="p-0 overflow-hidden"
>
	{#if formInPreview}
		<div class="px-3 py-2 text-2xs leading-4 text-hint">
			These inputs are open in the preview panel.
		</div>
	{:else if pending}
		<RunArgsFormDisplay toolCallId={message.tool_call_id} {runForm} />
	{:else}
		<!-- One region holding the strip and the body, fixed so the card is the same size on every
		     tab — a cap would not do it, since the body is a scroll region and a max-height
		     silently beats flex-grow. The raw view takes that height as a floor instead: its
		     blocks scroll on their own, as an ordinary tool call's do, so a scroller around them
		     would be one too many. -->
		<div class={twMerge('relative flex flex-col', rawView ? 'min-h-[20rem]' : 'h-[20rem]')}>
			<!-- The tabs go in raw view — they name the parts of the body, and the raw call is not
			     one of them — while the strip stays, since the JSON toggle lives there. Hence its
			     fixed height: a row sized by its contents would step every time the tabs leave, and
			     the tighter Tab padding below is what fits a label inside that height. -->
			<Tabs
				selected={activeTab}
				on:selected={(e) => (userTab = { id: message.tool_call_id, value: e.detail })}
				class="h-8 px-3 font-main"
				wrapperClass="shrink-0"
				slidingIndicator
			>
				{#if !rawView}
					{#each tabs as tab (tab.value)}
						<!-- The tab widens in first and the bar follows it, because a run adds its tabs as
						     it produces them: landing the selection on a tab in the frame it appears reads
						     as one unexplained jump. `border-b-0` because the bar is the selection now.
						     Size only: Tab's own colour and weight mark the selection, and this class
						     lands after them in its twMerge, so a colour here would silently win. -->
						<span class="inline-flex overflow-hidden" in:growIn={{ live: liveStrip }}>
							<Tab
								value={tab.value}
								label={tab.label}
								class="border-b-0 py-0.5 text-2xs leading-4"
								exact
							>
								{#snippet extra()}
									{#if tab.value === 'logs' && logLineCount > 0}
										<span class="text-2xs text-hint">{logLineCount}</span>
									{/if}
								{/snippet}
							</Tab>
						</span>
					{/each}
				{/if}
				<div class="ml-auto flex items-center pl-2">
					<Toggle
						checked={rawView}
						on:change={(e) => (jsonView = { id: message.tool_call_id, on: e.detail })}
						size="2xs"
						options={{ right: 'JSON', rightTooltip: 'Show this call as raw JSON' }}
						lightMode
					/>
				</div>
			</Tabs>

			<!-- Logs sit on the softer surface, the way program output is shown everywhere else.
			     The raw view keeps the card's own, as an ordinary tool call has it. -->
			<div
				bind:this={bodyEl}
				use:fades.container
				onscroll={fades.measure}
				class={twMerge(
					'min-h-0 flex-1 px-3 py-2',
					rawView ? '' : 'overflow-auto',
					!rawView && activeTab === 'logs' ? 'bg-surface-secondary/50' : ''
				)}
			>
				<!-- min-h-full rather than h-full: the states that centre themselves need the height,
				     and a box that always filled it would measure as never scrollable. -->
				<div use:fades.content class="flex min-h-full flex-col">
					{#if rawView}
						<div class="space-y-3">
							<!-- Each block scrolls on its own, so each fades on its own. -->
							<ToolContentDisplay title="Parameters" content={message.parameters} showFade />
							<ToolContentDisplay title="Logs" content={message.logs} tail showFade />
							<ToolContentDisplay
								title="Result"
								content={message.result}
								error={message.error}
								showFade
							/>
						</div>
					{:else}
						<!-- Keyed on the tab so the body arrives rather than cuts. One region serves every
						     tab, so only the incoming pane moves: overlapping them would ask this scroller
						     to hold two at once. -->
						{#key activeTab}
							<div class="flex min-h-full flex-1 flex-col" in:enterPane>
								{#if activeTab === 'input'}
									<!-- What the runs page shows a finished job's arguments as: the operator has already
					     read this table. The job id is what lets it fetch arguments too big to have been
					     persisted with the card. -->
									<JobArgs
										args={parameters}
										id={chatJob?.jobId}
										workspace={chatJob?.workspace}
										disableExpand
									/>
								{:else if activeTab === 'logs'}
									{#if logs.trim()}
										{#if logs.length >= MAX_LOG_LENGTH}
											<p class="mb-1 text-2xs text-tertiary">
												Tail of the logs, the last {MAX_LOG_LENGTH} characters.
											</p>
										{/if}
										<pre class="whitespace-pre-wrap break-words font-mono text-2xs text-primary"
											>{logs}</pre
										>
									{:else}
										<p class="text-2xs text-tertiary">No logs yet.</p>
									{/if}
									{#if running}
										<div class="mt-1 flex items-center gap-1.5 text-2xs text-tertiary">
											<Loader2 class="h-3 w-3 animate-spin" />
											streaming
										</div>
									{/if}
								{:else if failed}
									<pre
										class="whitespace-pre-wrap break-words font-mono text-2xs text-red-700 dark:text-red-300"
										>{message.error}</pre
									>
								{:else if streaming}
									<!-- The same renderer as a landed result, handed the partial: it is the one that
					     knows how to show a result arriving in pieces. -->
									<DisplayResult
										result={undefined}
										result_stream={resultStream}
										jobId={chatJob?.jobId}
										workspaceId={chatJob?.workspace}
										disableExpand
										hideAsJson
									/>
								{:else if resultValue !== undefined}
									<!-- The run page's own renderer, not a second one invented for the chat: it handles
					     markdown, tables, images and deep nesting without the card guessing at the
					     shape, and reaches the job through jobId/workspace for an S3 preview.
					     `disableExpand` drops its toolbar and `hideAsJson` its Pretty/JSON switch,
					     which the row already owns. -->
									<DisplayResult
										result={resultValue}
										jobId={chatJob?.jobId}
										workspaceId={chatJob?.workspace}
										disableExpand
										hideAsJson
									/>
								{:else if canceled}
									<!-- All that is left to render is the fact itself: a form cancelled before Run
						     never reached a job, so there is no result the way a cancelled run has one. -->
									<div
										class="flex flex-1 flex-col items-center justify-center gap-1.5 px-4 text-center"
									>
										<Ban class="h-4 w-4 text-tertiary" />
										<p class="text-2xs font-medium leading-4 text-secondary">{cancelReason}</p>
										{#if !ran}
											<p class="text-2xs leading-4 text-tertiary">
												The inputs it would have run with are on the Inputs tab.
											</p>
										{/if}
									</div>
								{:else}
									<p class="text-2xs text-tertiary">This run returned no result.</p>
								{/if}
							</div>
						{/key}
					{/if}
				</div>
			</div>

			<!-- Over the body, not inside it: what is still below fades out, and nothing at the top,
			     as on the form and on the tool cards. Two layers on the logs, whose ground is the
			     card's surface with the softer one at half strength over it — one gradient would end
			     on the wrong colour and leave a band at the very edge. -->
			{#if fades.bottom && fadeBody}
				<div
					class="pointer-events-none absolute inset-x-0 bottom-0 h-[min(2.5rem,35%)] bg-gradient-to-t from-surface via-surface/60 to-transparent"
				></div>
				{#if activeTab === 'logs'}
					<div
						class="pointer-events-none absolute inset-x-0 bottom-0 h-[min(2.5rem,35%)] bg-gradient-to-t from-surface-secondary/50 via-surface-secondary/30 to-transparent"
					></div>
				{/if}
			{/if}
		</div>

		{#if running && chatJob}
			<!-- Where the form keeps its own actions, so the button that stops a run and the one
			     that starts it sit in the same corner of the same card. The run page's own cancel
			     button, down to the icon: the operator has already pressed this one. -->
			<div class="flex justify-end border-t border-border-light px-3 py-2">
				<Button
					variant="accent"
					unifiedSize="sm"
					destructive
					startIcon={{ icon: TimerOff }}
					title="Cancel the script"
					onClick={() => aiChatManager.cancelJob(chatJob.jobId)}
				>
					Cancel
				</Button>
			</div>
		{/if}
	{/if}
</ChatCollapsibleCard>
