<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { ChevronDown, X } from 'lucide-svelte'
	import { getGroupEditorContext, GROUP_HEADER_HEIGHT, type FlowGroup } from './groupEditor.svelte'
	import { NoteColor } from './noteColors'
	import NoteColorPicker from './NoteColorPicker.svelte'
	import Button from '../common/button/Button.svelte'
	import GroupNodeCard from './GroupNodeCard.svelte'

	interface Props {
		hoveredNodeId: string | null
		allNodes: (Node & { type: string })[]
		editMode: boolean
	}

	let { hoveredNodeId, allNodes, editMode }: Props = $props()

	const groupEditorContext = getGroupEditorContext()

	// Color picker open state
	let colorPickerOpen = $state(false)

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
		} else if (!colorPickerOpen && !actionBarHovered) {
			hideTimeout = setTimeout(() => {
				visibleGroup = undefined
			}, 150)
		}
	})

	// All groups for always-visible labels
	let allGroups = $derived(groupEditorContext?.groupEditor.getGroups() ?? [])

	// Compute bounds for each group (with extra top padding for header card)
	function computeGroupBounds(group: FlowGroup) {
		if (group.module_ids.length === 0) return null
		const { minX, minY, maxX, maxY } = calculateNodesBoundsWithOffset(group.module_ids, allNodes)
		const padding = 16
		const topPadding = padding + GROUP_HEADER_HEIGHT
		return {
			x: minX - padding,
			y: minY - topPadding,
			width: maxX - minX + 2 * padding,
			height: maxY - minY + padding + topPadding
		}
	}

	// Border color mapping from NoteColor (same shade as background)
	const GROUP_BORDER_COLORS: Record<NoteColor, string> = {
		[NoteColor.YELLOW]: 'border-yellow-200 dark:border-yellow-900',
		[NoteColor.BLUE]: 'border-blue-100 dark:border-blue-950',
		[NoteColor.GREEN]: 'border-green-200 dark:border-green-900',
		[NoteColor.PURPLE]: 'border-purple-200 dark:border-purple-900',
		[NoteColor.PINK]: 'border-pink-200 dark:border-pink-900',
		[NoteColor.ORANGE]: 'border-orange-200 dark:border-orange-900',
		[NoteColor.RED]: 'border-red-200 dark:border-red-900',
		[NoteColor.CYAN]: 'border-cyan-200 dark:border-cyan-900',
		[NoteColor.LIME]: 'border-lime-200 dark:border-lime-900',
		[NoteColor.GRAY]: 'border-gray-200 dark:border-gray-800'
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
		const current =
			groupEditorContext?.groupEditor.getGroups().find((g) => g.id === groupId)?.collapsed ?? false
		groupEditorContext?.groupEditor.updateCollapsedDefault(groupId, !current)
	}
</script>

{#each allGroups as group (group.id)}
	{#if !group.collapsed}
		{@const bounds = computeGroupBounds(group)}
		{#if bounds}
			<ViewportPortal target="front">
				<!-- Always-visible border (no bg, solid 1px) -->
				<div
					class="absolute rounded-lg border pointer-events-none transition-colors duration-150 {getBorderColorClass(group.color, visibleGroup?.id === group.id)}"
					style:transform="translate({bounds.x}px, {bounds.y}px)"
					style:width="{bounds.width}px"
					style:height="{bounds.height}px"
					style:z-index="4"
				>
					<!-- Full-width header card (inside the border) -->
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
						<GroupNodeCard summary={group.summary} stepCount={group.module_ids.length} color={group.color} fullWidth>
							{#snippet actions()}
								{#if editMode && visibleGroup?.id === group.id}
									<Button
										variant="subtle"
										unifiedSize="xs"
										iconOnly
										title="Collapse group"
										startIcon={{ icon: ChevronDown }}
										onclick={() => toggleCollapse(group.id)}
									/>
									<NoteColorPicker
										selectedColor={(group.color as NoteColor) ?? NoteColor.BLUE}
										onColorChange={(color) => {
											groupEditorContext?.groupEditor.updateColor(group.id, color)
										}}
										bind:isOpen={colorPickerOpen}
									/>
									<Button
										variant="subtle"
										unifiedSize="xs"
										title="Delete group"
										startIcon={{ icon: X }}
										onclick={() => {
											groupEditorContext?.groupEditor.deleteGroup(group.id)
											visibleGroup = undefined
										}}
										iconOnly
										destructive
									/>
								{/if}
							{/snippet}
						</GroupNodeCard>
					</div>
				</div>
			</ViewportPortal>
		{/if}
	{/if}
{/each}
