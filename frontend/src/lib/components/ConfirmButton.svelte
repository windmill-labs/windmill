<script lang="ts">
	import { Button } from './common'
	import { Check, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let confirmation: string = 'Are you sure?'
	let firstClick = false
	const dispatch = createEventDispatcher()
</script>

	<div class="p-2 flex flex-row w-full gap-2">
{#if !firstClick}
	<Button
		on:click={() => {
			firstClick = true
		}}><slot /></Button
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
