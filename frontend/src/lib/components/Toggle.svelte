<script lang="ts">
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import Tooltip from './Tooltip.svelte'

	export let options: {
		left?: string
		right?: string
		rightTooltip?: string
	} = {}
	export let checked: boolean = false
	export let disabled = false
	export let textClass = ''
	export let textStyle = ''
	export let color: 'blue' | 'red' = 'blue'
	export let id = (Math.random() + 1).toString(36).substring(10)

	export let size: 'sm' | 'xs' = 'sm'

	const dispatch = createEventDispatcher()
	const bothOptions = Boolean(options.left) && Boolean(options.right)
</script>

<label
	for={id}
	class="{$$props.class || ''} z-auto inline-flex items-center duration-50 {disabled
		? 'grayscale opacity-50'
		: 'cursor-pointer'}"
>
	{#if Boolean(options?.left)}
		<span
			class={twMerge(
				'mr-2 font-medium duration-50 select-none',
				bothOptions ? (checked ? 'text-disabled' : 'text-primary') : 'text-primary',
				size === 'xs' ? 'text-xs' : 'text-sm',
				textClass
			)}
			style={textStyle}
		>
			{options?.left}
		</span>
	{/if}

	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div class="relative" on:click|stopPropagation>
		<input
			on:focus
			on:click
			{disabled}
			type="checkbox"
			{id}
			class="sr-only peer"
			bind:checked
			on:change|stopPropagation={(e) => {
				dispatch('change', checked)
			}}
		/>
		<div
			class={classNames(
				"transition-all bg-surface-selected rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute  after:bg-surface after:border-white after:border after:rounded-full after:transition-all ",
				color == 'red' ? 'peer-checked:bg-red-600' : 'peer-checked:bg-blue-600',
				size === 'sm'
					? 'w-11 h-6 after:top-0.5 after:left-[2px] after:h-5 after:w-5'
					: 'w-7 h-4 after:top-0.5 after:left-[2px] after:h-3 after:w-3'
			)}
		/>
	</div>
	{#if Boolean(options?.right)}
		<span
			class={twMerge(
				'ml-2 font-medium duration-50 select-none',
				bothOptions ? (checked ? 'text-primary' : 'text-disabled') : 'text-primary',
				size === 'xs' ? 'text-xs' : 'text-sm',
				textClass
			)}
			style={textStyle}
		>
			{options?.right}
			{#if options?.rightTooltip}
				<Tooltip>{options?.rightTooltip}</Tooltip>
			{/if}
		</span>
	{/if}
	<slot name="right" />
</label>
