<!--
@component
One clickable segment of the editor breadcrumb. The segment opens a popover
containing a `WorkspaceItemDrillPicker`. Each instance owns its own `isOpen`
state — switching popovers relies on melt-ui's `closeOnOtherPopoverOpen` to
close siblings.
-->
<script lang="ts">
	import BreadcrumbItemContent from './BreadcrumbItemContent.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import WorkspaceItemDrillPicker, {
		type Scope
	} from '$lib/components/WorkspaceItemDrillPicker.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'

	interface Props {
		label: string
		/** Leading glyph, e.g. the folder or user a path scope names. */
		icon?: import('svelte').Snippet
		extraClass?: string
		disabled?: boolean
		/** Where the picker lands when this segment is clicked. */
		initialScope?: Scope
		/** Composite key of the row to highlight (e.g. `dir:flow:f/demo`). */
		initialHighlight?: string
		/** Mark this segment as the current page (leaf of the breadcrumb). */
		isCurrent?: boolean
		currentItem?: WorkspaceItem & { savedPath?: string }
		onPick: (item: WorkspaceItem) => void
		/** Load the picker's items from this workspace (session editors pass their
		 * acting workspace); falls back to $workspaceStore inside the picker. */
		workspaceId?: string
	}

	let {
		label,
		icon,
		extraClass = '',
		disabled = false,
		initialScope,
		initialHighlight,
		isCurrent = false,
		currentItem,
		onPick,
		workspaceId
	}: Props = $props()

	let isOpen = $state(false)

	const SEGMENT_BASE_CLASS =
		'font-normal inline-flex items-center px-1 rounded hover:bg-surface-hover hover:text-primary transition-colors'

	function handlePick(item: WorkspaceItem) {
		isOpen = false
		onPick(item)
	}
</script>

<Popover
	placement="bottom-start"
	usePointerDownOutside
	excludeSelectors=".drawer"
	disableFocusTrap
	closeOnOtherPopoverOpen
	class="{SEGMENT_BASE_CLASS} {extraClass} {disabled ? 'cursor-default' : 'cursor-pointer'}"
	bind:isOpen
	openFocus="[data-workspace-picker-search]"
	triggerAttrs={{
		'aria-label': label,
		...(isCurrent ? { 'aria-current': 'page' } : {})
	}}
	{disabled}
>
	{#snippet trigger()}
		<BreadcrumbItemContent {label} {icon} />
	{/snippet}
	{#snippet content()}
		<WorkspaceItemDrillPicker
			{initialScope}
			{initialHighlight}
			{currentItem}
			{workspaceId}
			onPick={handlePick}
		/>
	{/snippet}
</Popover>
