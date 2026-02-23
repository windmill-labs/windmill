<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { ChevronDown, ChevronRight, Pen, X } from 'lucide-svelte'
	import { getGroupEditorContext, type FlowGroup } from './groupEditor.svelte'
	import { NoteColor, NOTE_COLORS } from './noteColors'
	import NoteColorPicker from './NoteColorPicker.svelte'
	import Button from '../common/button/Button.svelte'

	interface Props {
		hoveredNodeId: string | null
		allNodes: (Node & { type: string })[]
		editMode: boolean
	}

	let { hoveredNodeId, allNodes, editMode }: Props = $props()

	const groupEditorContext = getGroupEditorContext()

	// Runtime collapse state, keyed by group ID
	let collapsedState: Record<string, boolean> = $state({})

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

			// Initialize collapse state from persisted value if not already set
			if (activeGroup.id && !(activeGroup.id in collapsedState)) {
				collapsedState[activeGroup.id] = activeGroup.collapsed ?? false
			}
		} else if (!colorPickerOpen && !actionBarHovered && !editingSummary) {
			hideTimeout = setTimeout(() => {
				visibleGroup = undefined
			}, 150)
		}
	})

	// All groups for always-visible labels
	let allGroups = $derived(groupEditorContext?.groupEditor.getGroups() ?? [])

	// Compute bounds for each group (for labels)
	function computeGroupBounds(group: FlowGroup) {
		if (group.module_ids.length === 0) return null
		const { minX, minY, maxX, maxY } = calculateNodesBoundsWithOffset(
			group.module_ids,
			allNodes
		)
		const padding = 16
		return {
			x: minX - padding,
			y: minY - padding,
			width: maxX - minX + 2 * padding,
			height: maxY - minY + 2 * padding
		}
	}

	// Border color mapping from NoteColor
	const GROUP_BORDER_COLORS: Record<NoteColor, string> = {
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

	function getBorderColorClass(color?: string): string {
		return (
			GROUP_BORDER_COLORS[(color as NoteColor) ?? NoteColor.BLUE] ??
			GROUP_BORDER_COLORS[NoteColor.BLUE]
		)
	}

	function getTextColorClass(color?: string): string {
		return (
			NOTE_COLORS[(color as NoteColor) ?? NoteColor.BLUE]?.text ??
			NOTE_COLORS[NoteColor.BLUE].text
		)
	}

	function isCollapsed(groupId: string): boolean {
		return collapsedState[groupId] ?? false
	}

	function toggleCollapse(groupId: string) {
		collapsedState[groupId] = !isCollapsed(groupId)
	}

	// Label hover state (for showing pen button)
	let hoveredLabelGroupId = $state<string | null>(null)

	// Inline summary editing (tracks which group is being edited)
	let editingGroupId = $state<string | null>(null)
	let summaryInput = $state('')

	let editingSummary = $derived(editingGroupId !== null)

	function startEditSummary(group: FlowGroup) {
		editingGroupId = group.id
		summaryInput = group.summary ?? ''
	}

	function commitSummary(groupId: string) {
		editingGroupId = null
		groupEditorContext?.groupEditor.updateSummary(groupId, summaryInput)
	}
</script>

{#each allGroups as group (group.id)}
	{@const bounds = computeGroupBounds(group)}
	{#if bounds}
		<ViewportPortal target="front">
			<!-- Always-visible border (no bg, solid 1px) -->
			<div
				class="absolute rounded-lg border pointer-events-none {getBorderColorClass(group.color)}"
				style:transform="translate({bounds.x}px, {bounds.y}px)"
				style:width="{bounds.width}px"
				style:height="{bounds.height}px"
				style:z-index="4"
			>
				<!-- Label (top-left, above the border) -->
				<div
					class="absolute -top-6 left-0 flex items-center gap-1 h-5"
					style="pointer-events: auto; cursor: default;"
					onpointerenter={() => {
						if (hideTimeout) {
							clearTimeout(hideTimeout)
							hideTimeout = undefined
						}
						hoveredLabelGroupId = group.id
						visibleGroup = group
						if (group.id && !(group.id in collapsedState)) {
							collapsedState[group.id] = group.collapsed ?? false
						}
					}}
					onpointerleave={() => {
						hoveredLabelGroupId = null
						if (!colorPickerOpen && !actionBarHovered && !editingSummary) {
							hideTimeout = setTimeout(() => {
								visibleGroup = undefined
							}, 150)
						}
					}}
				>
					{#if editingGroupId === group.id}
						<input
							class="text-xs font-medium bg-transparent border-none outline-none {getTextColorClass(group.color)} w-24"
							bind:value={summaryInput}
							onblur={() => commitSummary(group.id)}
							onkeydown={(e) => {
								if (e.key === 'Enter') commitSummary(group.id)
								if (e.key === 'Escape') {
									editingGroupId = null
								}
							}}
							autofocus
						/>
					{:else}
						<span class="text-xs font-medium {getTextColorClass(group.color)}">
							{group.summary || 'Group'}
						</span>
						{#if editMode && hoveredLabelGroupId === group.id}
							<button
								class="flex items-center justify-center w-4 h-4 rounded hover:bg-surface-hover {getTextColorClass(group.color)} opacity-60 hover:opacity-100"
								onclick={() => startEditSummary(group)}
								title="Edit group name"
							>
								<Pen size={10} />
							</button>
						{/if}
					{/if}
				</div>

				<!-- Action bar (top-right, hover only) — matches group note style -->
				{#if editMode && visibleGroup?.id === group.id}
					<div
						class="absolute -top-7 right-0 p-1 h-7 group flex justify-end"
						style="pointer-events: auto;"
						onpointerenter={() => {
							actionBarHovered = true
							if (hideTimeout) {
								clearTimeout(hideTimeout)
								hideTimeout = undefined
							}
						}}
						onpointerleave={() => {
							actionBarHovered = false
						}}
					>
						<div class="flex flex-row gap-2 h-fit">
							<Button
								variant="subtle"
								unifiedSize="xs"
								iconOnly
								title={isCollapsed(group.id) ? 'Expand group' : 'Collapse group'}
								startIcon={{ icon: isCollapsed(group.id) ? ChevronRight : ChevronDown }}
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
						</div>
					</div>
				{/if}
			</div>
		</ViewportPortal>
	{/if}
{/each}
