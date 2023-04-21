<script context="module" lang="ts">
	export type ToggleButtonContext = {
		selected: Writable<any>
		select: (value: any) => void
	}
</script>

<script lang="ts">
	import { createEventDispatcher, setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'
	import { TabGroup, TabList } from '@rgossiaux/svelte-headlessui'
	import { twMerge } from 'tailwind-merge'

	export let selected: any

	const dispatch = createEventDispatcher()
	const selectedContent = writable(selected)

	function setSelected(selected: any) {
		selectedContent.set(selected)
	}

	$: setSelected(selected)

	$: $selectedContent && dispatch('selected', $selectedContent)
	setContext<ToggleButtonContext>('ToggleButtonGroup', {
		selected: selectedContent,
		select: (value: any) => {
			selectedContent.set(value)
			selected = value
		}
	})
</script>

<TabGroup class={twMerge('h-8 flex w-full', $$props.class)}>
	<TabList class="flex bg-gray-100 rounded-md p-0.5 gap-1 h-full">
		<slot />
	</TabList>
</TabGroup>
