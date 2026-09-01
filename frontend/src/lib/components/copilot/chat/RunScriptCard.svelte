<script lang="ts">
	import { Ban, Loader2, TimerOff } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Button, Tab, Tabs } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import JobStatusIcon from '$lib/components/runs/JobStatusIcon.svelte'
	import type { Job } from '$lib/gen'
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
	// text it was stored as is one click away under { }.
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
	// before it started never ran, so it is still the thing that was going to be run.
	const label = $derived(
		running
			? `Running ${runForm.path}`
			: settled && ran
				? `Ran ${runForm.path}`
				: `Run ${runForm.path}`
	)

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

	// Undefined until a tab is clicked, and never cleared after: the run follows itself
	// only for as long as nobody has steered the card, and then it stops taking it back.
	let userTab = $state<string | undefined>(undefined)
	let jsonView = $state(false)

	// However the run landed, that is what the card opens on.
	const autoTab = $derived(hasOutcome ? 'outcome' : ran ? 'logs' : 'input')
	const activeTab = $derived(userTab && tabs.some((t) => t.value === userTab) ? userTab : autoTab)

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

	let bodyEl: HTMLDivElement | undefined = $state()

	const fades = scrollFades()
	// The arguments table brings its own surface, so a fade ending in the card's would seam
	// against it; logs and a result stand on the body's own ground and take it cleanly.
	const fadeBody = $derived(!jsonView && (activeTab === 'logs' || activeTab === 'outcome'))

	// One scroll region serves every tab, so a switch has to place it: logs open on their
	// end, which is where a run is read from, and everything else on its start — otherwise
	// the tab opened after the logs would begin part-way down its own content.
	$effect(() => {
		void jsonView
		if (!bodyEl) return
		bodyEl.scrollTop = activeTab === 'logs' && !jsonView ? bodyEl.scrollHeight : 0
	})

	// And stay on the end while the job writes: a log stream the user has to scroll to
	// read is not following the run.
	$effect(() => {
		void logs
		if (!bodyEl || jsonView || activeTab !== 'logs' || !running) return
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

	// The card outlives its job, and sometimes precedes it: a call cancelled before Run never
	// had one, and one that failed to start has none either. Synthesizing the shape
	// JobStatusIcon discriminates on keeps a single vocabulary of status badges rather than a
	// second one for the states only the card knows about.
	const statusJob = $derived(
		chatJob?.job ??
			((canceled
				? { canceled: true, success: false }
				: failed
					? { success: false, canceled: false }
					: { running: false }) as unknown as Job)
	)
	// The badge carries the outcome, so this is only ever how long it took, and 'Not run' where
	// there is no time to give because nothing ran.
	const statusTime = $derived(
		running
			? elapsed
			: ran
				? duration || (failed ? 'Failed' : canceled ? 'Cancelled' : 'Done')
				: 'Not run'
	)

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
			? 'Open this form in the preview panel'
			: aiChatManager.openRunInPreview
				? 'Open this run in the preview panel'
				: 'Open this run in a new tab'
	)

	function openPreview() {
		const label = runForm.summary || runForm.path
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

<!-- The run page's own status badge, so a run reads the same wherever it is met, with the
     time beside it: the badge says how it went, the number how long it took, and while it
     runs that number is still moving. Ahead of the label, so the chevron stays next to what
     it opens and the preview chip keeps the other end of the row to itself. -->
{#snippet status()}
	{#if !pending}
		<span class="inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap text-2xs text-hint">
			<JobStatusIcon job={statusJob} roundedFull size={11} badgeClass="p-1" />
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
	/>
{/snippet}

<!-- scroll-mb clears the chat's sticky "Waiting for your input" chip so the mount
     scrollIntoView of the form below leaves the Run button uncovered. -->
<ChatCollapsibleCard
	{label}
	{expanded}
	onToggle={() => (toggled = { id: message.tool_call_id, open: !expanded })}
	headerLeft={status}
	headerRight={previewTarget ? previewChip : undefined}
	class="scroll-mb-8"
	contentClass="p-0 overflow-hidden"
>
	{#if formInPreview}
		<div class="px-3 py-2 text-2xs leading-4 text-hint">
			These inputs are open in the preview panel.
		</div>
	{:else if pending}
		<RunArgsFormDisplay toolCallId={message.tool_call_id} {runForm} />
	{:else}
		<!-- One fixed-height region holding the strip and the body, so the card is the same size
		     on every tab and switching to the raw JSON does not resize it under the cursor. A cap
		     would not do it — the body is a scroll region, and a max-height silently beats
		     flex-grow. -->
		<div class="relative flex h-[20rem] flex-col">
			<!-- The tabs go in raw view: they name the parts of the body, and the raw call is not
			     one of them. The strip stays because the switch out of raw lives there — the run
			     page's own JSON toggle, which carries its label and so does not read as a fourth
			     tab. -->
			<!-- The strip's own height, not one its contents happen to add up to: the tabs leave
			     in raw view, and a row sized by what is in it would step every time they do. -->
			<Tabs
				selected={activeTab}
				on:selected={(e) => (userTab = e.detail)}
				class="h-8 px-3"
				wrapperClass="shrink-0"
			>
				{#if !jsonView}
					{#each tabs as tab (tab.value)}
						<!-- leading-4 and the tighter padding are the strip's height: text-2xs
						     inherits a 22px line box, which with Tab's own padding puts 12px of air
						     above and below a 11px label. -->
						<Tab
							value={tab.value}
							label={tab.label}
							class="py-0.5 text-2xs font-medium leading-4 text-secondary"
							selectedClass="text-accent border-border-accent"
							exact
						>
							{#snippet extra()}
								{#if tab.value === 'logs' && logLineCount > 0}
									<span class="text-2xs text-hint">{logLineCount}</span>
								{/if}
							{/snippet}
						</Tab>
					{/each}
				{/if}
				<div class="ml-auto flex items-center pl-2">
					<Toggle
						bind:checked={jsonView}
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
					'min-h-0 flex-1 overflow-auto px-3 py-2',
					!jsonView && activeTab === 'logs' ? 'bg-surface-secondary/50' : ''
				)}
			>
				<!-- min-h-full rather than h-full: the states that centre themselves need the height,
				     and a box that always filled it would measure as never scrollable. -->
				<div use:fades.content class="flex min-h-full flex-col">
					{#if jsonView}
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
					{:else if activeTab === 'input'}
						<!-- What the runs page shows a finished job's arguments as, for the same reason the
					     Result tab is DisplayResult: the operator has already read this table. The job id
					     is what lets it fetch arguments too big to have been persisted with the card. -->
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
						<!-- The run page's own renderer, not a second one invented for the chat: it is
					     what the result already looks like everywhere else, and it is the only thing
					     that handles markdown, tables, images, S3 files and deep nesting without the
					     card guessing at the shape. `disableExpand` drops its whole toolbar and
					     `hideAsJson` its Pretty/JSON switch: the row already owns both, opening it
					     bigger and reading it raw. jobId and workspace still let it reach the job for
					     an S3 preview. -->
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
						<div class="flex flex-1 flex-col items-center justify-center gap-1.5 px-4 text-center">
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
			</div>

			<!-- Over the body, not inside it: what is still below fades out, and nothing at the top,
			     as on the form and on the tool cards. Two layers on the logs, whose ground is the
			     card's surface with the softer one at half strength over it — one gradient would end
			     on the wrong colour and leave a band at the very edge. -->
			{#if fades.bottom && fadeBody}
				<div
					class="pointer-events-none absolute inset-x-0 bottom-0 h-[min(2rem,25%)] bg-gradient-to-t from-surface via-surface/60 to-transparent"
				></div>
				{#if activeTab === 'logs'}
					<div
						class="pointer-events-none absolute inset-x-0 bottom-0 h-[min(2rem,25%)] bg-gradient-to-t from-surface-secondary/50 via-surface-secondary/30 to-transparent"
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
