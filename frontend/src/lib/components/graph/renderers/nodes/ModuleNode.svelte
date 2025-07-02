<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	import { GitBranchPlus, Maximize2 } from 'lucide-svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import { getStateColor, getStateHoverColor } from '../../util'
	import type { ModuleN } from '../../graphBuilder.svelte'

	interface Props {
		data: ModuleN['data']
	}

	let { data }: Props = $props()

	let moduleState = $derived(data.flowModuleStates?.[data.id])
	let flowJobs = $derived(
		moduleState?.flow_jobs
			? {
					flowJobs: moduleState?.flow_jobs,
					selected: moduleState?.selectedForloopIndex ?? 0,
					selectedManually: moduleState?.selectedForLoopSetManually,
					flowJobsSuccess: moduleState?.flow_jobs_success
				}
			: (undefined as any)
	)

	let type = $derived.by(() => {
		let typ = data.flowModuleStates?.[data.id]?.type
		if (!typ && flowJobs) {
			return 'InProgress'
		}
		return typ
	})
</script>

<NodeWrapper offset={data.offset}>
	{#snippet children({ darkMode })}
		{#if data.module.value.type == 'flow'}
			<button
				title="Expand subflow"
				class="z-50 absolute -top-[10px] right-[25px] rounded-full h-[20px] w-[20px] center-center text-primary bg-surface duration-0 hover:bg-surface-hover"
				onclick={stopPropagation(
					preventDefault(() => {
						if (data.module.value.type == 'flow') {
							data.eventHandlers.expandSubflow(data.id, data.module.value.path)
						}
					})
				)}
			>
				<Maximize2 size={12} />
			</button>
		{/if}
		<MapItem
			moduleId={data.id}
			mod={data.module}
			insertable={data.insertable}
			editMode={data.editMode}
			annotation={flowJobs &&
			(data.module.value.type === 'forloopflow' || data.module.value.type === 'whileloopflow')
				? 'Iteration: ' +
					((moduleState?.selectedForloopIndex ?? 0) >= 0
						? (moduleState?.selectedForloopIndex ?? 0) + 1
						: moduleState?.flow_jobs?.length) +
					'/' +
					(moduleState?.iteration_total ?? '?')
				: ''}
			bgColor={getStateColor(
				data.editMode ? undefined : type,
				darkMode,
				true,
				moduleState?.skipped
			)}
			bgHoverColor={getStateHoverColor(
				data.editMode ? undefined : type,
				darkMode,
				true,
				moduleState?.skipped
			)}
			moving={data.moving}
			duration_ms={data.editMode ? undefined : moduleState?.duration_ms}
			retries={moduleState?.retries}
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
			waitingJob={data.waitingJob}
			isOwner={data.isOwner}
			{type}
			{darkMode}
			skipped={moduleState?.skipped}
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
