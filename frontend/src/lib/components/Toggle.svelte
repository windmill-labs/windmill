<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { classNames } from '$lib/utils'
	import { createEventDispatcher, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import Tooltip from './Tooltip.svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import { inputBorderClass } from './text_input/TextInput.svelte'
	import EEOnly from './EEOnly.svelte'

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
	const bothOptions = Boolean(untrack(() => options).left) && Boolean(untrack(() => options).right)
	// Same badge the labelled containers (Label, Section, Subsection) show. Gated on
	// `disabled` rather than on the license: a caller can leave an EE-only toggle
	// interactive on CE precisely because it is already on, and "EE only" beside a control
	// that plainly works reads as a lie.
	const showEeBadge = $derived(eeOnly && disabled)
</script>

{#snippet control()}
	<label
		for={id}
		class="{className || ''} z-auto flex flex-row items-center duration-50 {disabled
			? 'grayscale opacity-50 cursor-not-allowed'
			: 'cursor-pointer'}"
		title={options?.title}
	>
		{#if Boolean(options?.left)}
			<span
				class={twMerge(
					'mr-2 font-normal duration-50 select-none',
					bothOptions || textDisabled
						? checked
							? 'text-disabled'
							: 'text-primary'
						: 'text-primary',
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
		<!-- Keeps the sr-only checkbox's absolute box on the toggle; it does not size the knob,
		     which resolves against the track. -->
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
			<!-- Knob offsets resolve against the track's padding box, so the track stays
			     relative. Whole px and 2px of inset: at 1x a track on a fractional x blurs its
			     edge over a whole pixel, which swallows a thinner gap.
			     knob = h - 6, checked translate = w - h. -->
			<div
				class={classNames(
					"relative transition-all bg-surface-sunken rounded-full peer peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-surface after:border-white after:border after:rounded-full after:transition-all items-center",
					color == 'red'
						? 'peer-checked:bg-red-600'
						: color == 'blue'
							? 'peer-checked:bg-blue-400 '
							: 'peer-checked:bg-nord-950 dark:peer-checked:bg-nord-900',
					size === 'md'
						? 'w-[44px] h-[24px] after:h-[18px] after:w-[18px] peer-checked:after:translate-x-[20px]'
						: size === 'sm'
							? 'w-[36px] h-[20px] after:h-[14px] after:w-[14px] peer-checked:after:translate-x-[16px]'
							: size === '2xs'
								? 'w-[20px] h-[12px] after:h-[6px] after:w-[6px] peer-checked:after:translate-x-[8px]'
								: 'w-[28px] h-[16px] after:h-[10px] after:w-[10px] peer-checked:after:translate-x-[12px]',
					inputBorderClass()
				)}
			></div>
		</div>
		{#if Boolean(options?.right)}
			<span
				class={twMerge(
					'ml-2 font-normal duration-50 select-none',
					bothOptions || textDisabled
						? checked
							? 'text-primary'
							: 'text-disabled'
						: 'text-primary',
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
{/snippet}

{#if showEeBadge}
	<!-- The badge sits outside the label: a disabled one is greyed out and half
	     transparent, which would swallow it. -->
	<div class="flex flex-row items-center gap-2">
		{@render control()}
		<EEOnly />
	</div>
{:else}
	{@render control()}
{/if}
