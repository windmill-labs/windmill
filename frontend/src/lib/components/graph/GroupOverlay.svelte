<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { getGroupEditorContext, type FlowGroup } from './groupEditor.svelte'
	import { NoteColor } from './noteColors'
	import GroupActionBar from './GroupActionBar.svelte'
	import StepCountTab from './StepCountTab.svelte'
	import type { CollapsedSubflowN } from './graphBuilder.svelte'

	interface Props {
		hoveredNodeId: string | null
		allNodes: (Node & { type: string })[]
		editMode: boolean
	}

	let { hoveredNodeId, allNodes, editMode }: Props = $props()

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

	// Compute bounds for each group (no header card when expanded)
	function computeGroupBounds(group: FlowGroup) {
		if (group.module_ids.length === 0) return null
		const { minX, minY, maxX, maxY } = calculateNodesBoundsWithOffset(group.module_ids, allNodes)
		const padding = 16
		return {
			x: minX - padding,
			y: minY - padding,
			width: maxX - minX + 2 * padding,
			height: maxY - minY + 2 * padding
		}
	}

	// Outline color mapping — default uses /60 opacity, hover uses full opacity
	const GROUP_OUTLINE_COLORS: Record<NoteColor, string> = {
		[NoteColor.YELLOW]: 'outline-yellow-400/60 dark:outline-yellow-600/60',
		[NoteColor.BLUE]: 'outline-blue-400/60 dark:outline-blue-600/60',
		[NoteColor.GREEN]: 'outline-green-400/60 dark:outline-green-600/60',
		[NoteColor.PURPLE]: 'outline-purple-400/60 dark:outline-purple-600/60',
		[NoteColor.PINK]: 'outline-pink-400/60 dark:outline-pink-600/60',
		[NoteColor.ORANGE]: 'outline-orange-400/60 dark:outline-orange-600/60',
		[NoteColor.RED]: 'outline-red-400/60 dark:outline-red-600/60',
		[NoteColor.CYAN]: 'outline-cyan-400/60 dark:outline-cyan-600/60',
		[NoteColor.LIME]: 'outline-lime-400/60 dark:outline-lime-600/60',
		[NoteColor.GRAY]: 'outline-gray-400/60 dark:outline-gray-600/60'
	}

	const GROUP_OUTLINE_COLORS_HOVER: Record<NoteColor, string> = {
		[NoteColor.YELLOW]: 'outline-yellow-400 dark:outline-yellow-600',
		[NoteColor.BLUE]: 'outline-blue-400 dark:outline-blue-600',
		[NoteColor.GREEN]: 'outline-green-400 dark:outline-green-600',
		[NoteColor.PURPLE]: 'outline-purple-400 dark:outline-purple-600',
		[NoteColor.PINK]: 'outline-pink-400 dark:outline-pink-600',
		[NoteColor.ORANGE]: 'outline-orange-400 dark:outline-orange-600',
		[NoteColor.RED]: 'outline-red-400 dark:outline-red-600',
		[NoteColor.CYAN]: 'outline-cyan-400 dark:outline-cyan-600',
		[NoteColor.LIME]: 'outline-lime-400 dark:outline-lime-600',
		[NoteColor.GRAY]: 'outline-gray-400 dark:outline-gray-600'
	}

	function getOutlineColorClass(color?: string, hovered?: boolean): string {
		const map = hovered ? GROUP_OUTLINE_COLORS_HOVER : GROUP_OUTLINE_COLORS
		return map[(color as NoteColor) ?? NoteColor.BLUE] ?? map[NoteColor.BLUE]
	}

	const GROUP_BG_COLORS: Record<NoteColor, string> = {
		[NoteColor.YELLOW]: 'bg-yellow-400/5 dark:bg-yellow-600/5',
		[NoteColor.BLUE]: 'bg-blue-400/5 dark:bg-blue-600/5',
		[NoteColor.GREEN]: 'bg-green-400/5 dark:bg-green-600/5',
		[NoteColor.PURPLE]: 'bg-purple-400/5 dark:bg-purple-600/5',
		[NoteColor.PINK]: 'bg-pink-400/5 dark:bg-pink-600/5',
		[NoteColor.ORANGE]: 'bg-orange-400/5 dark:bg-orange-600/5',
		[NoteColor.RED]: 'bg-red-400/5 dark:bg-red-600/5',
		[NoteColor.CYAN]: 'bg-cyan-400/5 dark:bg-cyan-600/5',
		[NoteColor.LIME]: 'bg-lime-400/5 dark:bg-lime-600/5',
		[NoteColor.GRAY]: 'bg-gray-400/5 dark:bg-gray-600/5'
	}

	function getBgColorClass(color?: string): string {
		return (
			GROUP_BG_COLORS[(color as NoteColor) ?? NoteColor.BLUE] ?? GROUP_BG_COLORS[NoteColor.BLUE]
		)
	}

	function toggleCollapse(groupId: string) {
		groupEditorContext?.groupEditor.toggleRuntimeCollapse(groupId)
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
			<!-- Bounding box background + outline (behind nodes) -->
			<ViewportPortal target="back">
				<div
					class="absolute rounded-lg outline outline-1 pointer-events-none transition-colors duration-150 {getOutlineColorClass(
						group.color,
						visibleGroup?.id === group.id
					)} {getBgColorClass(group.color)}"
					style:transform="translate({bounds.x}px, {bounds.y}px)"
					style:width="{bounds.width}px"
					style:height="{bounds.height}px"
				></div>
			</ViewportPortal>

			<!-- StepCountTab + ellipsis menu (above nodes) -->
			<ViewportPortal target="front">
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="absolute"
					style="pointer-events: auto; transform: translate({bounds.x + 16}px, {bounds.y}px);"
					style:z-index="4"
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
					<div class="relative flex items-center">
						<StepCountTab
							stepCount={group.module_ids.length}
							summary={group.summary}
							color={group.color}
							collapsed={false}
							short
							onExpand={() => toggleCollapse(group.id)}
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
								onUpdateCollapsedDefault={(v) =>
									groupEditorContext?.groupEditor.updateCollapsedDefault(group.id, v)}
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
		<ViewportPortal target="back">
			<div
				class="absolute rounded-lg outline outline-1 pointer-events-none outline-blue-400/60 dark:outline-blue-600/60"
				style:transform="translate({bounds.x}px, {bounds.y}px)"
				style:width="{bounds.width}px"
				style:height="{bounds.height}px"
				style:z-index="4"
			></div>
		</ViewportPortal>
	{/if}
{/each}
