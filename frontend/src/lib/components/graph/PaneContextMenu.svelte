<script lang="ts">
	import { useSvelteFlow } from '@xyflow/svelte'
	import { getOverlayHost } from '$lib/components/common/overlayHost.svelte'
	import ConditionalPortal from '$lib/components/common/drawer/ConditionalPortal.svelte'
	import { StickyNote } from 'lucide-svelte'
	import { getNoteEditorContext } from './noteEditor.svelte'
	import { DEFAULT_NOTE_COLOR } from './noteColors'
	import { fly } from 'svelte/transition'
	import {
		getContextMenuContainerClass,
		CONTEXT_MENU_ITEM_BASE_CLASS,
		CONTEXT_MENU_ITEM_HOVER_CLASS,
		CONTEXT_MENU_ITEM_DISABLED_CLASS
	} from '../common/contextmenu/contextMenuStyles'
	import { getGraphContext } from './graphContext'
	import type { Item } from '$lib/utils'

	interface Props {
		editMode?: boolean
		/** Actions on the current multi-selection; when non-empty they replace "Add sticky note". */
		selectionMenuItems?: Item[]
	}

	let { editMode = false, selectionMenuItems = [] }: Props = $props()

	const { screenToFlowPosition } = useSvelteFlow()
	const noteEditorContext = getNoteEditorContext()

	const graphContext = getGraphContext()

	let contextMenuVisible = $state(false)
	let contextMenuPosition = $state<{ x: number; y: number }>({ x: 0, y: 0 })
	let pendingFlowPosition = $state<{ x: number; y: number } | null>(null)

	// Coordinates come from the pointer event, so they are viewport-relative. Inside a pane
	// the menu is portalled into it and positioned against it, or its click-catcher would
	// blanket the app chrome around it. The portal is what makes the pane the offset parent
	// the coordinates are then rebased onto — this component otherwise renders inside the
	// graph's `overflow-clip` box, which would both displace and clip the menu.
	const overlayHost = getOverlayHost()
	const hostEl = $derived(overlayHost?.el())
	const posClass = $derived(hostEl ? 'absolute' : 'fixed')
	const menuOrigin = $derived.by(() => {
		const rect = contextMenuVisible ? hostEl?.getBoundingClientRect() : undefined
		return { x: rect?.left ?? 0, y: rect?.top ?? 0 }
	})

	// Kept inside the canvas: past its right edge the editor's side panel covers the menu,
	// and past its bottom the items leave the window. A menu that would overflow the bottom
	// opens above the pointer instead.
	let canvasRect = $state<DOMRect | undefined>(undefined)
	let menuWidth = $state(0)
	let menuHeight = $state(0)
	const menuPosition = $derived.by(() => {
		let { x, y } = contextMenuPosition
		if (canvasRect) {
			if (x + menuWidth > canvasRect.right) x = Math.max(canvasRect.left, x - menuWidth)
			if (y + menuHeight > canvasRect.bottom) y = Math.max(canvasRect.top, y - menuHeight)
		}
		return { x: x - menuOrigin.x, y: y - menuOrigin.y }
	})

	function handlePaneContextMenu(event: MouseEvent) {
		// Only show context menu in edit mode
		if (!editMode || (selectionMenuItems.length === 0 && !noteEditorContext?.noteEditor)) {
			return
		}

		event.preventDefault()
		event.stopPropagation()

		// Store screen coordinates for context menu positioning
		contextMenuPosition = {
			x: event.clientX,
			y: event.clientY
		}
		canvasRect = (event.target as Element | null)?.closest('.svelte-flow')?.getBoundingClientRect()

		// Convert to flow coordinates for note placement
		pendingFlowPosition = screenToFlowPosition({
			x: event.clientX,
			y: event.clientY
		})

		contextMenuVisible = true
	}

	function handleAddStickyNote() {
		if (noteEditorContext?.noteEditor && pendingFlowPosition) {
			noteEditorContext.noteEditor.addNote({
				text: '## Note\nDouble click to edit me',
				position: {
					x: pendingFlowPosition.x,
					y: pendingFlowPosition.y - (graphContext?.yOffset || 0)
				},
				size: { width: 300, height: 200 },
				color: DEFAULT_NOTE_COLOR,
				type: 'free',
				locked: false
			})
		}
		contextMenuVisible = false
	}

	// Export the handler to be used by parent
	export function onPaneContextMenu(event: MouseEvent) {
		handlePaneContextMenu(event)
	}
</script>

<ConditionalPortal condition={!!hostEl} target={hostEl} class="contents">
	{#if contextMenuVisible}
		<!-- Context menu -->
		<div
			class="{posClass} {getContextMenuContainerClass('z-[9999]')}"
			style="left: {menuPosition.x}px; top: {menuPosition.y}px;"
			bind:offsetWidth={menuWidth}
			bind:offsetHeight={menuHeight}
			transition:fly={{ duration: 150, y: -10 }}
			role="menu"
			tabindex="-1"
			onclick={(e) => {
				e.stopPropagation()
			}}
			onkeydown={(e) => {
				if (e.key === 'Escape') {
					contextMenuVisible = false
				}
			}}
		>
			{#if selectionMenuItems.length > 0}
				{#each selectionMenuItems as item (item.displayName)}
					<button
						class="{CONTEXT_MENU_ITEM_BASE_CLASS} {CONTEXT_MENU_ITEM_HOVER_CLASS} {item.type ===
						'delete'
							? 'text-red-600 dark:text-red-400 hover:bg-red-500/10 dark:hover:bg-red-900/80'
							: ''} {item.disabled ? CONTEXT_MENU_ITEM_DISABLED_CLASS : ''}"
						disabled={item.disabled}
						onclick={(e) => {
							contextMenuVisible = false
							item.action?.(e)
						}}
						type="button"
					>
						{#if item.icon}
							<item.icon size={14} class="mr-2" />
						{/if}
						<span>{item.displayName}</span>
						{#if item.shortcut}
							<span class="ml-auto pl-4 text-2xs text-hint">{item.shortcut}</span>
						{/if}
					</button>
				{/each}
			{:else}
				<button
					class="{CONTEXT_MENU_ITEM_BASE_CLASS} {CONTEXT_MENU_ITEM_HOVER_CLASS}"
					onclick={handleAddStickyNote}
					type="button"
				>
					<StickyNote size={14} class="mr-2" />
					<span>Add sticky note</span>
				</button>
			{/if}
		</div>

		<!-- Invisible click catcher to close context menu -->
		<div
			class="{posClass} inset-0 z-[9998]"
			role="presentation"
			onclick={() => {
				contextMenuVisible = false
			}}
			oncontextmenu={(e) => {
				e.preventDefault()
				contextMenuVisible = false
			}}
		></div>
	{/if}
</ConditionalPortal>
