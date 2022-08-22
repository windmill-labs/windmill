<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	import FlowBoxHeader from './FlowBoxHeader.svelte'
	export let title: string | undefined = undefined
	export let headerClickable = false
	const dispatch = createEventDispatcher()

	let slots = $$props.$$slots
</script>

<div class="bg-white border border-gray-300 rounded-md shadow-md">
	<FlowBoxHeader clickable={headerClickable} on:click={() => dispatch('clickheader')} {title}>
		<slot name="header" />
	</FlowBoxHeader>
	{#if slots.content}
		<slot name="content" />
	{/if}
</div>
