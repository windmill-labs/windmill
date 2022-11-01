<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import Button from '../button/Button.svelte'
	import type { ToggleButtonContext } from './ToggleButtonGroup.svelte'

	export let value: string
	export let position: 'left' | 'center' | 'right'

	const { select, selected } = getContext<ToggleButtonContext>('ToggleButtonGroup')
</script>

<Button
	{...$$props}
	on:click={() => select(value)}
	btnClasses={classNames(
		'py-1 px-2 text-sm font-medium border-gray-200 ring-0',
		position === 'left' ? 'rounded-none rounded-l-lg border' : '',
		position === 'center' ? 'rounded-none border-t border-b' : '',
		position === 'right' ? 'rounded-none rounded-r-md !border border-l-0' : ''
	)}
	color={$selected.includes(value) ? 'dark' : 'light'}
	variant="contained"
>
	<slot />
</Button>
