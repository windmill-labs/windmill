<script lang="ts">
	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	import {
		GitBranchPlus,
		Move,
		Copy,
		Trash2,
		StickyNote,
		PictureInPicture2,
		Bug
	} from 'lucide-svelte'
	import NodeWrapper from './NodeWrapper.svelte'
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

	/**
	 * Error handlers that ran inside this step but that the graph does not draw: anywhere in a
	 * collapsed subflow, or in a loop's other iterations while the one shown draws none. The
	 * selected iteration's own handlers are drawn, and nested steps report theirs.
	 */
	let hiddenErrorHandlers = $derived.by(
		(): Promise<{ label: string; reveal: () => void } | undefined> | undefined => {
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
									label: `${count} error handler${count > 1 ? 's' : ''} ran`,
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
										label: `Error handler ran in iteration ${first + 1}`,
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
		}
	)

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
			{flowJobs}
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

		{#await hiddenErrorHandlers then hidden}
			{#if hidden}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="nodrag nopan absolute left-0 top-full z-10"
					onpointerdown={(e) => e.stopPropagation()}
				>
					<Button
						variant="subtle"
						unifiedSize="2xs"
						startIcon={{ icon: Bug, classes: 'text-red-400 dark:text-red-500' }}
						btnClasses="text-2xs text-secondary px-0 gap-1"
						title={subflowPath ? 'Expand the subflow' : 'Show that iteration'}
						onClick={(e) => {
							e?.stopPropagation()
							hidden.reveal()
						}}
					>
						{hidden.label}
					</Button>
				</div>
			{/if}
		{/await}

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
