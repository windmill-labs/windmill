<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { ChevronDown, ChevronRight, Ellipsis, Trash2 } from 'lucide-svelte'
	import { getGroupEditorContext, type FlowGroup } from './groupEditor.svelte'
	import { NoteColor } from './noteColors'
	import NoteColorPicker from './NoteColorPicker.svelte'
	import Popover from '../meltComponents/Popover.svelte'
	import Toggle from '../Toggle.svelte'
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

	// Popover open state
	let popoverOpen = $state(false)

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
		} else if (!popoverOpen) {
			hideTimeout = setTimeout(() => {
				visibleGroup = undefined
			}, 150)
		}
	})

	// Compute bounds for the visible group
	let bounds = $derived.by(() => {
		if (!visibleGroup || visibleGroup.module_ids.length === 0) return null

		const { minX, minY, maxX, maxY } = calculateNodesBoundsWithOffset(
			visibleGroup.module_ids,
			allNodes
		)

		const padding = 16
		return {
			x: minX - padding,
			y: minY - padding,
			width: maxX - minX + 2 * padding,
			height: maxY - minY + 2 * padding
		}
	})

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

	const GROUP_BG_COLORS: Record<NoteColor, string> = {
		[NoteColor.YELLOW]: 'bg-yellow-200/10 dark:bg-yellow-800/10',
		[NoteColor.BLUE]: 'bg-blue-200/10 dark:bg-blue-800/10',
		[NoteColor.GREEN]: 'bg-green-200/10 dark:bg-green-800/10',
		[NoteColor.PURPLE]: 'bg-purple-200/10 dark:bg-purple-800/10',
		[NoteColor.PINK]: 'bg-pink-200/10 dark:bg-pink-800/10',
		[NoteColor.ORANGE]: 'bg-orange-200/10 dark:bg-orange-800/10',
		[NoteColor.RED]: 'bg-red-200/10 dark:bg-red-800/10',
		[NoteColor.CYAN]: 'bg-cyan-200/10 dark:bg-cyan-800/10',
		[NoteColor.LIME]: 'bg-lime-200/10 dark:bg-lime-800/10',
		[NoteColor.GRAY]: 'bg-gray-200/10 dark:bg-gray-800/10'
	}

	function getBorderColorClass(color?: string): string {
		return GROUP_BORDER_COLORS[(color as NoteColor) ?? NoteColor.BLUE] ?? GROUP_BORDER_COLORS[NoteColor.BLUE]
	}

	function getBgColorClass(color?: string): string {
		return GROUP_BG_COLORS[(color as NoteColor) ?? NoteColor.BLUE] ?? GROUP_BG_COLORS[NoteColor.BLUE]
	}

	function isCollapsed(groupId: string): boolean {
		return collapsedState[groupId] ?? false
	}

	function toggleCollapse(groupId: string) {
		collapsedState[groupId] = !isCollapsed(groupId)
	}

	// Inline summary editing
	let editingSummary = $state(false)
	let summaryInput = $state('')

	function startEditSummary(group: FlowGroup) {
		editingSummary = true
		summaryInput = group.summary ?? ''
	}

	function commitSummary(groupId: string) {
		editingSummary = false
		groupEditorContext?.groupEditor.updateSummary(groupId, summaryInput)
	}
</script>

{#if visibleGroup && bounds}
	{@const group = visibleGroup}
	{@const currentBounds = bounds}
	<ViewportPortal target="front">
		<div
			class="absolute rounded-lg border-2 border-dashed pointer-events-none {getBorderColorClass(
				group.color
			)} {getBgColorClass(group.color)}"
			style:transform="translate({currentBounds.x}px, {currentBounds.y}px)"
			style:width="{currentBounds.width}px"
			style:height="{currentBounds.height}px"
			style:z-index="5"
		>
			<!-- Action bar at top-right -->
			{#if editMode}
				<div
					class="absolute -top-8 right-0 flex items-center gap-1 bg-surface rounded-t-md px-1 py-0.5 shadow-sm border border-b-0 {getBorderColorClass(
						group.color
					)}"
					style="pointer-events: auto;"
				>
					<!-- Summary label -->
					{#if editingSummary}
						<input
							class="text-xs bg-transparent border-none outline-none text-primary px-1 w-24"
							bind:value={summaryInput}
							onblur={() => commitSummary(group.id)}
							onkeydown={(e) => {
								if (e.key === 'Enter') commitSummary(group.id)
								if (e.key === 'Escape') {
									editingSummary = false
								}
							}}
						/>
					{:else}
						<button
							class="text-xs text-secondary hover:text-primary px-1 truncate max-w-32"
							onclick={() => startEditSummary(group)}
							title="Click to edit group name"
						>
							{group.summary || 'Group'}
						</button>
					{/if}

					<!-- Collapse toggle button -->
					<Button
						variant="subtle"
						unifiedSize="xs"
						iconOnly
						title={isCollapsed(group.id) ? 'Expand group' : 'Collapse group'}
						startIcon={{ icon: isCollapsed(group.id) ? ChevronRight : ChevronDown }}
						onclick={() => toggleCollapse(group.id)}
					/>

					<!-- Settings popover -->
					<Popover
						placement="bottom-end"
						contentClasses="p-3"
						floatingConfig={{ strategy: 'absolute' }}
						usePointerDownOutside
						bind:isOpen={popoverOpen}
					>
						{#snippet trigger()}
							<Button
								variant="subtle"
								unifiedSize="xs"
								selected={popoverOpen}
								nonCaptureEvent
								title="Group settings"
								startIcon={{ icon: Ellipsis }}
								iconOnly
							/>
						{/snippet}
						{#snippet content()}
							<div class="flex flex-col gap-3 min-w-48">
								<Toggle
									checked={group.collapsed ?? false}
									options={{ right: 'Collapse by default' }}
									size="xs"
									on:change={(e) => {
										groupEditorContext?.groupEditor.updateCollapsedDefault(
											group.id,
											e.detail
										)
									}}
								/>
								<div class="flex items-center justify-between">
									<span class="text-xs text-secondary">Color</span>
									<NoteColorPicker
										selectedColor={(group.color as NoteColor) ?? NoteColor.BLUE}
										onColorChange={(color) => {
											groupEditorContext?.groupEditor.updateColor(group.id, color)
										}}
									/>
								</div>
								<Button
									variant="border"
									color="red"
									unifiedSize="xs"
									startIcon={{ icon: Trash2 }}
									onclick={() => {
										groupEditorContext?.groupEditor.deleteGroup(group.id)
										popoverOpen = false
										visibleGroup = undefined
									}}
								>
									Delete group
								</Button>
							</div>
						{/snippet}
					</Popover>
				</div>
			{/if}
		</div>
	</ViewportPortal>
{/if}
