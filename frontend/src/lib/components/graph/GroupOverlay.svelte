<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { getGroupEditorContext, type FlowGroup } from './groupEditor.svelte'
	import { NoteColor, NOTE_COLORS } from './noteColors'
	import GroupActionBar from './GroupActionBar.svelte'
	import GroupHeader from './GroupHeader.svelte'
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
		const topPadding = 34
		const headerHeight = 22
		const halfHeader = headerHeight / 2
		return {
			x: minX - padding,
			y: minY - topPadding + halfHeader,
			width: maxX - minX + 2 * padding,
			height: maxY - minY + topPadding - halfHeader + padding,
			headerY: minY - topPadding
		}
	}

	function getOutlineColorClass(color?: string, hovered?: boolean): string {
		const config = NOTE_COLORS[(color as NoteColor) ?? NoteColor.BLUE] ?? NOTE_COLORS[NoteColor.BLUE]
		return hovered ? config.outline : config.outlineHover
	}

	function getBgColorClass(color?: string): string {
		return (
			NOTE_COLORS[(color as NoteColor) ?? NoteColor.BLUE]?.backgroundLight ??
			NOTE_COLORS[NoteColor.BLUE].backgroundLight
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
					class="absolute rounded-lg outline outline-1 -outline-offset-1 pointer-events-none transition-colors duration-150 {getOutlineColorClass(
						group.color,
						visibleGroup?.id === group.id
					)} {getBgColorClass(group.color)}"
					style:transform="translate({bounds.x}px, {bounds.y}px)"
					style:width="{bounds.width}px"
					style:height="{bounds.height}px"
				></div>
			</ViewportPortal>

			<!-- Group header + ellipsis menu -->
			<ViewportPortal target="front">
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="absolute flex flex-col items-center"
					style="pointer-events: auto; transform: translate({bounds.x}px, {bounds.headerY}px); width: {bounds.width}px;"
					style:z-index="4"
				>
					<div class="relative" style="width: 275px;">
						<GroupHeader
							summary={group.summary}
							color={group.color}
							collapsed={false}
							{editMode}
							onToggleCollapse={() => toggleCollapse(group.id)}
							onSummaryUpdate={(text) =>
								groupEditorContext?.groupEditor.updateSummary(group.id, text)}
						/>
						{#if editMode}
							<GroupActionBar
								note={group.note}
								color={group.color}
								collapsedByDefault={group.collapsed_by_default ?? false}
								visible
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
