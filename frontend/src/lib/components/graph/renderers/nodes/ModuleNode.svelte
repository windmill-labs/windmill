<script lang="ts">
	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	import { GitBranchPlus, Move, Copy, Trash2, StickyNote, PictureInPicture2 } from 'lucide-svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { ModuleN } from '../../graphBuilder.svelte'
	import { jobToGraphModuleState } from '$lib/components/modulesTest.svelte'
	import { getNoteEditorContext } from '../../noteEditor.svelte'
	import { isMac, type Item } from '$lib/utils'
	import { getContext } from 'svelte'
	import { getGraphContext } from '../../graphContext'
	import { getFlowRunStatusContext } from '../../flowRunStatus.svelte'

	interface Props {
		data: ModuleN['data']
	}

	let { data }: Props = $props()

	// Get NoteEditor context for group note creation
	const noteEditorContext = getNoteEditorContext()
	const flowRunStatus = getFlowRunStatusContext()

	let state = $derived.by(() => {
		return data.testModuleState
			? (jobToGraphModuleState(data.testModuleState) ?? flowRunStatus?.getModuleState(data.id))
			: flowRunStatus?.getModuleState(data.id)
	})

	// A `message` is the one action with no tool node of its own, so the annotation is the only
	// place it can show.
	const AGENT_ACTION_KINDS = [
		{ type: 'tool_call', singular: 'tool call', plural: 'tool calls' },
		{ type: 'mcp_tool_call', singular: 'MCP call', plural: 'MCP calls' },
		{ type: 'web_search', singular: 'web search', plural: 'web searches' },
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
			// A missing entry in the parallel success array means the action is still running.
			const success = agentState?.agent_actions_success?.[index]
			if (success === undefined) entry.pending++
			else if (!success) failed++
			tally.set(action.type, entry)
			if (action.type !== 'message') {
				calls++
				if (success === undefined) pending++
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
					flowJobsSuccess: state?.flow_jobs_success
				}
			: (undefined as any)
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
			maximizeSubflow={data.module?.value?.type == 'flow' && 'path' in data.module.value
				? () => {
						const path =
							data.module?.value && 'path' in data.module.value
								? (data.module.value['path'] as string)
								: undefined
						if (path) {
							data.eventHandlers.expandSubflow(data.id, path)
						}
					}
				: undefined}
		/>

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
	{/snippet}
</NodeWrapper>
