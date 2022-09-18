<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { ToggleButtonContext } from './ToggleButtonGroup.svelte'

	export let value: string
	export let position: 'left' | 'center' | 'right'

	const { select, selected } = getContext<ToggleButtonContext>('ToggleButtonGroup')
</script>

<button
	type="button"
	on:click={() => select(value)}
	class={classNames(
		'py-1 px-2 text-sm font-medium border-gray-200 ',
		position === 'left' ? 'rounded-l-lg border' : '',
		position === 'center' ? 'border-t border-b' : '',
		position === 'right' ? 'rounded-r-md border border-l-0' : '',
		$selected.includes(value) ? 'text-white bg-gray-900 ' : 'text-gray-900 bg-white '
	)}
>
	<slot />
</button>
