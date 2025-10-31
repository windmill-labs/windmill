<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import Tooltip from './Tooltip.svelte'
	import { AlertTriangle } from 'lucide-svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import { inputBorderClass } from './text_input/TextInput.svelte'

	interface Props {
		options?: {
			left?: string
			leftTooltip?: string
			right?: string
			rightTooltip?: string
			rightDocumentationLink?: string
			title?: string
		}
		checked?: boolean
		disabled?: boolean
		textClass?: string
		textStyle?: string
		color?: 'blue' | 'red' | 'nord'
		id?: any
		lightMode?: boolean
		eeOnly?: boolean
		aiId?: string | undefined
		aiDescription?: string | undefined
		class?: string | undefined
		size?: '2xs' | 'xs' | 'sm' | 'md'
		textDisabled?: boolean
		right?: import('svelte').Snippet
	}

	let {
		options = {},
		checked = $bindable(),
		disabled = false,
		textClass = '',
		textStyle = '',
		color = 'blue',
		id = (Math.random() + 1).toString(36).substring(10),
		lightMode = false,
		eeOnly = false,
		aiId = undefined,
		aiDescription = undefined,
		class: className = undefined,
		size = 'sm',
		textDisabled = false,
		right
	}: Props = $props()

	const dispatch = createEventDispatcher<{ change: boolean }>()
	const bothOptions = Boolean(options.left) && Boolean(options.right)
</script>

<label
	for={id}
	class="{className || ''} z-auto flex flex-row items-center duration-50 {disabled
		? 'grayscale opacity-50'
		: 'cursor-pointer'}"
	title={options?.title}
>
	{#if Boolean(options?.left)}
		<span
			class={twMerge(
				'mr-2 font-normal duration-50 select-none',
				bothOptions || textDisabled ? (checked ? 'text-disabled' : 'text-primary') : 'text-primary',
				size === '2xs' ? 'text-2xs' : 'text-xs',
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

	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="relative"
		onclick={stopPropagation(bubble('click'))}
		use:triggerableByAI={{
			id: aiId,
			description: aiDescription,
			callback: () => {
				checked = !checked
			}
		}}
	>
		<input
			onfocus={bubble('focus')}
			onclick={bubble('click')}
			{disabled}
			type="checkbox"
			{id}
			class="sr-only peer"
			bind:checked
			onchange={stopPropagation((e) => {
				dispatch('change', !!checked)
			})}
		/>
		<div
			class={classNames(
				"transition-all bg-surface-sunken rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute  after:bg-surface after:border-white after:border after:rounded-full after:transition-all items-center",
				color == 'red'
					? 'peer-checked:bg-red-600'
					: color == 'blue'
						? 'peer-checked:bg-blue-400 '
						: 'peer-checked:bg-nord-950 dark:peer-checked:bg-nord-900',
				size === 'md'
					? 'w-11 h-6 after:top-0.5 after:left-[2px] after:h-5 after:w-5'
					: size === 'sm'
						? 'w-9 h-5 after:top-0.5 after:left-[2px] after:h-4 after:w-4'
						: size === '2xs'
							? 'w-5 h-3 after:top-0.5 after:left-[2px] after:h-2 after:w-2'
							: 'w-7 h-4 after:top-0.5 after:left-[2px] after:h-3 after:w-3',
				inputBorderClass()
			)}
		></div>
	</div>
	{#if Boolean(options?.right)}
		<span
			class={twMerge(
				'ml-2 font-normal duration-50 select-none',
				bothOptions || textDisabled ? (checked ? 'text-primary' : 'text-disabled') : 'text-primary',
				size === '2xs' ? 'text-2xs' : 'text-xs',
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
	{@render right?.()}
</label>
{#if eeOnly && disabled}
	<span class="inline-flex text-xs items-center gap-1 !text-yellow-500 whitespace-nowrap ml-8">
		<AlertTriangle size={16} />
		EE only <Tooltip>Enterprise Edition only feature</Tooltip>
	</span>
{/if}
