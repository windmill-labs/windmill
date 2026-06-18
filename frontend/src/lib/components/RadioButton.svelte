<script lang="ts">

	import { createEventDispatcher } from 'svelte'
	import Tooltip from './Tooltip.svelte'
	interface Props {
		label?: string;
		options: [string | { title: string; desc: string }, any][];
		value: any;
		disabled?: boolean;
		labelClass?: string;
		inputClass?: string;
	}

	let {
		label = '',
		options,
		value = $bindable(),
		disabled = false,
		labelClass = '',
		inputClass = ''
	}: Props = $props();

	const dispatch = createEventDispatcher()
</script>

<fieldset class="w-full">
	<legend class="sr-only {labelClass}">{label}</legend>
	<div class="flex flex-row flex-wrap gap-2 items-center mb-2 w-full">
		{#each options as [label, val]}
			<label
				class="text-center text-sm border border-gray-300 h-full rounded-sm cursor-pointer p-2
				grow whitespace-nowrap duration-200 hover:border-gray-600 hover:bg-surface-hover
				{val === value ? '!bg-blue-50 !border-blue-500 dark:!bg-frost-900' : ''} {inputClass}"
			>
				<input
					{disabled}
					type="radio"
					value={val}
					class="sr-only"
					bind:group={value}
					aria-labelledby="memory-option-0-label"
					onclick={() => dispatch('change', val)}
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
