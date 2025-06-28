<script lang="ts">
	import { run, createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { createEventDispatcher } from 'svelte'
	import { ButtonType } from './model'
	import { twMerge } from 'tailwind-merge'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import { getModifierKey, type Item } from '$lib/utils'
	import { Loader2, ChevronDown } from 'lucide-svelte'
	import { createTooltip } from '@melt-ui/svelte'
	import type { Placement } from '@floating-ui/core'
	import { conditionalMelt } from '$lib/utils'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import { triggerableByAI } from '$lib/actions/triggerableByAI'

	type MenuItem = {
		label: string
		onClick?: (e?: Event) => void
		href?: string
		icon?: any
		disabled?: boolean
	}
	interface Props {
		id?: string
		aiId?: string | undefined
		aiDescription?: string | undefined
		size?: ButtonType.Size
		spacingSize?: ButtonType.Size
		color?: ButtonType.Color | string
		variant?: ButtonType.Variant
		btnClasses?: string
		wrapperClasses?: string
		wrapperStyle?: string
		disabled?: boolean
		href?: string | undefined
		target?: '_self' | '_blank' | undefined
		iconOnly?: boolean
		loadUntilNav?: boolean
		clickableWhileLoading?: boolean
		element?: ButtonType.Element | undefined
		nonCaptureEvent?: boolean
		propagateEvent?: boolean
		loading?: boolean
		title?: string | undefined
		style?: string
		download?: string | undefined
		startIcon?: ButtonType.Icon | undefined
		endIcon?: ButtonType.Icon | undefined
		shortCut?: { key?: string; hide?: boolean; Icon?: any; withoutModifier?: boolean } | undefined
		tooltipPopover?:
			| {
					placement?: Placement
					openDelay?: number
					closeDelay?: number
					portal?: string
			  }
			| undefined
		dropdownBtnClasses?: string
		dropdownItems?: MenuItem[] | (() => MenuItem[]) | undefined
		hideDropdown?: boolean
		children?: import('svelte').Snippet
		tooltip?: import('svelte').Snippet
		[key: string]: any
	}

	let {
		id = '',
		aiId = undefined,
		aiDescription = undefined,
		size = 'md',
		spacingSize = size,
		color = 'blue',
		variant = 'contained',
		btnClasses = '',
		wrapperClasses = '',
		wrapperStyle = '',
		disabled = false,
		href = undefined,
		target = undefined,
		iconOnly = false,
		loadUntilNav = false,
		clickableWhileLoading = false,
		element = $bindable(undefined),
		nonCaptureEvent = false,
		propagateEvent = false,
		loading = $bindable(false),
		title = undefined,
		style = '',
		download = undefined,
		startIcon = undefined,
		endIcon = undefined,
		shortCut = undefined,
		tooltipPopover = undefined,
		dropdownBtnClasses = '',
		dropdownItems = undefined,
		hideDropdown = false,
		children,
		tooltip,
		...rest
	}: Props = $props()

	function computeDropdowns(menuItems: MenuItem[] | (() => MenuItem[])): Item[] {
		const items = typeof menuItems === 'function' ? menuItems() : menuItems
		return items.map((item) => ({
			displayName: item.label,
			action: item.onClick ? (e) => item.onClick?.(e) : undefined,
			icon: item.icon,
			disabled: item.disabled ?? false,
			href: item.href
		}))
	}

	export function focus() {
		element?.focus({})
	}

	export function click() {
		element?.click()
	}

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)
	// Order of classes: border, border modifier, bg, bg modifier, text, text modifier, everything else

	async function onClick(event: MouseEvent) {
		if (!nonCaptureEvent) {
			event.preventDefault()
			if (!propagateEvent) {
				// by default events are not propagated, added this prop so that we can
				event.stopPropagation()
			}
			dispatch('click', event)
		}
	}

	function getColorClass(color, variant) {
		if (color in ButtonType.ColorVariants) {
			return ButtonType.ColorVariants[color][variant]
		} else {
			return color
		}
	}

	let buttonClass = $derived(
		twMerge(
			'w-full',
			getColorClass(color, variant),
			variant === 'border' ? 'border' : '',
			ButtonType.FontSizeClasses[size],
			ButtonType.SpacingClasses[spacingSize][variant],
			'focus-visible:ring-2 font-semibold',
			dropdownItems && dropdownItems.length > 0 ? 'rounded-l-md h-full' : 'rounded-md',
			'justify-center items-center text-center whitespace-nowrap inline-flex gap-2',
			btnClasses,
			'active:opacity-80 transition-all',
			disabled ? '!bg-surface-disabled !text-tertiary cursor-not-allowed' : '',
			loading ? 'cursor-wait' : ''
		)
	)

	const iconMap = {
		xs3: 12,
		xs2: 14,
		xs: 14,
		sm: 16,
		md: 16,
		lg: 18,
		xl: 18
	}

	const iconOnlyPadding = {
		xs3: 'm-[0.5px] qhd:m-[1px]',
		xs2: 'm-[1px] qhd:m-[1.125px]',
		xs: 'm-[1px] qhd:m-[1.125px]',
		sm: 'm-[2px] qhd:m-[2.25px]',
		md: 'm-[2px] qhd:m-[2.25px]',
		lg: 'm-[5px] qhd:m-[5.625px]',
		xl: 'm-[5px] qhd:m-[5.625px]'
	}

	let lucideIconSize = $derived((iconMap[size] ?? 12) * 1)

	const {
		elements: { trigger, content },
		states: { open },
		options: { openDelay }
	} = tooltipPopover
		? createTooltip({
				positioning: {
					placement: tooltipPopover?.placement
				},
				closeDelay: tooltipPopover?.closeDelay,
				group: true,
				portal: tooltipPopover?.portal
			})
		: {
				elements: { trigger: undefined, content: undefined },
				states: { open: undefined },
				options: { openDelay: undefined }
			}
	run(() => {
		tooltipPopover && openDelay !== undefined && ($openDelay = tooltipPopover?.openDelay)
	}) //This option is reactive

	run(() => {
		$open !== undefined && dispatchIfMounted('tooltipOpen', $open)
	})
</script>

<div
	class={twMerge(
		dropdownItems && dropdownItems.length > 0 && variant === 'contained'
			? ButtonType.ColorVariants[color].divider
			: '',
		wrapperClasses,
		'flex flex-row',
		disabled ? 'divide-text-disabled' : ''
	)}
	style={wrapperStyle}
	data-interactive
	use:triggerableByAI={{
		id: aiId,
		description: aiDescription,
		callback: () => {
			element?.click()
		}
	}}
>
	{#if href && !disabled}
		<a
			bind:this={element}
			onpointerdown={bubble('pointerdown')}
			onfocus={bubble('focus')}
			onblur={bubble('blur')}
			onmouseenter={bubble('mouseenter')}
			onmouseleave={bubble('mouseleave')}
			onclick={() => {
				loading = true
				dispatch('click', event)
				if (!loadUntilNav) {
					loading = false
				}
			}}
			{href}
			{download}
			class={buttonClass}
			{id}
			{target}
			tabindex={disabled ? -1 : 0}
			{...rest}
			{style}
		>
			{#if loading}
				<Loader2 class={twMerge('animate-spin', iconOnlyPadding[size])} size={lucideIconSize} />
			{:else if startIcon?.icon}
				<startIcon.icon
					class={twMerge(startIcon?.classes, iconOnlyPadding[size])}
					size={lucideIconSize}
					{...startIcon.props}
				/>
			{/if}

			{#if !iconOnly}
				{@render children?.()}
			{/if}
			{#if endIcon?.icon}
				<endIcon.icon
					class={twMerge(endIcon?.classes, iconOnlyPadding[size])}
					size={lucideIconSize}
				/>
			{/if}
			{#if shortCut && !shortCut.hide}
				<div class="flex flex-row items-center !text-md opacity-60 gap-0 font-normal">
					{#if shortCut.withoutModifier !== true}{getModifierKey()}{/if}{#if shortCut.Icon}<shortCut.Icon
							class="w-4 h-4"
							size={lucideIconSize}
						/>{:else}{shortCut.key}{/if}
				</div>
			{/if}
		</a>
	{:else}
		<button
			bind:this={element}
			onpointerdown={bubble('pointerdown')}
			onclick={onClick}
			onfocus={bubble('focus')}
			onblur={bubble('blur')}
			onmouseenter={bubble('mouseenter')}
			onmouseleave={bubble('mouseleave')}
			class={buttonClass}
			{id}
			tabindex={disabled ? -1 : 0}
			{title}
			{...rest}
			disabled={disabled || (loading && !clickableWhileLoading)}
			{style}
			use:conditionalMelt={trigger}
			{...$trigger}
		>
			{#if loading}
				<Loader2 class={twMerge('animate-spin', iconOnlyPadding[size])} size={lucideIconSize} />
			{:else if startIcon?.icon}
				<startIcon.icon
					class={twMerge(startIcon?.classes, iconOnlyPadding[size])}
					size={lucideIconSize}
					{...startIcon.props}
				/>
			{/if}

			{#if !iconOnly}
				{@render children?.()}
			{/if}
			{#if endIcon?.icon}
				<endIcon.icon
					class={twMerge(endIcon?.classes, iconOnlyPadding[size])}
					size={lucideIconSize}
				/>
			{/if}
			{#if shortCut && !shortCut.hide}
				{@const Icon = shortCut.Icon}
				<div class="flex flex-row items-center !text-md opacity-60 gap-0 font-normal">
					{#if shortCut.withoutModifier !== true}{getModifierKey()}{/if}{#if shortCut.Icon}<Icon
							size={lucideIconSize}
						/>{:else}{shortCut.key}{/if}
				</div>
			{/if}
		</button>
		{#if tooltipPopover && $open}
			<div use:conditionalMelt={content} {...$content} class="z-[20000]">
				{@render tooltip?.()}
			</div>
		{/if}
	{/if}

	{#if dropdownItems && dropdownItems.length > 0}
		<Dropdown
			aiId={aiId ? `${aiId}-dropdown` : undefined}
			aiDescription={aiDescription ? `${aiDescription} dropdown` : undefined}
			items={computeDropdowns(dropdownItems)}
			class="h-auto w-fit"
			hidePopup={hideDropdown}
			usePointerDownOutside
			on:open={() => dispatch('dropdownOpen', true)}
			on:close={() => dispatch('dropdownOpen', false)}
		>
			{#snippet buttonReplacement()}
				<div
					class={twMerge(
						buttonClass,
						'rounded-md m-0 p-0 center-center h-full',
						variant === 'border' ? 'border-0 border-r border-y ' : 'border-0',
						'rounded-r-md !rounded-l-none',
						size === 'xs2' || size === 'xs' ? '!w-8' : '!w-10',
						dropdownBtnClasses
					)}
				>
					<ChevronDown size={lucideIconSize} />
				</div>
			{/snippet}
		</Dropdown>
	{/if}
</div>
