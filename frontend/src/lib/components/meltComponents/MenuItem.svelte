<script lang="ts">
	import { melt } from '@melt-ui/svelte'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import TriggerableByAI from '$lib/components/TriggerableByAI.svelte'
	import { goto } from '$app/navigation'
	import { createEventDispatcher } from 'svelte'

	export let aiId: string | undefined = undefined
	export let aiDescription: string | undefined = undefined
	export let href: string | undefined = undefined
	export let disabled: boolean = false
	export let target: string | undefined = undefined
	export let item: MenubarMenuElements['item']

	const dispatch = createEventDispatcher()
</script>

<TriggerableByAI
	id={aiId}
	description={aiDescription}
	onTrigger={() => {
		if (href) {
			goto(href)
		} else {
			dispatch('click')
		}
	}}
>
	{#if href}
		<a
			use:melt={$item}
			{href}
			class={$$props.class}
			role="menuitem"
			aria-disabled={disabled}
			tabindex={disabled ? -1 : undefined}
			{target}
			on:m-focusin
			on:m-focusout
		>
			<slot />
		</a>
	{:else}
		<button
			on:click
			use:melt={$item}
			{disabled}
			class={$$props.class}
			role="menuitem"
			on:m-focusin
			on:m-focusout
		>
			<slot />
		</button>
	{/if}
</TriggerableByAI>
