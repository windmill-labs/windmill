<script context="module" lang="ts">
	export type TabsContext = {
		selected: Writable<string>
		update: (value: string) => void
		hashNavigation: boolean
	}
</script>

<script lang="ts">
	import { setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import { getTabStateContext, type TabsState } from './tabsState.svelte'

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	export let selected: string
	export let hideTabs = false

	let c = ''
	export { c as class }
	export let wrapperClass = ''
	export let style = ''
	export let hashNavigation = false
	export let values: string[] | undefined = undefined
	export let id: string | undefined = undefined // if provided, the tabs state will be stored in a parent context

	$: selected && updateSelected()

	const selectedStore = writable(selected)

	let tabsState: TabsState | undefined = undefined

	$: $selectedStore && dispatchIfMounted('selected', $selectedStore)

	$: hashValues = values ? values.map((x) => '#' + x) : undefined
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

	if (id) {
		tabsState = getTabStateContext()
		console.log('dbg tabsState', tabsState)
		const tabState = tabsState?.getSelected(id)
		if (tabState) {
			selected = tabState
		}
	}
</script>

<svelte:window on:hashchange={hashChange} />
{#if !hideTabs}
	<div class="overflow-x-auto {wrapperClass}">
		<div class={twMerge('border-b flex flex-row whitespace-nowrap scrollbar-hidden', c)} {style}>
			<slot {selected} />
		</div>
	</div>
{/if}
<slot name="content" />
