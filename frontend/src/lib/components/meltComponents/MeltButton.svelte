<script lang="ts">
	import { type AnyMeltElement } from '@melt-ui/svelte'
	import { conditionalMelt } from '$lib/utils'

	export let meltElement: AnyMeltElement | undefined = undefined
	export let type: 'button' | 'submit' | 'reset' | null | undefined = undefined
	export let title: string = ''
	export let id: string | undefined = undefined
</script>

<button
	use:conditionalMelt={meltElement}
	class={$$props.class}
	{type}
	{title}
	{id}
	{...$meltElement}
	on:click
>
	<slot />
</button>
