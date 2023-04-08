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

	const dispatch = createEventDispatcher()

	export let selected: string
	let c = ''
	export { c as class }
	export let wrapperClass = ''
	export let style = ''
	export let hashNavigation = false
	export let dflt: string | undefined = undefined

	$: selected && updateSelected()

	const selectedStore = writable(selected)

	$: $selectedStore && dispatch('selected', $selectedStore)

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
			if (hash) {
				const id = hash.replace('#', '')
				selectedStore.set(id)
				selected = id
			} else if (dflt) {
				selectedStore.set(dflt)
				selected = dflt
			}
		}
	}
</script>

<svelte:window on:hashchange={hashChange} />

<div class="overflow-x-auto {wrapperClass}">
	<div
		class={twMerge('border-b border-gray-200 flex flex-row whitespace-nowrap  scrollbar-hidden', c)}
		{style}
	>
		<slot {selected} />
	</div>
</div>
<slot name="content" />
