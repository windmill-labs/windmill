<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let options: {
		left?: string
		right?: string
	} = {}
	export let checked: boolean = false
	export let disabled = false
	export let textClass = ''
	export let textStyle = ''
	export let color: 'blue' | 'red' = 'blue'

	export let size: 'sm' | 'xs' = 'sm'
	const id = (Math.random() + 1).toString(36).substring(10)
	const dispatch = createEventDispatcher()
</script>

<span class="{$$props.class} z-auto">
	<label
		for={id}
		class="inline-flex items-center mt-2 duration-200 {disabled
			? 'grayscale opacity-50'
			: 'cursor-pointer'}"
	>
		{#if Boolean(options?.left)}
			<span
				class={twMerge(
					'mr-2 font-medium duration-200',
					disabled ? 'text-gray-500' : 'text-gray-900',
					size === 'xs' ? 'text-xs' : 'text-sm',
					textClass
				)}
				style={textStyle}
			>
				{options?.left}
			</span>
		{/if}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<div class="relative" on:click|stopPropagation>
			<input
				on:focus
				on:click
				{disabled}
				type="checkbox"
				value={false}
				{id}
				class="sr-only peer"
				bind:checked
				on:change|stopPropagation={(e) => {
					dispatch('change', checked)
				}}
			/>
			<div
				class="w-11 h-6 bg-gray-200 rounded-full peer peer-focus:ring-4 peer-focus:ring-blue-300 
				peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] 
				after:absolute after:top-0.5 after:left-[2px] after:bg-white after:border-gray-300 
				after:border after:rounded-full after:h-5 after:w-5 after:transition-all {color == 'red'
					? 'peer-checked:bg-red-600'
					: 'peer-checked:bg-blue-600'}"
			/>
		</div>
		{#if Boolean(options?.right)}
			<span
				class={twMerge(
					'ml-2 font-medium duration-200',
					disabled ? 'text-gray-500' : 'text-gray-900',
					size === 'xs' ? 'text-xs' : 'text-sm',
					textClass
				)}
				style={textStyle}
			>
				{options?.right}
			</span>
		{/if}
	</label>
</span>
