<script lang="ts">
	import { melt } from '@melt-ui/svelte'

	export let href: string | undefined = undefined
	export let disabled: boolean = false
	export let target: string | undefined = undefined
	export let item: any
</script>

{#if href}
	<a
		use:melt={item}
		{href}
		class={$$props.class}
		role="menuitem"
		aria-disabled={disabled}
		tabindex={disabled ? -1 : undefined}
		{target}
	>
		<slot />
	</a>
{:else}
	<button use:melt={item} on:click {disabled} class={$$props.class} role="menuitem">
		<slot />
	</button>
{/if}
