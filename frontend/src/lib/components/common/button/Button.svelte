<script lang="ts">
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

	export let id: string = ''
	export let aiId: string | undefined = undefined
	export let aiDescription: string | undefined = undefined
	export let size: ButtonType.Size = 'md'
	export let spacingSize: ButtonType.Size = size
	export let color: ButtonType.Color | string = 'blue'
	export let variant: ButtonType.Variant = 'contained'
	export let btnClasses: string = ''
	export let wrapperClasses: string = ''
	export let wrapperStyle: string = ''
	export let disabled: boolean = false
	export let href: string | undefined = undefined
	export let target: '_self' | '_blank' | undefined = undefined
	export let iconOnly: boolean = false
	export let loadUntilNav: boolean = false

	export let clickableWhileLoading = false

	export let element: ButtonType.Element | undefined = undefined
	export let nonCaptureEvent: boolean = false
	export let propagateEvent: boolean = false
	export let loading = false
	export let title: string | undefined = undefined
	export let style: string = ''
	export let download: string | undefined = undefined
	export let startIcon: ButtonType.Icon | undefined = undefined
	export let endIcon: ButtonType.Icon | undefined = undefined
	export let shortCut:
		| { key?: string; hide?: boolean; Icon?: any; withoutModifier?: boolean }
		| undefined = undefined
	export let tooltipPopover:
		| {
				placement?: Placement
				openDelay?: number
				closeDelay?: number
				portal?: string
		  }
		| undefined = undefined

	type MenuItem = {
		label: string
		onClick?: (e?: Event) => void
		href?: string
		icon?: any
		disabled?: boolean
	}
	export let dropdownItems: MenuItem[] | (() => MenuItem[]) | undefined = undefined
	export let hideDropdown: boolean = false

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

	$: buttonClass = twMerge(
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

	$: lucideIconSize = (iconMap[size] ?? 12) * 1

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
	$: tooltipPopover && openDelay !== undefined && ($openDelay = tooltipPopover?.openDelay) //This option is reactive

	$: $open !== undefined && dispatchIfMounted('tooltipOpen', $open)
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
				on:pointerdown
				on:focus
				on:blur
				on:mouseenter
				on:mouseleave
				on:click={() => {
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
				{...$$restProps}
				{style}
			>
				{#if loading}
					<Loader2 class={twMerge('animate-spin', iconOnlyPadding[size])} size={lucideIconSize} />
				{:else if startIcon?.icon}
					<svelte:component
						this={startIcon.icon}
						class={twMerge(startIcon?.classes, iconOnlyPadding[size])}
						size={lucideIconSize}
						{...startIcon.props}
					/>
				{/if}

				{#if !iconOnly}
					<slot />
				{/if}
				{#if endIcon?.icon}
					<svelte:component
						this={endIcon.icon}
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
				on:pointerdown
				on:click={onClick}
				on:focus
				on:blur
				on:mouseenter
				on:mouseleave
				class={buttonClass}
				{id}
				tabindex={disabled ? -1 : 0}
				{title}
				{...$$restProps}
				disabled={disabled || (loading && !clickableWhileLoading)}
				{style}
				use:conditionalMelt={trigger}
				{...$trigger}
			>
				{#if loading}
					<Loader2 class={twMerge('animate-spin', iconOnlyPadding[size])} size={lucideIconSize} />
				{:else if startIcon?.icon}
					<svelte:component
						this={startIcon.icon}
						class={twMerge(startIcon?.classes, iconOnlyPadding[size])}
						size={lucideIconSize}
						{...startIcon.props}
					/>
				{/if}

				{#if !iconOnly}
					<slot />
				{/if}
				{#if endIcon?.icon}
					<svelte:component
						this={endIcon.icon}
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
					<slot name="tooltip" />
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
				<svelte:fragment slot="buttonReplacement">
					<div
						class={twMerge(
							buttonClass,
							'rounded-md m-0 p-0 center-center h-full',
							variant === 'border' ? 'border-0 border-r border-y ' : 'border-0',
							'rounded-r-md !rounded-l-none',
							size === 'xs2' ? '!w-8' : '!w-10'
						)}
					>
						<ChevronDown size={lucideIconSize} />
					</div>
				</svelte:fragment>
			</Dropdown>
		{/if}
</div>
