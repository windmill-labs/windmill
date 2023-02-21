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
	import { classNames } from '$lib/utils'

	const dispatch = createEventDispatcher()

	export let selected: string

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

<div class="overflow-x-auto">
	<div
		class={classNames(
			'border-b border-gray-200 flex flex-row whitespace-nowrap  scrollbar-hidden',
			$$props.class
		)}
	>
		<slot />
	</div>
</div>
<slot name="content" />
