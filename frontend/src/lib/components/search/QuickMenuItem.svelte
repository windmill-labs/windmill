<script lang="ts">
	import { createEventDispatcher, onDestroy, onMount } from 'svelte'

	export let hovered: boolean = false
	export let id: string
	export let label: string = ''
	export let icon: any

	const dispatch = createEventDispatcher()

	onMount(() => {
		window.addEventListener('keydown', handleKeydown)
	})

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown)
	})

	async function handleKeydown(event: KeyboardEvent) {
		if (hovered && event.key === 'Enter') {
			event.preventDefault()
			runAction()
		}
	}

	function runAction() {
		console.log('run' + id)
		dispatch('select')
	}
</script>

<div
	on:click={runAction}
	on:mouseenter={() => dispatch('hover')}
	class={`rounded-md ${hovered ? 'bg-surface-hover' : ''}`}
>
	{#if $$slots.itemReplacement}
		<slot name="itemReplacement" />
	{:else}
		<div class="flex flex-row gap-2 items-center px-1 py-0.5 rounded-md">
			<div class="w-4">
				{#if icon}
					<svelte:component this={icon} size={14} />
				{/if}
			</div>
			{label}
		</div>
	{/if}
</div>
