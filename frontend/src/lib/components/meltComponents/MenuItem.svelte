<script lang="ts">
	import { melt } from '@melt-ui/svelte'
	import { createEventDispatcher } from 'svelte'

	export let href: string | undefined = undefined
	export let disabled: boolean = false

	export let item: any

	const dispatch = createEventDispatcher()
</script>

{#if href}
	<a {href} use:melt={item} {disabled} class={$$props.class}>
		<slot />
	</a>
{:else}
	<button use:melt={item} on:click={(e) => dispatch('click', e)} {disabled} class={$$props.class}>
		<slot />
	</button>
{/if}
