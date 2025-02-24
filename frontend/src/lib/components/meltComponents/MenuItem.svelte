<script lang="ts">
	import { melt } from '@melt-ui/svelte'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import { goto } from '$app/navigation'

	export let href: string | undefined = undefined
	export let disabled: boolean = false
	export let target: string | undefined = undefined
	export let item: MenubarMenuElements['item']

	function handleClick(e: MouseEvent) {
		if (href && !target) {
			e.preventDefault()
			goto(href)
		}
	}
</script>

{#if href}
	<a
		use:melt={$item}
		{href}
		class={$$props.class}
		role="menuitem"
		aria-disabled={disabled}
		tabindex={disabled ? -1 : undefined}
		{target}
		on:click={handleClick}
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
