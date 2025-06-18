<script lang="ts">
	import { setContext, untrack } from 'svelte'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'
	import type { TabsContext } from '$lib/components/apps/editor/settingsPanel/inputEditor/tabs.svelte'
	import { getTabStateContext, type TabsState } from './tabsState.svelte'

	interface Props {
		selected: string
		hideTabs?: boolean
		class?: string
		wrapperClass?: string
		style?: string
		hashNavigation?: boolean
		values?: string[] | undefined
		children?: import('svelte').Snippet<[any]>
		content?: import('svelte').Snippet
		onSelectedChange?: (value: string) => void
	}

	let {
		selected = $bindable(),
		hideTabs = false,
		class: c = '',
		wrapperClass = '',
		style = '',
		hashNavigation = false,
		values = undefined,
		children,
		content,
		onSelectedChange
	}: Props = $props()

	const selectedStore = writable(selected)

	setContext<TabsContext>('Tabs', {
		selected: selectedStore,
		update: (value: string) => {
			selectedStore.set(value)
			selected = value
		},
		hashNavigation
	})

	function updateSelected() {
		selectedStore.set(selected)
		if (tabsState && id) {
			tabsState.setSelected(id, selected)
		}
	}

	function hashChange() {
		if (hashNavigation) {
			const hash = window.location.hash
			if (hash && hashValues?.includes(hash)) {
				const id = hash.replace('#', '')
				selectedStore.set(id)
				selected = id
			}
		}
	}
	$effect(() => {
		selected && untrack(() => updateSelected())
	})
	$effect(() => {
		$selectedStore && untrack(() => onSelectedChange?.($selectedStore))
	})

	let hashValues = $derived(values ? values.map((x) => '#' + x) : undefined)

	if (id) {
		tabsState = getTabStateContext()
		const tabState = tabsState?.getSelected(id)
		if (tabState) {
			selected = tabState
		}
	}
</script>

<svelte:window onhashchange={hashChange} />
{#if !hideTabs}
	<div class="overflow-x-auto {wrapperClass}">
		<div class={twMerge('border-b flex flex-row whitespace-nowrap scrollbar-hidden', c)} {style}>
			{@render children?.({ selected })}
		</div>
	</div>
{/if}
{@render content?.()}
