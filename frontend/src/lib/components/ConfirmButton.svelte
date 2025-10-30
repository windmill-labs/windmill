<script lang="ts">
	import { Button } from './common'
	import { Check, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		confirmation?: string;
		children?: import('svelte').Snippet;
	}

	let { confirmation = 'Are you sure?', children }: Props = $props();
	let firstClick = $state(false)
	const dispatch = createEventDispatcher()
</script>

	<div class="p-2 flex flex-row w-full gap-2">
{#if !firstClick}
	<Button
		on:click={() => {
			firstClick = true
		}}>{@render children?.()}</Button
	>
{:else}
		{confirmation}
		<Button
			color="red"
			on:click={() => {
				firstClick = false
				dispatch('click')
			}}><Check /></Button
		>
		<Button
			on:click={() => {
				firstClick = false
			}}><X /></Button
		>
{/if}
	</div>
