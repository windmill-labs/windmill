<script context="module" lang="ts">
	export type TabsContext = {
		selected: Writable<string>
		update: (value: string) => void
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
	export let id = ''

	$: selected && updateSelected()

	const selectedStore = writable(selected)

	$: $selectedStore && dispatch('selected', $selectedStore)

	setContext<TabsContext>('Tabs', {
		selected: selectedStore,
		update: (value: string) => {
			selectedStore.set(value)
			selected = value
		}
	})

	function updateSelected() {
		selectedStore.set(selected)
	}
</script>

<div class="overflow-x-auto {wrapperClass}">
	<div
		class={twMerge('border-b border-gray-200 flex flex-row whitespace-nowrap  scrollbar-hidden', c)}
		{style}
		{id}
	>
		<slot {selected} />
	</div>
</div>
<slot name="content" />
