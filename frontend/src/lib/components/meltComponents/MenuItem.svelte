<script lang="ts">
	import { melt } from '@melt-ui/svelte'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI'

	export let aiId: string | undefined = undefined
	export let aiDescription: string | undefined = undefined
	export let href: string | undefined = undefined
	export let disabled: boolean = false
	export let target: string | undefined = undefined
	export let item: MenubarMenuElements['item']

	let aRef: HTMLAnchorElement | undefined = undefined
	let buttonRef: HTMLButtonElement | undefined = undefined
</script>

{#if href}
	<a
		bind:this={aRef}
		use:melt={$item}
		use:triggerableByAI={{
			id: aiId,
			description: aiDescription,
			callback: () => {
				aRef?.click()
			}
		}}
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
		bind:this={buttonRef}
		on:click
		use:melt={$item}
		use:triggerableByAI={{
			id: aiId,
			description: aiDescription,
			callback: () => {
				buttonRef?.click()
			}
		}}
		{disabled}
		class={$$props.class}
		role="menuitem"
		on:m-focusin
		on:m-focusout
	>
		<slot />
	</button>
{/if}
