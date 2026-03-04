<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { getGroupEditorContext, GROUP_HEADER_HEIGHT, type FlowGroup } from './groupEditor.svelte'
	import { NoteColor } from './noteColors'
	import GroupNodeCard from './GroupNodeCard.svelte'
	import GroupActionBar from './GroupActionBar.svelte'
	import type { FlowModule } from '$lib/gen'
	import StepCountTab from './StepCountTab.svelte'
	import type { CollapsedSubflowN } from './graphBuilder.svelte'

	interface Props {
		hoveredNodeId: string | null
		allNodes: (Node & { type: string })[]
		editMode: boolean
		showNotes: boolean
	}

	let { hoveredNodeId, allNodes, editMode, showNotes }: Props = $props()

	const groupEditorContext = getGroupEditorContext()

	// Menu open state
	let menuOpen = $state(false)

	// Action bar hover state to prevent flicker
	let actionBarHovered = $state(false)

	// Hide delay to prevent flickering
	let hideTimeout: ReturnType<typeof setTimeout> | undefined = undefined
	let visibleGroup = $state<FlowGroup | undefined>(undefined)

	// Derive the active group from hoveredNodeId
	let activeGroup = $derived.by(() => {
		if (!hoveredNodeId || !groupEditorContext?.groupEditor) return undefined
		return groupEditorContext.groupEditor.getClosestGroup(hoveredNodeId)
	})

	// Manage visible group with delay to prevent flickering
	$effect(() => {
		if (activeGroup) {
			if (hideTimeout) {
				clearTimeout(hideTimeout)
				hideTimeout = undefined
			}
			visibleGroup = activeGroup
		} else if (!menuOpen && !actionBarHovered) {
			hideTimeout = setTimeout(() => {
				visibleGroup = undefined
			}, 150)
		}
	})

	// All groups for always-visible labels
	let allGroups = $derived(groupEditorContext?.groupEditor.getGroups() ?? [])

	// Compute bounds for each group (with extra top padding for header card + note)
	function computeGroupBounds(group: FlowGroup) {
		if (group.module_ids.length === 0) return null
		const { minX, minY, maxX, maxY } = calculateNodesBoundsWithOffset(group.module_ids, allNodes)
		const padding = 16
		const noteHeight = groupEditorContext?.groupEditor.getNoteHeights()[group.id] ?? 0
		const topPadding = padding + GROUP_HEADER_HEIGHT + noteHeight
		return {
			x: minX - padding,
			y: minY - topPadding,
			width: maxX - minX + 2 * padding,
			height: maxY - minY + padding + topPadding
		}
	}

	// Border color mapping — default uses /60 opacity, hover uses full opacity
	const GROUP_BORDER_COLORS: Record<NoteColor, string> = {
		[NoteColor.YELLOW]: 'border-yellow-400/60 dark:border-yellow-600/60',
		[NoteColor.BLUE]: 'border-blue-400/60 dark:border-blue-600/60',
		[NoteColor.GREEN]: 'border-green-400/60 dark:border-green-600/60',
		[NoteColor.PURPLE]: 'border-purple-400/60 dark:border-purple-600/60',
		[NoteColor.PINK]: 'border-pink-400/60 dark:border-pink-600/60',
		[NoteColor.ORANGE]: 'border-orange-400/60 dark:border-orange-600/60',
		[NoteColor.RED]: 'border-red-400/60 dark:border-red-600/60',
		[NoteColor.CYAN]: 'border-cyan-400/60 dark:border-cyan-600/60',
		[NoteColor.LIME]: 'border-lime-400/60 dark:border-lime-600/60',
		[NoteColor.GRAY]: 'border-gray-400/60 dark:border-gray-600/60'
	}

	const GROUP_BORDER_COLORS_HOVER: Record<NoteColor, string> = {
		[NoteColor.YELLOW]: 'border-yellow-400 dark:border-yellow-600',
		[NoteColor.BLUE]: 'border-blue-400 dark:border-blue-600',
		[NoteColor.GREEN]: 'border-green-400 dark:border-green-600',
		[NoteColor.PURPLE]: 'border-purple-400 dark:border-purple-600',
		[NoteColor.PINK]: 'border-pink-400 dark:border-pink-600',
		[NoteColor.ORANGE]: 'border-orange-400 dark:border-orange-600',
		[NoteColor.RED]: 'border-red-400 dark:border-red-600',
		[NoteColor.CYAN]: 'border-cyan-400 dark:border-cyan-600',
		[NoteColor.LIME]: 'border-lime-400 dark:border-lime-600',
		[NoteColor.GRAY]: 'border-gray-400 dark:border-gray-600'
	}

	function getBorderColorClass(color?: string, hovered?: boolean): string {
		const map = hovered ? GROUP_BORDER_COLORS_HOVER : GROUP_BORDER_COLORS
		return map[(color as NoteColor) ?? NoteColor.BLUE] ?? map[NoteColor.BLUE]
	}

	function toggleCollapse(groupId: string) {
		groupEditorContext?.groupEditor.toggleRuntimeCollapse(groupId)
	}

	function getGroupModules(group: FlowGroup): FlowModule[] {
		return group.module_ids
			.map(
				(id) =>
					allNodes.find((n) => n.id === id)?.data?.module as FlowModule | undefined
			)
			.filter((m): m is FlowModule => m != null)
	}

	// Expanded subflows — derive from allNodes
	let expandedSubflowNodes = $derived(
		allNodes.filter(
			(n) =>
				n.type === 'collapsedSubflow' &&
				(n.data as CollapsedSubflowN['data']).expanded &&
				(n.data as CollapsedSubflowN['data']).innerNodeIds?.length
		)
	)

	function computeSubflowBounds(node: Node) {
		const data = node.data as CollapsedSubflowN['data']
		const innerIds = data.innerNodeIds ?? []
		if (innerIds.length === 0) return null
		// Include the header node itself in the bounds
		const allIds = [node.id, ...innerIds]
		const { minX, minY, maxX, maxY } = calculateNodesBoundsWithOffset(allIds, allNodes)
		const padding = 16
		return {
			x: minX - padding,
			y: minY - padding,
			width: maxX - minX + 2 * padding,
			height: maxY - minY + 2 * padding
		}
	}
</script>

{#each allGroups as group (group.id)}
	{#if !groupEditorContext?.groupEditor.isRuntimeCollapsed(group.id)}
		{@const bounds = computeGroupBounds(group)}
		{#if bounds}
			<ViewportPortal target="front">
				<!-- Always-visible border -->
				<div
					class="absolute rounded-lg border pointer-events-none transition-colors duration-150 {getBorderColorClass(
						group.color,
						visibleGroup?.id === group.id
					)}"
					style:transform="translate({bounds.x}px, {bounds.y}px)"
					style:width="{bounds.width}px"
					style:height="{bounds.height}px"
					style:z-index="4"
				>
					<!-- Header card (inside the border) -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="absolute top-0 left-0 right-0"
						style="pointer-events: auto;"
						onpointerenter={() => {
							actionBarHovered = true
							visibleGroup = group
							if (hideTimeout) {
								clearTimeout(hideTimeout)
								hideTimeout = undefined
							}
						}}
						onpointerleave={() => {
							actionBarHovered = false
						}}
					>
						<StepCountTab
							stepCount={group.module_ids.length}
							color={group.color}
							collapsed={false}
							onExpand={() => toggleCollapse(group.id)}
						/>
						<GroupNodeCard
							summary={group.summary}
							stepCount={group.module_ids.length}
							color={group.color}
							note={group.note}
							showNote={showNotes && group.note != null}
							{editMode}
							modules={getGroupModules(group)}
							onExpand={() => toggleCollapse(group.id)}
							onSummaryUpdate={(text) => groupEditorContext?.groupEditor.updateSummary(group.id, text)}
							onNoteUpdate={(text) => groupEditorContext?.groupEditor.updateNote(group.id, text)}
							onHeightChange={(h) => groupEditorContext?.groupEditor.setNoteHeight(group.id, h)}
						/>
						{#if editMode && visibleGroup?.id === group.id}
							<GroupActionBar
								note={group.note}
								color={group.color}
								collapsedByDefault={group.collapsed_by_default ?? false}
								bind:menuOpen
								onAddNote={() => groupEditorContext?.groupEditor.addNote(group.id)}
								onRemoveNote={() => groupEditorContext?.groupEditor.removeNote(group.id)}
								onUpdateColor={(c) => groupEditorContext?.groupEditor.updateColor(group.id, c)}
								onUpdateCollapsedDefault={(v) => groupEditorContext?.groupEditor.updateCollapsedDefault(group.id, v)}
								onDeleteGroup={() => {
									groupEditorContext?.groupEditor.deleteGroup(group.id)
									visibleGroup = undefined
								}}
							/>
						{/if}
					</div>
				</div>
			</ViewportPortal>
		{/if}
	{/if}
{/each}

{#each expandedSubflowNodes as node (node.id)}
	{@const bounds = computeSubflowBounds(node)}
	{#if bounds}
		<ViewportPortal target="front">
			<div
				class="absolute rounded-lg border pointer-events-none border-blue-400/60 dark:border-blue-600/60"
				style:transform="translate({bounds.x}px, {bounds.y}px)"
				style:width="{bounds.width}px"
				style:height="{bounds.height}px"
				style:z-index="4"
			></div>
		</ViewportPortal>
	{/if}
{/each}
