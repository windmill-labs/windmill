<script lang="ts">
	export let label = ''
	export let options: [string | { title: string; desc: string }, any][]
	export let value: any

	import { createEventDispatcher } from 'svelte'
	import Tooltip from './Tooltip.svelte'

	const dispatch = createEventDispatcher()
</script>

<fieldset class="w-full">
	<legend class="sr-only">{label}</legend>
	<div class="flex flex-row flex-wrap gap-2 items-center mb-2 w-full">
		{#each options as [label, val]}
			<label
				class:item-button-selected={val == value}
				class="item-button text-center text-sm h-full p-2 grow whitespace-nowrap"
			>
				<input
					type="radio"
					value={val}
					class="sr-only"
					bind:group={value}
					aria-labelledby="memory-option-0-label"
					on:click={() => dispatch('change', val)}
				/>
				<p>
					{#if typeof label !== 'string'}
						{label.title}
						<Tooltip>{label.desc}</Tooltip>
					{:else}{label}{/if}
				</p>
			</label>
		{/each}
	</div>
</fieldset>
