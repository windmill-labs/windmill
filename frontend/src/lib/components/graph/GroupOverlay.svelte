<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset, expandWithContainerDescendants } from './util'
	import { getGroupEditorContext, GROUP_HEADER_HEIGHT, GROUP_TOP_MARGIN, type FlowGroup } from './groupEditor.svelte'
	import { NoteColor, NOTE_COLORS } from './noteColors'
	import GroupActionBar from './GroupActionBar.svelte'
	import GroupHeader from './GroupHeader.svelte'
	import GroupNoteArea from './GroupNoteArea.svelte'
	import type { CollapsedSubflowN } from './graphBuilder.svelte'

	interface Props {
		hoveredNodeId: string | null
		allNodes: (Node & { type: string })[]
		editMode: boolean
		showNotes: boolean
		containerDescendants: Map<string, string[]>
	}

	let { hoveredNodeId, allNodes, editMode, showNotes, containerDescendants }: Props = $props()

	const groupEditorContext = getGroupEditorContext()

	// Track which group's menu is open (only one at a time)
	let menuOpenGroupId = $state<string | undefined>(undefined)

	// Action bar hover state to prevent flicker
	let actionBarHovered = $state(false)

	// Header hover state (overlay headers are outside xyflow, so hoveredNodeId doesn't cover them)
	let headerHoveredGroupId = $state<string | undefined>(undefined)

	// Hide delay to prevent flickering
	let hideTimeout: ReturnType<typeof setTimeout> | undefined = undefined
	let visibleGroup = $state<FlowGroup | undefined>(undefined)

	// Derive the active group from hoveredNodeId
	let activeGroup = $derived.by(() => {
		if (headerHoveredGroupId) {
			return allGroups.find((g) => g.id === headerHoveredGroupId)
		}
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
		} else if (!menuOpenGroupId && !actionBarHovered) {
			hideTimeout = setTimeout(() => {
				visibleGroup = undefined
			}, 150)
		}
	})

	// All groups for always-visible labels
	let allGroups = $derived(groupEditorContext?.groupEditor.getGroups() ?? [])

	// Note heights tracked by the group editor
	let noteHeights = $derived(groupEditorContext?.groupEditor.getNoteHeights() ?? {})

	function getGroupNoteHeight(groupId: string): number {
		return showNotes ? (noteHeights[groupId] ?? 0) : 0
	}

	// Pre-compute bounds for all groups reactively (tracks allNodes measured changes)
	let groupBoundsMap = $derived.by(() => {
		const map: Record<string, { x: number; y: number; width: number; height: number; headerY: number } | null> = {}
		for (const group of allGroups) {
			if (group.module_ids.length === 0) {
				map[group.id] = null
				continue
			}
			if (groupEditorContext?.groupEditor.isRuntimeCollapsed(group.id)) {
				// Collapsed group bounds
				const nodeId = `collapsed-group:${group.id}`
				const node = allNodes.find((n) => n.id === nodeId)
				if (!node) {
					map[group.id] = null
					continue
				}
				const width = node.measured?.width ?? 275
				const nodeHeight = node.measured?.height ?? 34
				const noteHeight = getGroupNoteHeight(group.id)
				const headerTotal = GROUP_HEADER_HEIGHT + noteHeight
				map[group.id] = {
					x: node.position.x,
					y: node.position.y - noteHeight,
					width,
					height: nodeHeight + noteHeight,
					headerY: node.position.y - headerTotal
				}
			} else {
				// Expanded group bounds — expand module_ids to include container descendants
				const expandedIds = expandWithContainerDescendants(group.module_ids, containerDescendants)
				const { minX, minY, maxX, maxY } = calculateNodesBoundsWithOffset(expandedIds, allNodes)
				const padding = 16
				const noteHeight = getGroupNoteHeight(group.id)
				const topPadding = GROUP_HEADER_HEIGHT + noteHeight + GROUP_TOP_MARGIN
				const halfHeader = GROUP_HEADER_HEIGHT / 2
				// Find the topmost node's center x to symmetrize the bounding box
				let topNodeCenterX = (minX + maxX) / 2
				let topNodeY = Infinity
				for (const id of group.module_ids) {
					const node = allNodes.find((n) => n.id === id)
					if (node && node.position.y < topNodeY) {
						topNodeY = node.position.y
						const w = node.measured?.width ?? 275
						topNodeCenterX = node.position.x + w / 2
					}
				}
				// Symmetrize: extend both sides equally from the top node's center
				const distLeft = topNodeCenterX - minX + padding
				const distRight = maxX - topNodeCenterX + padding
				const halfWidth = Math.max(distLeft, distRight)
				const symX = topNodeCenterX - halfWidth
				const symWidth = halfWidth * 2
				map[group.id] = {
					x: symX,
					y: minY - topPadding + halfHeader,
					width: symWidth,
					height: maxY - minY + topPadding - halfHeader + padding,
					headerY: minY - topPadding
				}
			}
		}
		return map
	})

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

	/** Derive border classes from the header background classes (bg-* → border-*) */
	function getHeaderBorderClass(color?: string): string {
		const config = NOTE_COLORS[(color as NoteColor) ?? NoteColor.BLUE] ?? NOTE_COLORS[NoteColor.BLUE]
		return config.background.replace(/\bbg-/g, 'border-')
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
	{#if groupEditorContext?.groupEditor.isRuntimeCollapsed(group.id)}
		{@const bounds = groupBoundsMap[group.id]}
		{#if bounds}
			<!-- Collapsed: bounding box background + border on sides/bottom only (behind nodes) -->
			<ViewportPortal target="back">
				<div
					class="absolute rounded-b-lg border-x border-b pointer-events-none transition-colors duration-150 {getHeaderBorderClass(
						group.color
					)} {getBgColorClass(group.color)}"
					style:transform="translate({bounds.x}px, {bounds.y}px)"
					style:width="{bounds.width}px"
					style:height="{bounds.height}px"
				></div>
			</ViewportPortal>

			<!-- Collapsed: header + action bar -->
			<ViewportPortal target="front">
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="absolute flex flex-col items-center"
					style="pointer-events: auto; transform: translate({bounds.x}px, {bounds.headerY}px); width: {bounds.width}px;"
					style:z-index="4"
					onmouseenter={() => (headerHoveredGroupId = group.id)}
					onmouseleave={() => (headerHoveredGroupId = undefined)}
				>
					<div class="relative" style="width: 275px;">
						<GroupHeader
							summary={group.summary}
							color={group.color}
							collapsed={true}
							{editMode}
							onToggleCollapse={() => toggleCollapse(group.id)}
							onSummaryUpdate={(text) =>
								groupEditorContext?.groupEditor.updateSummary(group.id, text)}
						/>
						{#if showNotes && group.note != null}
							<GroupNoteArea
								note={group.note ?? ''}
								color={group.color}
								collapsed={true}
								{editMode}
								onHeightChange={(h) => groupEditorContext?.groupEditor.setNoteHeight(group.id, h)}
								onNoteUpdate={(text) => groupEditorContext?.groupEditor.updateNote(group.id, text)}
							/>
						{/if}
						{#if editMode}
							<GroupActionBar
								note={group.note}
								color={group.color}
								collapsedByDefault={group.collapsed_by_default ?? false}
								visible={visibleGroup?.id === group.id}
								menuOpen={menuOpenGroupId === group.id}
								onMenuOpenChange={(open) => (menuOpenGroupId = open ? group.id : undefined)}
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
	{:else}
		{@const bounds = groupBoundsMap[group.id]}
		{#if bounds}
			<!-- Uncollapsed: bounding box background + outline (behind nodes) -->
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

			<!-- Uncollapsed: header + action bar -->
			<ViewportPortal target="front">
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="absolute flex flex-col items-center"
					style="pointer-events: auto; transform: translate({bounds.x}px, {bounds.headerY}px); width: {bounds.width}px;"
					style:z-index="4"
					onmouseenter={() => (headerHoveredGroupId = group.id)}
					onmouseleave={() => (headerHoveredGroupId = undefined)}
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
						{#if showNotes && group.note != null}
							<GroupNoteArea
								note={group.note ?? ''}
								color={group.color}
								{editMode}
								onHeightChange={(h) => groupEditorContext?.groupEditor.setNoteHeight(group.id, h)}
								onNoteUpdate={(text) => groupEditorContext?.groupEditor.updateNote(group.id, text)}
							/>
						{/if}
						{#if editMode}
							<GroupActionBar
								note={group.note}
								color={group.color}
								collapsedByDefault={group.collapsed_by_default ?? false}
								visible={visibleGroup?.id === group.id}
								menuOpen={menuOpenGroupId === group.id}
								onMenuOpenChange={(open) => (menuOpenGroupId = open ? group.id : undefined)}
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
