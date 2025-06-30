<script lang="ts">
	import { type AnyMeltElement } from '@melt-ui/svelte'
	import { conditionalMelt } from '$lib/utils'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'

	export let aiId: string | undefined = undefined
	export let aiDescription: string | undefined = undefined
	export let meltElement: AnyMeltElement | undefined = undefined
	export let type: 'button' | 'submit' | 'reset' | null | undefined = undefined
	export let title: string = ''
	export let id: string | undefined = undefined

	let buttonRef: HTMLButtonElement | undefined = undefined
</script>

<button
	bind:this={buttonRef}
	use:conditionalMelt={meltElement}
	use:triggerableByAI={{
		id: aiId,
		description: aiDescription,
		callback: () => {
			buttonRef?.click()
		}
	}}
	class={$$props.class}
	{type}
	{title}
	{id}
	{...$meltElement}
	on:click
>
	<slot />
</button>
