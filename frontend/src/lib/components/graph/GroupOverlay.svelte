<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { getGroupEditorContext, GROUP_HEADER_HEIGHT } from './groupEditor.svelte'
	import { getGraphContext } from './graphContext'
	import { NoteColor, NOTE_COLORS } from './noteColors'
	import type { GroupHeadN } from './graphBuilder.svelte'

	interface Props {
		allNodes: (Node & { type: string })[]
	}

	let { allNodes }: Props = $props()

	const groupEditorContext = getGroupEditorContext()
	const graphContext = getGraphContext()

	// All groups for always-visible overlays
	let allGroups = $derived(groupEditorContext?.groupEditor.getGroups() ?? [])

	// Pre-compute bounds for all groups reactively (tracks allNodes measured changes)
	let groupBoundsMap = $derived.by(() => {
		const map: Record<string, { x: number; y: number; width: number; height: number } | null> = {}
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

	// Compute nesting depth from visual bounds — group B is nested inside A
	// if A's vertical extent strictly contains B's
	let groupDepths = $derived.by(() => {
		const depths: Record<string, number> = {}
		for (const group of allGroups) {
			const bounds = groupBoundsMap[group.id]
			if (!bounds) continue
			let depth = 0
			const top = bounds.y
			const bottom = bounds.y + bounds.height
			for (const other of allGroups) {
				if (other.id === group.id) continue
				const ob = groupBoundsMap[other.id]
				if (!ob) continue
				const oTop = ob.y
				const oBottom = ob.y + ob.height
				if (oTop <= top && oBottom >= bottom && (oTop < top || oBottom > bottom)) {
					depth++
				}
			}
			depths[group.id] = depth
		}
		return depths
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

{#each allGroups as group (group.id)}
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
