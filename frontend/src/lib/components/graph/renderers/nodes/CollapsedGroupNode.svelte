<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import type { CollapsedGroupN } from '../../graphBuilder.svelte'
	import GroupModuleIcons from '../../GroupModuleIcons.svelte'
	import GroupHeaderBlock from '../../GroupHeaderBlock.svelte'
	import { NoteColor, NOTE_COLORS } from '../../noteColors'
	import { NODE } from '../../util'
	import { Hourglass } from 'lucide-svelte'
	import FlowStatusWaitingForEvents from '$lib/components/FlowStatusWaitingForEvents.svelte'
	import { dfs } from '$lib/components/flows/dfs'
	import { getFlowRunStatusContext } from '../../flowRunStatus.svelte'

	interface Props {
		data: CollapsedGroupN['data']
		id: string
	}

	let { data, id }: Props = $props()

	const flowRunStatus = getFlowRunStatusContext()

	let outlineColorClass = $derived(
		(NOTE_COLORS[(data.color as NoteColor) ?? NoteColor.BLUE] ?? NOTE_COLORS[NoteColor.BLUE])
			.outline
	)

	let bgColorClass = $derived(
		NOTE_COLORS[(data.color as NoteColor) ?? NoteColor.BLUE]?.backgroundLight ??
			NOTE_COLORS[NoteColor.BLUE].backgroundLight
	)

	let allModuleIds = $derived(data.modules ? dfs(data.modules, (m) => m.id) : [])

	let waitingForEvents = $derived(
		allModuleIds.some(
			(mid) =>
				flowRunStatus?.getModuleState(mid)?.type === 'WaitingForEvents' ||
				flowRunStatus?.getModuleState(`${mid}-v`)?.type === 'WaitingForEvents'
		)
	)
</script>

<NodeWrapper nodeId={id}>
	<div class="relative">
		<div
			class="w-full max-w-full rounded-lg outline outline-1 -outline-offset-1 {outlineColorClass} {bgColorClass}"
			style="width: {NODE.width}px;"
		>
			<GroupHeaderBlock
				groupId={data.groupId}
				summary={data.summary}
				note={data.note}
				color={data.color}
				collapsed={true}
				autocollapse={data.autocollapse ?? false}
				editMode={data.editMode}
				showNotes={data.showNotes}
			/>
			{#if data.modules && data.modules.length > 0}
				<div class="flex items-center justify-center w-full gap-1.5 px-2 h-[34px] overflow-hidden">
					<GroupModuleIcons modules={data.modules} eventHandlers={data.eventHandlers} />
				</div>
			{/if}
		</div>

		{#if waitingForEvents && flowRunStatus?.flowJob && flowRunStatus.flowJob.type === 'QueuedJob'}
			<div
				class="absolute top-1/2 -translate-y-1/2 left-full ml-2 flex items-center gap-2 nodrag nowheel"
			>
				<div
					class="px-2 py-0.5 rounded-md bg-surface shadow-md text-violet-700 dark:text-violet-400 text-xs flex items-center gap-1"
				>
					<Hourglass size={12} />
					<div class="flex">
						<span class="dot">.</span>
						<span class="dot">.</span>
						<span class="dot">.</span>
					</div>
				</div>
				<div class="rounded-md bg-surface flex items-center justify-center p-2 shadow-md">
					{#if flowRunStatus?.flowJob?.flow_status?.modules?.[flowRunStatus.flowJob.flow_status?.step]?.type === 'WaitingForEvents'}
						<FlowStatusWaitingForEvents
							job={flowRunStatus.flowJob}
							workspaceId={flowRunStatus.flowJob.workspace_id}
							isOwner={data.isOwner}
							light
						/>
					{:else if flowRunStatus?.suspendStatus && Object.keys(flowRunStatus.suspendStatus).length > 0}
						<div class="flex gap-2 flex-col">
							{#each Object.values(flowRunStatus.suspendStatus) as suspendCount (suspendCount.job.id)}
								<FlowStatusWaitingForEvents
									job={suspendCount.job}
									workspaceId={suspendCount.job.workspace_id}
									isOwner={data.isOwner}
									light
								/>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		{/if}
	</div>
</NodeWrapper>
