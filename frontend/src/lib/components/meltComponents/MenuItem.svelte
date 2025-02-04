<script lang="ts">
	import { melt } from '@melt-ui/svelte'

	export let href: string | undefined = undefined
	export let disabled: boolean = false

	export let item: any
</script>

{#if href}
	<a {href} use:melt={item} {disabled} class={$$props.class}>
		<slot />
	</a>
{:else}
	<button use:melt={item} on:click {disabled} class={$$props.class}>
		<slot />
	</button>
{/if}
