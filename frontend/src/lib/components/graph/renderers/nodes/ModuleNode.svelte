<script lang="ts">
	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	import {
		GitBranchPlus,
		Move,
		Copy,
		Trash2,
		StickyNote,
		PictureInPicture2,
		Bug,
		RefreshCw,
		Gauge,
		Hourglass,
		Moon
	} from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import WaitingMark from './WaitingMark.svelte'
	import AiDecisionNodeBranch from './AiDecisionNodeBranch.svelte'
	import type { ModuleN } from '../../graphBuilder.svelte'
	import { jobToGraphModuleState } from '$lib/components/modulesTest.svelte'
	import { getNoteEditorContext } from '../../noteEditor.svelte'
	import { isMac, type Item } from '$lib/utils'
	import { getContext, untrack } from 'svelte'
	import { getGraphContext } from '../../graphContext'
	import { getFlowRunStatusContext } from '../../flowRunStatus.svelte'
	import { waitingRequest } from '$lib/components/agentRequest'
	import { Button } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'
	import { countErrorHandlersUnder, firstIterationWithHandler } from '../../nestedErrorHandlers'

	interface Props {
		data: ModuleN['data']
	}

	let { data }: Props = $props()

	// Get NoteEditor context for group note creation
	const noteEditorContext = getNoteEditorContext()
	const flowRunStatus = getFlowRunStatusContext()

	let state = $derived.by(() => {
		const state = data.testModuleState
			? (jobToGraphModuleState(data.testModuleState) ?? flowRunStatus?.getModuleState(data.id))
			: flowRunStatus?.getModuleState(data.id)
		// An agent waiting on a person reads like an approval step, though its job is running.
		return state?.type === 'InProgress' && waitingRequest(state)
			? { ...state, type: 'WaitingForEvents' as const }
			: state
	})

	// A `message` is the one action with no tool node of its own, so the annotation is the only
	// place it can show.
	const AGENT_ACTION_KINDS = [
		{ type: 'tool_call', singular: 'tool call', plural: 'tool calls' },
		{ type: 'mcp_tool_call', singular: 'MCP call', plural: 'MCP calls' },
		{ type: 'web_search', singular: 'web search', plural: 'web searches' },
		{ type: 'approval', singular: 'approval request', plural: 'approval requests' },
		{ type: 'question', singular: 'question', plural: 'questions' },
		{ type: 'message', singular: 'message', plural: 'messages' }
	] as const

	/**
	 * The editor draws an agent's declared tools, not the calls a run makes, so a run would
	 * otherwise leave no trace on the step. It is drawn beside the step, out of the flow and clear
	 * of the tool rows above, so it reports the run without moving anything.
	 */
	let agentToolSummary = $derived.by(() => {
		if (!data.insertable || data.module?.value?.type !== 'aiagent') return undefined
		const agentState = flowRunStatus?.getModuleState(data.id)
		const actions = agentState?.agent_actions
		if (!actions?.length) return undefined
		const tally = new Map<string, { count: number; pending: number }>()
		let calls = 0
		let pending = 0
		let failed = 0
		actions.forEach((action, index) => {
			const entry = tally.get(action.type) ?? { count: 0, pending: 0 }
			entry.count++
			// A missing or null entry in the parallel success array means the action is still running.
			const success = agentState?.agent_actions_success?.[index]
			if (success == null) entry.pending++
			else if (!success) failed++
			tally.set(action.type, entry)
			if (action.type !== 'message') {
				calls++
				if (success == null) pending++
			}
		})
		const detail: string[] = []
		for (const kind of AGENT_ACTION_KINDS) {
			const entry = tally.get(kind.type)
			if (!entry) continue
			detail.push(`${entry.count} ${entry.count > 1 ? kind.plural : kind.singular}`)
		}
		if (failed > 0) detail.push(`${failed} failed`)
		// The visible line sits beside the step where a long string would run into the canvas, so
		// it stays a total and the per-kind breakdown rides the tooltip.
		const messages = tally.get('message')?.count ?? 0
		const short =
			calls > 0
				? `${calls} call${calls > 1 ? 's' : ''}${pending > 0 ? '…' : ''}${failed > 0 ? ` · ${failed}✗` : ''}`
				: `${messages} msg${messages > 1 ? 's' : ''}`
		return { short, detail: detail.join(' · ') }
	})

	let flowJobs = $derived(
		state?.flow_jobs
			? {
					flowJobs: state?.flow_jobs,
					selected: state?.selectedForloopIndex ?? 0,
					selectedManually: state?.selectedForLoopSetManually,
					flowJobsSuccess: state?.flow_jobs_success,
					flowJobsLabels: state?.flow_jobs_labels
				}
			: (undefined as any)
	)

	let subflowPath = $derived(
		data.module?.value?.type == 'flow' && 'path' in data.module.value
			? (data.module.value['path'] as string | undefined)
			: undefined
	)

	function expandSubflow() {
		if (subflowPath) data.eventHandlers.expandSubflow(data.id, subflowPath)
	}

	// The walk below only re-runs when the step's jobs change: `state` is a new object on
	// every status update. A step still running has no count yet, as its jobs are still changing.
	let handlerCountKey = $derived(
		!data.editMode && (state?.type === 'Success' || state?.type === 'Failure')
			? [state.job_id, state.selectedForloopIndex, ...(state.flow_jobs ?? [])].join(',')
			: undefined
	)

	/** Something that happened inside a step the graph does not draw, and how to reveal it. */
	type InsideMark = { count: number; label: string; title: string; reveal: () => void }

	/**
	 * Error handlers that ran inside this step but that the graph does not draw: anywhere in a
	 * collapsed subflow, or in a loop's other iterations while the one shown draws none. The
	 * selected iteration's own handlers are drawn, and nested steps report theirs.
	 */
	let hiddenErrorHandlers = $derived.by((): Promise<InsideMark | undefined> | undefined => {
		const workspace = $workspaceStore
		if (!handlerCountKey || !workspace) return undefined
		const s = untrack(() => state)
		if (!s) return undefined
		const valueType = data.module?.value?.type
		if (valueType === 'flow' && s.job_id && subflowPath) {
			return countErrorHandlersUnder(workspace, s.job_id).then(
				(count) =>
					count > 0
						? {
								count,
								label: `${count} error handler${count > 1 ? 's' : ''} ran`,
								title: 'Expand the subflow',
								reveal: expandSubflow
							}
						: undefined,
				() => undefined
			)
		}
		const iterations = s.flow_jobs
		const isLoop = valueType === 'forloopflow' || valueType === 'whileloopflow'
		if (isLoop && data.mayRunErrorHandler && iterations?.length) {
			const index = s.selectedForloopIndex ?? 0
			const shown = index >= 0 ? index : iterations.length - 1
			const success = s.flow_jobs_success
			return countErrorHandlersUnder(workspace, iterations[shown])
				.then(async (count) =>
					// The iteration on screen already draws a handler: pointing at the others would
					// keep the reader clicking through iterations.
					count > 0 ? undefined : await firstIterationWithHandler(workspace, iterations, success)
				)
				.then(
					(first) =>
						first === undefined
							? undefined
							: {
									count: 1,
									label: `Error handler ran in iteration ${first + 1}`,
									title: 'Show that iteration',
									reveal: () =>
										data.eventHandlers.selectedIteration({
											id: iterations[first],
											index: first,
											manuallySet: true,
											moduleId: data.id
										})
								},
					() => undefined
				)
		}
		return undefined
	})

	let attempts = $derived.by(() => {
		const jobs = state?.attempt_jobs
		if (!jobs?.length) return undefined
		const shown = state?.selectedAttempt ? jobs.indexOf(state.selectedAttempt) : -1
		return {
			flowJobs: jobs,
			flowJobsSuccess: state?.attempt_jobs_success ?? [],
			selected: shown >= 0 ? shown : jobs.length - 1
		}
	})

	// An expanded subflow is drawn as its steps instead of this node, each with its own bar.
	let retriedInside = $derived.by((): InsideMark | undefined => {
		const count =
			flowRunStatus?.showsRun && !data.editMode && data.module?.value?.type === 'flow'
				? flowRunStatus.retriedStepsInside(data.id)
				: 0
		return count > 0
			? {
					count,
					label: `${count} step${count === 1 ? '' : 's'} retried inside`,
					title: 'Expand to see the retried steps',
					reveal: expandSubflow
				}
			: undefined
	})

	// Live, so it takes the slot of the retried and error handler counts while it lasts.
	let waitingInside = $derived.by(() => {
		if (!flowRunStatus?.showsRun || data.editMode || data.module?.value?.type !== 'flow') return
		const { approval, sleep, concurrency } = flowRunStatus.waitingStepsInside(data.id)
		if (approval > 0)
			return {
				reason: 'approval' as const,
				label: `${approval === 1 ? 'Approval' : `${approval} approvals`} waiting inside`
			}
		if (concurrency > 0)
			return {
				reason: 'concurrency' as const,
				label: `${concurrency} step${concurrency === 1 ? '' : 's'} held by concurrency limit`
			}
		if (sleep > 0)
			return { reason: 'sleep' as const, label: `${sleep} step${sleep === 1 ? '' : 's'} sleeping` }
	})

	let type = $derived.by(() => {
		let typ = state?.type
		if (!typ && flowJobs) {
			return 'InProgress'
		}
		return typ
	})

	// Define context menu items
	let noteDisabled = $derived(
		!noteEditorContext?.noteEditor ||
			(noteEditorContext?.noteEditor?.isNodeOnlyMemberOfGroupNote(data.id) ?? false)
	)

	let isPreprocessor = $derived(data.id === 'preprocessor')

	// In modal-panel mode (sessions) step details open on double-click, or on a click on the
	// already selected step. Surface the action in the ellipsis menu too, with the gesture
	// that works from any state as its shortcut.
	const stepExploreHint = getContext<(() => boolean) | undefined>('flowGraphStepExploreHint')
	const selectionManager = getGraphContext()?.selectionManager

	const menuItems: Item[] = $derived(
		data.editMode
			? [
					...(stepExploreHint?.()
						? [
								{
									displayName: 'Open details',
									icon: PictureInPicture2,
									shortcut: 'Double click',
									action: () => selectionManager?.selectId(data.id, { openPanel: true })
								}
							]
						: []),
					...(isPreprocessor
						? []
						: [
								{
									displayName: 'Move',
									icon: Move,
									action: () => data.eventHandlers.move({ id: data.id })
								},
								{
									displayName: 'Duplicate',
									icon: Copy,
									action: () => data.eventHandlers.duplicate({ id: data.id })
								}
							]),
					{
						displayName: 'Delete',
						icon: Trash2,
						type: 'delete' as const,
						shortcut: isMac() ? '⌫' : 'Del',
						action: () => data.eventHandlers.delete({ id: data.id }, '')
					},
					{
						displayName: 'Add note',
						icon: StickyNote,
						separatorTop: true,
						disabled: noteDisabled,
						action: () => {
							if (noteEditorContext?.noteEditor && !noteDisabled) {
								noteEditorContext.noteEditor.createGroupNote([data.id])
							}
						}
					}
				]
			: []
	)
</script>

<NodeWrapper {menuItems}>
	{#snippet children({ darkMode })}
		<MapItem
			moduleId={data.id}
			mod={data.module}
			insertable={data.insertable}
			editMode={data.editMode}
			mockApplied={data.editMode
				? undefined
				: flowRunStatus?.showsRun
					? state?.job_kind === 'identity' && !state.skipped
					: !data.module?.mock?.preview_only}
			moduleAction={data.moduleAction}
			{menuItems}
			sideAnnotation={agentToolSummary?.short}
			sideAnnotationTitle={agentToolSummary?.detail}
			annotation={flowJobs &&
			(data.module?.value?.type === 'forloopflow' || data.module?.value?.type === 'whileloopflow')
				? 'Iteration: ' +
					((state?.selectedForloopIndex ?? 0) >= 0
						? (state?.selectedForloopIndex ?? 0) + 1
						: state?.flow_jobs?.length) +
					'/' +
					(state?.iteration_total ?? '?')
				: ''}
			nodeState={state?.skipped ? '_Skipped' : type}
			duration_ms={state?.duration_ms}
			retries={state?.retries}
			held={type === 'WaitingForExecutor' && state?.waiting_reason === 'concurrency'
				? { since: state.held_since, nextCheck: state.scheduled_for }
				: undefined}
			{flowJobs}
			{attempts}
			on:delete={(e) => {
				data.eventHandlers.delete(e.detail, '')
			}}
			on:changeId={(e) => {
				data.eventHandlers.changeId(e.detail)
			}}
			on:move={(e) => {
				data.eventHandlers.move({ id: data.id })
			}}
			on:newBranch={(e) => {
				data.eventHandlers.newBranch(data.id)
			}}
			onSelect={(e) => {
				setTimeout(() => e && data.eventHandlers.select(e))
			}}
			onSelectedIteration={(e) => {
				data.eventHandlers.selectedIteration(e)
			}}
			onTestUpTo={data.eventHandlers.testUpTo}
			onUpdateMock={(detail) => {
				data.eventHandlers.updateMock(detail)
			}}
			onEditInput={data.eventHandlers.editInput}
			flowJob={flowRunStatus?.flowJob}
			isOwner={data.isOwner}
			errorHandler={data.isErrorHandler}
			maximizeSubflow={data.module?.value?.type == 'flow' && 'path' in data.module.value
				? expandSubflow
				: undefined}
		/>

		{#if type === 'WaitingForExecutor' && state?.waiting_reason}
			<WaitingMark
				reason={state.waiting_reason}
				until={state.scheduled_for}
				concurrencyKey={state.concurrency_key}
				heldSince={state.held_since}
				onclick={() => data.module && data.eventHandlers.select(data.module.id)}
			/>
		{:else if state?.approval_wait}
			<WaitingMark
				reason="approval"
				approvalWait={state.approval_wait}
				onclick={() => data.module && data.eventHandlers.select(data.module.id)}
			/>
		{/if}
		{#if waitingInside}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="nodrag nopan absolute -left-1 top-[calc(100%+3px)] z-10"
				onpointerdown={(e) => e.stopPropagation()}
			>
				<Button
					variant="subtle"
					unifiedSize="2xs"
					startIcon={{
						icon:
							waitingInside.reason === 'approval'
								? Hourglass
								: waitingInside.reason === 'sleep'
									? Moon
									: Gauge,
						classes:
							waitingInside.reason === 'approval'
								? 'text-violet-500'
								: waitingInside.reason === 'sleep'
									? ''
									: 'text-orange-500'
					}}
					btnClasses="text-2xs text-secondary gap-1"
					title="Expand to see the waiting steps"
					onClick={(e) => {
						e?.stopPropagation()
						expandSubflow()
					}}
				>
					{waitingInside.label}
				</Button>
			</div>
		{:else}
			{#await hiddenErrorHandlers}
				{@render insideBar(retriedInside, undefined)}
			{:then handlers}
				{@render insideBar(retriedInside, handlers)}
			{/await}
		{/if}

		{#if (data.module?.value?.type === 'branchall' || data.module?.value?.type === 'branchone') && data.insertable}
			<div class="absolute -bottom-10 left-1/2 transform -translate-x-1/2 z-10 flex gap-1">
				<button
					title="Add branch"
					class="rounded text-secondary border hover:bg-surface-hover bg-surface p-1"
					onclick={() => {
						data?.eventHandlers?.newBranch(data.id)
					}}
				>
					<GitBranchPlus size={16} />
				</button>
			</div>
		{/if}
		{#if data.module?.value?.type === 'aidecision' && data.insertable}
			<AiDecisionNodeBranch moduleId={data.id} />
		{/if}
	{/snippet}
</NodeWrapper>

<!-- Left of the settings bar, which is this step's own policy: this is what happened inside. -->
{#snippet insideBar(retried: InsideMark | undefined, handlers: InsideMark | undefined)}
	{#if retried || handlers}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="nodrag nopan absolute -left-1 top-[calc(100%+3px)] z-10"
			onpointerdown={(e) => e.stopPropagation()}
		>
			{#if retried && handlers}
				<!-- Both at once: their counts side by side, each way in from the menu. -->
				<DropdownV2
					placement="bottom-start"
					usePointerDownOutside
					fixedHeight={false}
					items={[
						{
							displayName: retried.label,
							icon: RefreshCw,
							iconColor: '#f97316',
							action: retried.reveal
						},
						{
							displayName: handlers.label,
							icon: Bug,
							iconColor: '#f87171',
							action: handlers.reveal
						}
					]}
				>
					{#snippet buttonReplacement()}
						<Button
							nonCaptureEvent
							variant="subtle"
							unifiedSize="2xs"
							btnClasses="gap-2 text-2xs text-secondary"
							title="{retried.label}, {handlers.label}"
						>
							<span class="flex items-center gap-0.5">
								<RefreshCw size={12} class="text-orange-500" />{retried.count}
							</span>
							<span class="flex items-center gap-0.5">
								<Bug size={12} class="text-red-400 dark:text-red-500" />{handlers.count}
							</span>
						</Button>
					{/snippet}
				</DropdownV2>
			{:else}
				{@const mark = (retried ?? handlers)!}
				<Button
					variant="subtle"
					unifiedSize="2xs"
					startIcon={{
						icon: retried ? RefreshCw : Bug,
						classes: retried ? 'text-orange-500' : 'text-red-400 dark:text-red-500'
					}}
					btnClasses="text-2xs text-secondary gap-1"
					title={mark.title}
					onClick={(e) => {
						e?.stopPropagation()
						mark.reveal()
					}}
				>
					{mark.label}
				</Button>
			{/if}
		</div>
	{/if}
{/snippet}
