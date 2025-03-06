<script lang="ts">
	import { melt } from '@melt-ui/svelte'
	import type { MenubarMenuElements } from '@melt-ui/svelte'

	export let href: string | undefined = undefined
	export let disabled: boolean = false
	export let target: string | undefined = undefined
	export let item: MenubarMenuElements['item']
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
