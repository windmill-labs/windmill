<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import Button from '../button/Button.svelte'
	import type { ToggleButtonContext } from './ToggleButtonGroup.svelte'

	export let value: any
	export let position: 'left' | 'center' | 'right'
	export let light = false

	const { select, selected } = getContext<ToggleButtonContext>('ToggleButtonGroup')
</script>

<Button
	{...$$props}
	on:click={() => select(value)}
	btnClasses={classNames(
		'border-gray-200 focus:ring-0 w-full',
		position === 'left' ? 'rounded-none rounded-l-lg border' : '',
		position === 'center' ? 'rounded-none border-t border-b border-r' : '',
		position === 'right' ? 'rounded-none rounded-r-md  border-r border-y' : ''
	)}
	color={$selected === value ? (light ? 'dark' : 'dark') : 'light'}
	variant="contained"
>
	<slot />
</Button>
