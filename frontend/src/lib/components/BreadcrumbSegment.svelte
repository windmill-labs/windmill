<!--
@component
One clickable segment of the editor breadcrumb. The segment opens a popover
containing a `WorkspaceItemPicker`. Each instance owns its own `isOpen`
state — switching popovers relies on melt-ui's `closeOnOtherPopoverOpen` to
close siblings.
-->
<script lang="ts">
	import { ChevronRight } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import WorkspaceItemPicker, {
		type WorkspaceItem
	} from '$lib/components/WorkspaceItemPicker.svelte'

	interface Props {
		label: string
		withChevron?: boolean
		extraClass?: string
		disabled?: boolean
		initialOpen: string[]
		initialHighlight: string
		currentItem?: WorkspaceItem & { savedPath?: string }
		onPick: (item: WorkspaceItem) => void
	}

	let {
		label,
		withChevron = false,
		extraClass = '',
		disabled = false,
		initialOpen,
		initialHighlight,
		currentItem,
		onPick
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
	{disabled}
>
	{#snippet trigger()}
		{#if withChevron}<ChevronRight size={10} class="shrink-0" /><span class="truncate">{label}</span
			>{:else}{label}{/if}
	{/snippet}
	{#snippet content()}
		<WorkspaceItemPicker {initialOpen} {initialHighlight} {currentItem} onPick={handlePick} />
	{/snippet}
</Popover>
