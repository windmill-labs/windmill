<script lang="ts">
	import { Ban, Braces, Code, Loader2, PanelRight, TimerOff } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Button, Tab, Tabs } from '$lib/components/common'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { displayDate, msToReadableTime } from '$lib/utils'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import { base } from '$lib/base'
	import { getAiChatManager } from './aiChatManagerContext'
	import RunArgsFormDisplay from './RunArgsFormDisplay.svelte'
	import ToolContentDisplay from './ToolContentDisplay.svelte'
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
	const tabs = $derived([
		// The table below keeps JobArgs' own "Input" heading, so the tab is named for what the
		// chat calls them instead of repeating that word twice over.
		{ value: 'input', label: 'Parameters' },
		...(ran ? [{ value: 'logs', label: 'Logs' }] : []),
		// Only once there is an outcome: the tab appearing is how the card says the run
		// landed, so it must not sit there empty while the job is still going.
		...(settled ? [{ value: 'outcome', label: outcomeTab }] : [])
	])

	// Undefined until a tab is clicked, and never cleared after: the run follows itself
	// only for as long as nobody has steered the card, and then it stops taking it back.
	let userTab = $state<string | undefined>(undefined)
	let jsonView = $state(false)

	// However the run landed, that is what the card opens on.
	const autoTab = $derived(settled ? 'outcome' : ran ? 'logs' : 'input')
	const activeTab = $derived(userTab && tabs.some((t) => t.value === userTab) ? userTab : autoTab)

	let bodyEl: HTMLDivElement | undefined = $state()

	// One scroll region serves every tab, so a switch has to rewind it: the logs leave it
	// at the tail, and the tab opened next would start part-way down its own content.
	$effect(() => {
		void activeTab
		void jsonView
		if (bodyEl) bodyEl.scrollTop = 0
	})

	// Follow the tail while the job writes: a log stream the user has to scroll to read
	// is not following the run.
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
	const startedAt = $derived(displayDate(chatJob?.job?.started_at ?? undefined, true, false))
	const worker = $derived(chatJob?.job?.worker ?? '')

	// The run's own coordinates, on the row where a card ends. No duration: the header
	// already carries it, and next to the status is where it means something. Empty parts
	// are dropped rather than left as stray separators, since a job that has not reported
	// yet has none.
	const footerParts = $derived(
		running ? [logLineCount > 0 ? `${logLineCount} lines` : '', worker] : [worker, startedAt]
	)
	const footer = $derived(footerParts.filter(Boolean).join(' · '))

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
	// True while the panel holds this call, form or run: read through the resolver so this
	// tracks the tab list, not the pane's mounted tab.
	const inPreview = $derived(
		aiChatManager.isCallInPreview?.({
			toolCallId: message.tool_call_id,
			jobId: chatJob?.jobId
		}) ?? false
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

<!-- scroll-mb clears the chat's sticky "Waiting for your input" chip so the mount
     scrollIntoView of the form below leaves the Run button uncovered. -->
<div
	class="scroll-mb-8 flex flex-col rounded-md border border-border-light bg-surface-tertiary shadow-sm"
>
	<div class="flex items-start gap-2 p-3">
		<!-- The script's own kind icon, as `getJobKindIcon` gives it everywhere else. Run belongs
		     to the button that runs it, not to the heading of the thing being run. -->
		<Code class="mt-0.5 h-4 w-4 shrink-0 text-accent" />
		<div class="min-w-0 flex-1">
			<p class="truncate text-xs font-semibold text-emphasis">
				Run {runForm.summary || runForm.path}
			</p>
			{#if runForm.summary}
				<p class="truncate font-mono text-2xs text-secondary">{runForm.path}</p>
			{/if}
		</div>
		{#if !pending || previewTarget}
			<div class="flex shrink-0 items-center gap-1">
				<!-- Status and time read as one thing: the colour and the dot say how it went, the
				     number says how long it took, and while it runs that number is still moving. -->
				{#if pending}
					<!-- Nothing to report yet, and nothing to read as JSON either. -->
				{:else if running}
					<span
						class="inline-flex items-center gap-1.5 whitespace-nowrap text-2xs font-medium text-blue-600 dark:text-blue-400"
					>
						<Loader2 class="h-3 w-3 animate-spin" />
						{elapsed}
					</span>
				{:else if failed}
					<span
						class="inline-flex items-center gap-1.5 whitespace-nowrap text-2xs font-medium text-red-600 dark:text-red-400"
					>
						<span class="h-[7px] w-[7px] shrink-0 rounded-full bg-current"></span>
						Failed
					</span>
				{:else if canceled && ran}
					<!-- A cancelled run ends on an execution error like any other, and it still took
					     the time it took: red, with the clock rather than the word, since the outcome
					     tab is already the one saying it was stopped. -->
					<span
						class="inline-flex items-center gap-1.5 whitespace-nowrap text-2xs font-medium text-red-600 dark:text-red-400"
					>
						<span class="h-[7px] w-[7px] shrink-0 rounded-full bg-current"></span>
						{duration || 'Cancelled'}
					</span>
				{:else if canceled}
					<!-- Nothing ran, so nothing errored. -->
					<span class="inline-flex items-center gap-1.5 whitespace-nowrap text-2xs text-tertiary">
						<span class="h-[7px] w-[7px] shrink-0 rounded-full bg-current"></span>
						Not run
					</span>
				{:else}
					<span
						class="inline-flex items-center gap-1.5 whitespace-nowrap text-2xs font-medium text-green-600 dark:text-green-400"
					>
						<span class="h-[7px] w-[7px] shrink-0 rounded-full bg-current"></span>
						{duration || 'Done'}
					</span>
				{/if}
				{#if !pending && !inPreview}
					<!-- One button, not a pair: pressed means the raw JSON of the whole call, the
					     shape every other tool card in the chat is read in. Gone while the panel
					     holds the call: there is no body here for it to switch. -->
					<Button
						iconOnly
						variant="default"
						unifiedSize="sm"
						btnClasses="h-[23px] min-h-[23px] w-[23px] p-0"
						selected={jsonView}
						title="Show this call as raw JSON"
						startIcon={{ icon: Braces }}
						onClick={() => (jsonView = !jsonView)}
					/>
				{/if}
				{#if previewTarget}
					<!-- One control for the whole call: the form on its way in, the run on its way
					     out. Not a toggle — pressing it again focuses the tab it already opened. -->
					<Button
						iconOnly
						variant="default"
						unifiedSize="sm"
						btnClasses="h-[23px] min-h-[23px] w-[23px] p-0"
						title={previewTitle}
						startIcon={{ icon: PanelRight }}
						onClick={openPreview}
					/>
				{/if}
			</div>
		{/if}
	</div>

	{#if inPreview}
		<!-- The panel is showing this call, so the card does not show it twice; closing that
		     tab brings the body back. For the form that is a requirement rather than a
		     preference: two mounted copies would be two views binding the one draft, each
		     reordering the schema SchemaForm edits in place. -->
		<div class="border-t border-border-light px-3 py-2 text-2xs leading-4 text-hint">
			{pending
				? 'The parameters are open in the preview panel.'
				: 'This run is open in the preview panel.'}
		</div>
	{:else if pending}
		<RunArgsFormDisplay toolCallId={message.tool_call_id} {runForm} />
	{:else}
		<!-- One fixed-height region holding the strip and the body, so the card is the same
		     size on every tab, in every state, and with the strip gone in JSON: what the
		     strip gives up, the body takes. A cap here would not do it — the body is a
		     scroll region, and a max-height silently beats flex-grow. -->
		<div class="flex h-[14.5rem] flex-col">
			{#if !jsonView}
				<Tabs
					selected={activeTab}
					on:selected={(e) => (userTab = e.detail)}
					class="border-t border-border-light px-3"
					wrapperClass="shrink-0"
				>
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
				</Tabs>
			{/if}

			<!-- Logs and raw JSON sit on the sunken surface, the way program output is shown
			     everywhere else; the two rendered views keep the card's own surface. -->
			<div
				bind:this={bodyEl}
				class={twMerge(
					'min-h-0 flex-1 overflow-auto px-3 py-2',
					jsonView || activeTab === 'logs' ? 'bg-surface-sunken' : ''
				)}
			>
				{#if jsonView}
					<div class="space-y-3">
						<ToolContentDisplay title="Parameters" content={message.parameters} />
						<ToolContentDisplay title="Logs" content={message.logs} />
						<ToolContentDisplay title="Result" content={message.result} error={message.error} />
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
						<pre class="whitespace-pre-wrap break-words font-mono text-2xs text-primary">{logs}</pre
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
				{:else if resultValue !== undefined}
					<!-- The run page's own renderer, not a second one invented for the chat: it is
					     what the result already looks like everywhere else, and it is the only thing
					     that handles markdown, tables, images, S3 files and deep nesting without the
					     card guessing at the shape. `disableExpand` drops its whole toolbar and
					     `hideAsJson` its Pretty/JSON switch: the header already owns both, opening it
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
					<div class="flex h-full flex-col items-center justify-center gap-1.5 px-4 text-center">
						<Ban class="h-4 w-4 text-tertiary" />
						<p class="text-2xs font-medium leading-4 text-secondary">{cancelReason}</p>
						{#if !ran}
							<p class="text-2xs leading-4 text-tertiary">
								The parameters it would have run with are on the Parameters tab.
							</p>
						{/if}
					</div>
				{:else}
					<p class="text-2xs text-tertiary">This run returned no result.</p>
				{/if}
			</div>
		</div>

		<!-- Where the run itself is named: which worker took it, how long it took, when it
		     started. It sits outside the fixed region, so the JSON view keeps it too. -->
		<div
			class="flex items-center gap-2 border-t border-border-light px-3 py-1.5 text-2xs leading-4 text-hint"
			class:hidden={!footer && !(running && chatJob)}
		>
			<span class="truncate">{footer}</span>
			{#if running && chatJob}
				<span class="flex-1"></span>
				<!-- The run page's own cancel button, down to the icon: stopping a run is the same
				     act here, and the operator has already pressed this one. -->
				<Button
					variant="accent"
					unifiedSize="sm"
					destructive
					startIcon={{ icon: TimerOff }}
					btnClasses="h-[26px] min-h-[26px] px-2.5"
					wrapperClasses="shrink-0"
					title="Cancel the script"
					onClick={() => aiChatManager.cancelJob(chatJob.jobId)}
				>
					Cancel
				</Button>
			{/if}
		</div>
	{/if}
</div>
