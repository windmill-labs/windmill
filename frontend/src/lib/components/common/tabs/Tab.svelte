<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'

	export let value: string

	const { selected, update } = getContext('Tabs')
</script>

<div
	class={classNames(
		value === $selected
			? 'border-b-2 border-gray-900 text-gray-900 text-sm'
			: 'text-sm hover:border-b-2 hover:border-gray-300',
		' py-1 px-4 cursor-pointer transition-all ease-linear  font-medium'
	)}
	on:click={() => update(value)}
>
	<slot />
</div>
