<script lang="ts">
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import Tooltip from './Tooltip.svelte'
	import { AlertTriangle } from 'lucide-svelte'
	import TriggerableByAI from './TriggerableByAI.svelte'

	export let aiId: string | undefined = undefined
	export let aiDescription: string | undefined = undefined
	export let options: {
		left?: string
		leftTooltip?: string
		right?: string
		rightTooltip?: string
		rightDocumentationLink?: string
	} = {}
	export let checked: boolean = false
	export let disabled = false
	export let textClass = ''
	export let textStyle = ''
	export let color: 'blue' | 'red' | 'nord' = 'blue'
	export let id = (Math.random() + 1).toString(36).substring(10)
	export let lightMode: boolean = false
	export let eeOnly: boolean = false

	export let size: 'sm' | 'xs' | '2xs' | '2sm' = 'sm'

	const dispatch = createEventDispatcher<{ change: boolean }>()
	const bothOptions = Boolean(options.left) && Boolean(options.right)

	export let textDisabled = false
</script>

<label
	for={id}
	class="{$$props.class || ''} z-auto flex flex-row items-center duration-50 {disabled
		? 'grayscale opacity-50'
		: 'cursor-pointer'}"
>
	{#if Boolean(options?.left)}
		<span
			class={twMerge(
				'mr-2 font-medium duration-50 select-none',
				bothOptions || textDisabled ? (checked ? 'text-disabled' : 'text-primary') : 'text-primary',
				size === 'xs' || size === '2sm' ? 'text-xs' : size === '2xs' ? 'text-[0.5rem]' : 'text-sm',
				textClass
			)}
			style={textStyle}
		>
			{options?.left}
			{#if options?.leftTooltip}
				<Tooltip light={lightMode}>{options?.leftTooltip}</Tooltip>
			{/if}
		</span>
	{/if}

	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<TriggerableByAI
		id={aiId}
		description={aiDescription}
		onTrigger={() => {
			checked = !checked
			dispatch('change', checked)
		}}
	>
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
					"transition-all bg-surface-selected rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute  after:bg-surface after:border-white after:border after:rounded-full after:transition-all items-center",
					color == 'red'
						? 'peer-checked:bg-red-600'
						: color == 'blue'
							? 'peer-checked:bg-blue-600 dark:peer-checked:bg-blue-500'
							: 'peer-checked:bg-nord-950 dark:peer-checked:bg-nord-400',
					size === 'sm'
						? 'w-11 h-6 after:top-0.5 after:left-[2px] after:h-5 after:w-5'
						: size === '2sm'
							? 'w-9 h-5 after:top-0.5 after:left-[2px] after:h-4 after:w-4'
							: size === '2xs'
								? 'w-5 h-3 after:top-0.5 after:left-[2px] after:h-2 after:w-2'
								: 'w-7 h-4 after:top-0.5 after:left-[2px] after:h-3 after:w-3'
				)}
			></div>
		</div>
	</TriggerableByAI>
	{#if Boolean(options?.right)}
		<span
			class={twMerge(
				'ml-2 font-medium duration-50 select-none',
				bothOptions || textDisabled ? (checked ? 'text-primary' : 'text-disabled') : 'text-primary',
				size === 'xs' || size === '2sm' ? 'text-xs' : 'text-sm',
				textClass
			)}
			style={textStyle}
		>
			{options?.right}
			{#if options?.rightTooltip}
				<Tooltip documentationLink={options.rightDocumentationLink}>
					{options.rightTooltip}
				</Tooltip>
			{/if}
		</span>
	{/if}
	<slot name="right" />
</label>
{#if eeOnly && disabled}
	<span class="inline-flex text-xs items-center gap-1 !text-yellow-500 whitespace-nowrap ml-8">
		<AlertTriangle size={16} />
		EE only <Tooltip>Enterprise Edition only feature</Tooltip>
	</span>
{/if}
