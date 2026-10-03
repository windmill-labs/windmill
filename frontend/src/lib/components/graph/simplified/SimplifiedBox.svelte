<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { GitBranchPlus } from 'lucide-svelte'
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import { Button } from '$lib/components/common'
	import StepContextMenu from './StepContextMenu.svelte'
	import { getSimplifiedEditContext, useStepMenu } from './simplifiedContext'
	import { getFlowRunStatusContext } from '../flowRunStatus.svelte'
	import { SIMPLIFIED, type SimplifiedBoxData } from './simplifiedLayout'
	import GroupHeaderBlock from '../GroupHeaderBlock.svelte'
	import { NOTE_COLORS, NoteColor } from '../noteColors'

	let { id, data, selected }: { id: string; data: SimplifiedBoxData; selected?: boolean } = $props()

	let header = $derived(data.header)

	// The loop or branch step has no node of its own here, so a run that fails or waits at the
	// step itself (an iterator or predicate throwing) is shown on its bracket. Success stays
	// neutral: the steps inside already carry it.
	const flowRunStatus = getFlowRunStatusContext()
	const STATUS_BRACKET: Record<string, string> = {
		Failure: 'border-red-500',
		InProgress: 'border-orange-400',
		WaitingForExecutor: 'border-orange-400',
		WaitingForEvents: 'border-purple-400'
	}
	let statusBracket = $derived(STATUS_BRACKET[flowRunStatus?.getModuleState(id)?.type ?? ''])

	const headerTop = SIMPLIFIED.loopPad / 2

	const edit = getSimplifiedEditContext()
	const stepMenu = useStepMenu()
	let menuItems = $derived(data.kind === 'group' ? [] : stepMenu(id, header.module))
	let canAddBranch = $derived(data.kind === 'branch' && !!edit?.editMode && !!edit.eventHandlers)

	let groupColors = $derived(
		NOTE_COLORS[(data.group?.value.color as NoteColor) ?? NoteColor.BLUE] ??
			NOTE_COLORS[NoteColor.BLUE]
	)
</script>

{#if data.kind === 'group' && data.group}
	<!-- A user-made group keeps the full graph's outlined, tinted box and its editable header -->
	<div
		class={twMerge(
			'relative rounded-lg outline outline-1 -outline-offset-1 pointer-events-none',
			groupColors.outline,
			groupColors.backgroundLight
		)}
		style="width: {data.width}px; height: {data.height}px;"
	>
		<div class="absolute inset-x-0 top-0 pointer-events-auto">
			<GroupHeaderBlock
				groupId={data.group.key}
				summary={data.group.value.summary}
				note={data.group.value.note}
				color={data.group.value.color}
				collapsed={false}
				autocollapse={data.group.value.autocollapse ?? false}
				editMode={!!edit?.editMode}
				showNotes={edit?.showNotes ?? true}
			/>
		</div>
	</div>
{:else}
	<!-- A bracket over the group rather than a box around it: only the label takes clicks, so the
     group's empty area stays part of the canvas. -->
	<div class="relative pointer-events-none" style="width: {data.width}px; height: {data.height}px;">
		<div
			class={twMerge(
				'absolute inset-x-0 h-2.5 rounded-t-lg border-t border-x',
				selected
					? 'border-border-selected'
					: (statusBracket ?? 'border-gray-300 dark:border-gray-600')
			)}
			style="top: {headerTop + SIMPLIFIED.loopHeader + 1}px;"
		></div>
		<StepContextMenu items={menuItems}>
			<div
				class="absolute left-1 max-w-[calc(100%-8px)] inline-flex items-center gap-1.5 px-1.5 text-2xs text-secondary min-w-0 cursor-pointer pointer-events-auto"
				style="top: {headerTop}px; height: {SIMPLIFIED.loopHeader}px;"
				title={[header.title, header.detail].filter(Boolean).join(' · ')}
			>
				{#if header.module}
					<span class="shrink-0"><FlowModuleIcon module={header.module} size={12} /></span>
				{/if}
				<span
					class={twMerge(
						'font-medium truncate shrink-0 max-w-[75%]',
						selected ? 'text-accent' : 'text-emphasis'
					)}
				>
					{header.title}
				</span>
				<span class="font-mono text-primary shrink-0">{header.stepId}</span>
				{#if header.detail}
					<span class="font-mono text-hint truncate min-w-0">{header.detail}</span>
				{/if}
				{#if canAddBranch}
					<Button
						variant="subtle"
						unifiedSize="2xs"
						iconOnly
						startIcon={{ icon: GitBranchPlus }}
						title="Add branch"
						btnClasses="nodrag shrink-0"
						onClick={(e) => {
							e?.stopPropagation()
							edit?.eventHandlers?.newBranch(id)
						}}
					/>
				{/if}
			</div>
		</StepContextMenu>
	</div>
{/if}
