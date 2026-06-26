<!--
@component
One clickable segment of the session-preview breadcrumb. Mirrors
`BreadcrumbSegment` but opens a `PreviewRouterPicker` (pages + items) instead
of the item-only editor picker, so any segment can route the preview anywhere.
-->
<script lang="ts">
	import { ChevronRight } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import PreviewRouterPicker, { type Scope } from './PreviewRouterPicker.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import type { PreviewTarget } from './previewRouter'

	interface Props {
		label: string
		withChevron?: boolean
		extraClass?: string
		/** Where the picker lands when this segment is clicked. */
		initialScope?: Scope
		/** Composite key of the row to highlight (e.g. `dir:flow:f/demo`). */
		initialHighlight?: string
		/** Mark this segment as the current page (leaf of the breadcrumb). */
		isCurrent?: boolean
		currentItem?: WorkspaceItem & { savedPath?: string }
		onPick: (target: PreviewTarget) => void
	}

	let {
		label,
		withChevron = false,
		extraClass = '',
		initialScope,
		initialHighlight,
		isCurrent = false,
		currentItem,
		onPick
	}: Props = $props()

	let isOpen = $state(false)

	const SEGMENT_BASE_CLASS =
		'font-normal inline-flex items-center px-1 rounded hover:bg-surface-hover hover:text-primary transition-colors cursor-pointer'

	function handlePick(target: PreviewTarget) {
		isOpen = false
		onPick(target)
	}
</script>

<Popover
	placement="bottom-start"
	usePointerDownOutside
	excludeSelectors=".drawer"
	disableFocusTrap
	closeOnOtherPopoverOpen
	class="{SEGMENT_BASE_CLASS} {extraClass}"
	bind:isOpen
	openFocus="[data-workspace-picker-search]"
	triggerAttrs={{
		'aria-label': label,
		...(isCurrent ? { 'aria-current': 'page' } : {})
	}}
>
	{#snippet trigger()}
		{#if withChevron}<ChevronRight size={10} class="shrink-0" /><span class="truncate">{label}</span
			>{:else}{label}{/if}
	{/snippet}
	{#snippet content()}
		<PreviewRouterPicker {initialScope} {initialHighlight} {currentItem} onPick={handlePick} />
	{/snippet}
</Popover>
