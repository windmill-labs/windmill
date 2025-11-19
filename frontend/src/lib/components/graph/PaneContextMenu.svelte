<script lang="ts">
	import { useSvelteFlow } from '@xyflow/svelte'
	import { StickyNote } from 'lucide-svelte'
	import { getNoteEditorContext } from './noteEditor.svelte'
	import { DEFAULT_NOTE_COLOR } from './noteColors'
	import { fly } from 'svelte/transition'
	import {
		getContextMenuContainerClass,
		CONTEXT_MENU_ITEM_BASE_CLASS,
		CONTEXT_MENU_ITEM_HOVER_CLASS
	} from '../common/contextmenu/contextMenuStyles'
	import { getGraphContext } from './graphContext'

	interface Props {
		editMode?: boolean
	}

	let { editMode = false }: Props = $props()

	const { screenToFlowPosition } = useSvelteFlow()
	const noteEditorContext = getNoteEditorContext()

	const graphContext = getGraphContext()

	let contextMenuVisible = $state(false)
	let contextMenuPosition = $state<{ x: number; y: number }>({ x: 0, y: 0 })
	let pendingFlowPosition = $state<{ x: number; y: number } | null>(null)

	function handlePaneContextMenu(event: MouseEvent) {
		// Only show context menu in edit mode
		if (!editMode || !noteEditorContext?.noteEditor) {
			return
		}

		event.preventDefault()
		event.stopPropagation()

		// Store screen coordinates for context menu positioning
		contextMenuPosition = {
			x: event.clientX,
			y: event.clientY
		}

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
				text: '### Free note\nDouble click to edit me',
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

{#if contextMenuVisible}
	<!-- Context menu -->
	<div
		class="fixed {getContextMenuContainerClass('z-[9999]')}"
		style="left: {contextMenuPosition.x}px; top: {contextMenuPosition.y}px;"
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
		<button
			class="{CONTEXT_MENU_ITEM_BASE_CLASS} {CONTEXT_MENU_ITEM_HOVER_CLASS}"
			onclick={handleAddStickyNote}
			type="button"
		>
			<StickyNote size={14} class="mr-2" />
			<span>Add sticky note</span>
		</button>
	</div>

	<!-- Invisible click catcher to close context menu -->
	<div
		class="fixed inset-0 z-[9998]"
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
