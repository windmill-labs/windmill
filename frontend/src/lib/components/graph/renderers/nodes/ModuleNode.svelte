<script lang="ts">
	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	import { GitBranchPlus } from 'lucide-svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import type { ModuleN } from '../../graphBuilder.svelte'
	import { jobToGraphModuleState } from '$lib/components/modulesTest.svelte'
	import { getNoteEditorContext } from '../../noteEditor.svelte'
	import type { ContextMenuItem } from '../../../common/contextmenu/ContextMenu.svelte'
	import { addGroupNoteContextMenuItem } from '../../noteUtils.svelte'

	interface Props {
		data: ModuleN['data']
	}

	let { data }: Props = $props()

	// Get NoteEditor context for group note creation
	const noteEditorContext = getNoteEditorContext()

	let state = $derived.by(() => {
		return data.testModuleState
			? (jobToGraphModuleState(data.testModuleState) ?? data.flowModuleState)
			: data.flowModuleState
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
	const contextMenuItems: ContextMenuItem[] = $derived(
		data.editMode ? [addGroupNoteContextMenuItem(data.id, noteEditorContext)] : []
	)
</script>

<NodeWrapper offset={data.offset} {contextMenuItems}>
	{#snippet children({ darkMode })}
		<MapItem
			moduleId={data.id}
			mod={data.module}
			insertable={data.insertable}
			editMode={data.editMode}
			moduleAction={data.moduleAction}
			annotation={flowJobs &&
			(data.module.value.type === 'forloopflow' || data.module.value.type === 'whileloopflow')
				? 'Iteration: ' +
					((state?.selectedForloopIndex ?? 0) >= 0
						? (state?.selectedForloopIndex ?? 0) + 1
						: state?.flow_jobs?.length) +
					'/' +
					(state?.iteration_total ?? '?')
				: ''}
			nodeState={state?.skipped ? '_Skipped' : type}
			moving={data.moving}
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
			flowJob={data.flowJob}
			isOwner={data.isOwner}
			maximizeSubflow={data.module.value.type == 'flow' && 'path' in data.module.value
				? () => {
						data.eventHandlers.expandSubflow(data.id, data.module.value['path'])
					}
				: undefined}
		/>

		<div class="absolute -bottom-10 left-1/2 transform -translate-x-1/2 z-10">
			{#if (data.module.value.type === 'branchall' || data.module.value.type === 'branchone') && data.insertable}
				<button
					title="Add branch"
					class="rounded text-secondary border hover:bg-surface-hover bg-surface p-1"
					onclick={() => {
						data?.eventHandlers?.newBranch(data.id)
					}}
				>
					<GitBranchPlus size={16} />
				</button>
			{/if}
		</div>
	{/snippet}
</NodeWrapper>
