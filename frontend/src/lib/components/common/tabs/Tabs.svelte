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

	$: selected && updateSelected()

	const selectedStore = writable(selected)

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
