<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { GROUP_HEADER_HEIGHT, type FlowGroup } from './groupEditor.svelte'
	import { getGraphContext } from './graphContext'
	import { NoteColor, NOTE_COLORS } from './noteColors'
	import type { GroupHeadN } from './graphBuilder.svelte'

	interface Props {
		allNodes: (Node & { type: string })[]
		groups: FlowGroup[]
		groupDepths: Record<string, number>
	}

	let { allNodes, groups, groupDepths }: Props = $props()

	const graphContext = getGraphContext()

	// Pre-compute bounds for all groups reactively (tracks allNodes measured changes)
	let groupBoundsMap = $derived.by(() => {
		const map: Record<string, { x: number; y: number; width: number; height: number } | null> = {}
		const nodeMap = new Map(allNodes.map((n) => [n.id, n]))
		for (const group of groups) {
			if (graphContext?.groupDisplayState?.isRuntimeCollapsed(group.id)) {
				continue
			}
			const headId = `group:${group.id}`
			const endId = `group:${group.id}-end`
			const headNode = nodeMap.get(headId)
			const endNode = nodeMap.get(endId)

			if (headNode && endNode) {
				const d = headNode.data as GroupHeadN['data']
				const headCenterX = headNode.position.x + (headNode.measured?.width ?? 275) / 2
				const wrapperWidth = d.wrapperWidth ?? 275
				const headHeight = headNode.measured?.height ?? GROUP_HEADER_HEIGHT
				const topY = headNode.position.y + headHeight / 2
				map[group.id] = {
					x: headCenterX - wrapperWidth / 2,
					y: topY,
					width: wrapperWidth,
					height: endNode.position.y - topY
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

	const moveManager = graphContext?.moveManager

	function isGroupDragged(groupId: string): boolean {
		if (!moveManager) return false
		return (
			moveManager.draggedNodeIds.has(`group:${groupId}`) ||
			moveManager.draggedNodeIds.has(`collapsed-group:${groupId}`)
		)
	}
</script>

{#each groups as group (group.id)}
	{@const bounds = groupBoundsMap[group.id]}
	{#if bounds}
		<ViewportPortal target="back">
			<div
				class="absolute rounded-lg outline outline-1 -outline-offset-1 pointer-events-none {getOutlineColorClass(
					group.color
				)} {getBgColorClass(group.color)}"
				class:opacity-30={isGroupDragged(group.id)}
				style:transform="translate({bounds.x}px, {bounds.y}px)"
				style:width="{bounds.width}px"
				style:height="{bounds.height}px"
				style:z-index={-10 + (groupDepths[group.id] ?? 0)}
			></div>
		</ViewportPortal>
	{/if}
{/each}
