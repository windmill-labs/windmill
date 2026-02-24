<script lang="ts">
	import { ViewportPortal, type Node } from '@xyflow/svelte'
	import { calculateNodesBoundsWithOffset } from './util'
	import { preventDefault, stopPropagation } from 'svelte/legacy'
	import { Minimize2, Settings, Ungroup } from 'lucide-svelte'
	import { getGroupEditorContext, GROUP_HEADER_HEIGHT, type FlowGroup } from './groupEditor.svelte'
	import { NoteColor, NOTE_COLOR_SWATCHES } from './noteColors'
	import Popover from '../meltComponents/Popover.svelte'
	import { Tooltip } from '../meltComponents'
	import Toggle from '../Toggle.svelte'
	import GroupNodeCard from './GroupNodeCard.svelte'

	interface Props {
		hoveredNodeId: string | null
		allNodes: (Node & { type: string })[]
		editMode: boolean
	}

	let { hoveredNodeId, allNodes, editMode }: Props = $props()

	const groupEditorContext = getGroupEditorContext()

	// Settings popover open state
	let settingsOpen = $state(false)

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
		} else if (!settingsOpen && !actionBarHovered) {
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
		groupEditorContext?.groupEditor.toggleRuntimeCollapse(groupId)
	}
</script>

{#each allGroups as group (group.id)}
	{#if !groupEditorContext?.groupEditor.isRuntimeCollapsed(group.id)}
		{@const bounds = computeGroupBounds(group)}
		{#if bounds}
			<ViewportPortal target="front">
				<!-- Always-visible border (no bg, solid 1px) -->
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
						<GroupNodeCard
							summary={group.summary}
							stepCount={group.module_ids.length}
							color={group.color}
							fullWidth
						/>
						{#if editMode && visibleGroup?.id === group.id}
							<div class="absolute -translate-y-[100%] top-2 right-0 h-7 p-1 flex flex-row gap-1">
								<Tooltip>
									<button
										class="center-center text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary p-1 rounded-md"
										onclick={stopPropagation(preventDefault(() => toggleCollapse(group.id)))}
										onpointerdown={stopPropagation(preventDefault(() => {}))}
									>
										<Minimize2 size={12} />
									</button>
									<svelte:fragment slot="text">Collapse group</svelte:fragment>
								</Tooltip>
								<Popover
									placement="bottom"
									contentClasses="p-4"
									floatingConfig={{ strategy: 'absolute' }}
									usePointerDownOutside
									bind:isOpen={settingsOpen}
								>
									{#snippet trigger()}
										<Tooltip>
											<button
												class="center-center text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary p-1 rounded-md"
											>
												<Settings size={12} />
											</button>
											<svelte:fragment slot="text">Group settings</svelte:fragment>
										</Tooltip>
									{/snippet}
									{#snippet content()}
										<div class="grid grid-cols-5 gap-1" style="min-width: 140px">
											{#each Object.values(NoteColor) as color (color)}
												<button
													class="w-6 h-6 rounded-full hover:scale-110 transition-transform duration-100
														{NOTE_COLOR_SWATCHES[color]}
														{(group.color ?? NoteColor.BLUE) === color ? 'ring-2 ring-accent' : 'dark:border-gray-600'}"
													onclick={() =>
														groupEditorContext?.groupEditor.updateColor(group.id, color)}
													title={color.charAt(0).toUpperCase() + color.slice(1)}
												></button>
											{/each}
										</div>
										<div class="border-t mt-2 pt-2">
											<Toggle
												size="xs"
												checked={group.collapsed_by_default ?? false}
												options={{ right: 'Collapsed by default' }}
												on:change={(e) =>
													groupEditorContext?.groupEditor.updateCollapsedDefault(
														group.id,
														e.detail
													)}
											/>
										</div>
									{/snippet}
								</Popover>
								<Tooltip>
									<button
										class="center-center text-secondary shadow-sm bg-surface duration-0 hover:bg-red-400 hover:text-white p-1 s rounded-md"
										onclick={stopPropagation(
											preventDefault(() => {
												groupEditorContext?.groupEditor.deleteGroup(group.id)
												visibleGroup = undefined
											})
										)}
										onpointerdown={stopPropagation(preventDefault(() => {}))}
									>
										<Ungroup size={12} />
									</button>
									<svelte:fragment slot="text">Ungroup</svelte:fragment>
								</Tooltip>
							</div>
						{/if}
					</div>
				</div>
			</ViewportPortal>
		{/if}
	{/if}
{/each}
