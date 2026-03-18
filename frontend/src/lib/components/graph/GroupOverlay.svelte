<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { getGroupEditorContext, GROUP_HEADER_HEIGHT } from './groupEditor.svelte'
	import { getGraphContext } from './graphContext'
	import type { GroupMembership } from './groupDetectionUtils'
	import { NoteColor, NOTE_COLORS } from './noteColors'
	import type { CollapsedSubflowN, GroupHeadN } from './graphBuilder.svelte'

	interface Props {
		allNodes: (Node & { type: string })[]
		showNotes: boolean
		groupMemberships: Map<string, GroupMembership>
	}

	let { allNodes, showNotes, groupMemberships }: Props = $props()

	const groupEditorContext = getGroupEditorContext()
	const graphContext = getGraphContext()

	// All groups for always-visible overlays
	let allGroups = $derived(groupEditorContext?.groupEditor.getGroups() ?? [])

	// Note heights tracked by the group display state
	let noteHeights = $derived(graphContext?.groupDisplayState?.getNoteHeights() ?? {})

	function getGroupNoteHeight(groupId: string): number {
		return showNotes ? (noteHeights[groupId] ?? 0) : 0
	}

	// Pre-compute bounds for all groups reactively (tracks allNodes measured changes)
	let groupBoundsMap = $derived.by(() => {
		const map: Record<
			string,
			{ x: number; y: number; width: number; height: number; headerY: number } | null
		> = {}
		for (const group of allGroups) {
			if (graphContext?.groupDisplayState?.isRuntimeCollapsed(group.id)) {
				continue
			}
			const headId = `group:${group.id}`
			const endId = `group:${group.id}-end`
			const headNode = allNodes.find((n) => n.id === headId)
			const endNode = allNodes.find((n) => n.id === endId)

			if (headNode && endNode) {
				const d = headNode.data as GroupHeadN['data']
				const headCenterX = headNode.position.x + (headNode.measured?.width ?? 275) / 2
				const wrapperWidth = d.wrapperWidth ?? 275
				const headHeight = headNode.measured?.height ?? GROUP_HEADER_HEIGHT
				const noteHeight = getGroupNoteHeight(group.id)
				const topY = headNode.position.y + headHeight / 2
				map[group.id] = {
					x: headCenterX - wrapperWidth / 2,
					y: topY,
					width: wrapperWidth,
					height: endNode.position.y - topY,
					headerY: headNode.position.y - noteHeight
				}
			} else {
				map[group.id] = null
			}
		}
		return map
	})

	function getOutlineColorClass(color?: string): string {
		const config =
			NOTE_COLORS[(color as NoteColor) ?? NoteColor.BLUE] ?? NOTE_COLORS[NoteColor.BLUE]
		return config.outline
	}

	function getBgColorClass(color?: string): string {
		return (
			NOTE_COLORS[(color as NoteColor) ?? NoteColor.BLUE]?.backgroundLight ??
			NOTE_COLORS[NoteColor.BLUE].backgroundLight
		)
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
	{@const bounds = groupBoundsMap[group.id]}
	{#if bounds}
		<ViewportPortal target="back">
			<div
				class="absolute rounded-lg outline outline-1 -outline-offset-1 pointer-events-none {getOutlineColorClass(
					group.color
				)} {getBgColorClass(group.color)}"
				style:transform="translate({bounds.x}px, {bounds.y}px)"
				style:width="{bounds.width}px"
				style:height="{bounds.height}px"
				style:z-index={-10 + (groupMemberships.get(group.id)?.depth ?? 0)}
			></div>
		</ViewportPortal>
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
				style:z-index="-5"
			></div>
		</ViewportPortal>
	{/if}
{/each}
