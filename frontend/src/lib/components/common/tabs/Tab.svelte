<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { TabsContext } from './Tabs.svelte'

	export let value: string

	const { selected, update } = getContext<TabsContext>('Tabs')
</script>

<div
	class={classNames(
		value === $selected
			? 'border-b-2 border-blue-700 text-blue-700 text-sm '
			: 'text-sm hover:border-b-2 hover:border-gray-300 text-gray-800',
		' py-2 px-2 cursor-pointer transition-all ease-linear font-medium'
	)}
	on:click={() => update(value)}
>
	<slot />
</div>
