<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

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
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'

	type MenuItem = {
		label: string
		onClick?: (e?: Event) => void
		href?: string
		icon?: any
		disabled?: boolean
		tooltip?: string
	}
	interface Props {
		id?: string
		aiId?: string | undefined
		aiDescription?: string | undefined
		/**
		 * @deprecated Use `unifiedSize` instead
		 */
		size?: ButtonType.Size
		/**
		 * @deprecated Use `unifiedSize` instead
		 */
		spacingSize?: ButtonType.Size
		/**
		 * Unified sizing: sm (28px), md (32px), lg (40px)
		 */
		unifiedSize?: ButtonType.UnifiedSize
		/**
		 * @description Extended size for App editor only
		 */
		extendedSize?: ButtonType.ExtendedSize
		/**
		 * @deprecated Use `variant` instead
		 */
		color?: ButtonType.Color | string
		/**
		 * Button style: accent, accent-secondary, default, subtle
		 */
		variant?: ButtonType.Variant
		/**
		 * Additional classes. Do NOT use for colors/fonts - use for layout only
		 */
		btnClasses?: string
		/**
		 * Wrapper classes. Do NOT use for colors/fonts - use for positioning only
		 */
		wrapperClasses?: string
		/**
		 * Wrapper styles. Avoid unless absolutely necessary
		 */
		wrapperStyle?: string
		disabled?: boolean
		selected?: boolean
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
		destructive?: boolean
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
		onClick?: (e?: Event) => void
		children?: import('svelte').Snippet
		tooltip?: import('svelte').Snippet
		[key: string]: any
		dropdownOpen?: boolean
		dropdownWidth?: number | undefined
	}

	let {
		id = '',
		aiId = undefined,
		aiDescription = undefined,
		size = 'md',
		spacingSize = size,
		unifiedSize = undefined,
		extendedSize = undefined,
		color = 'blue',
		variant = 'default',
		btnClasses = '',
		wrapperClasses = '',
		wrapperStyle = '',
		disabled = false,
		selected = false,
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
		destructive = false,
		shortCut = undefined,
		tooltipPopover = undefined,
		dropdownBtnClasses = '',
		dropdownItems = undefined,
		hideDropdown = false,
		children,
		tooltip,
		onClick,
		dropdownOpen = $bindable(false),
		dropdownWidth = undefined,
		...rest
	}: Props = $props()

	function computeDropdowns(menuItems: MenuItem[] | (() => MenuItem[])): Item[] {
		const items = typeof menuItems === 'function' ? menuItems() : menuItems
		return items.map((item) => ({
			displayName: item.label,
			action: item.onClick ? (e) => item.onClick?.(e) : undefined,
			icon: item.icon,
			disabled: item.disabled ?? false,
			href: item.href,
			tooltip: item.tooltip
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

	async function onclick(event: MouseEvent) {
		if (!nonCaptureEvent) {
			event.preventDefault()
			if (!propagateEvent) {
				// by default events are not propagated, added this prop so that we can
				event.stopPropagation()
			}
			onClick?.(event)
			dispatch('click', event)
		}
	}

	function getStyleClass(color, variant) {
		// Check if using new design system variants
		if (['accent-secondary', 'accent', 'default', 'subtle'].includes(variant)) {
			let style = destructive
				? ButtonType.DestructiveVariantStyles[variant]
				: ButtonType.VariantStyles[variant]
			// For default variant with dropdowns, remove border from button since it's on wrapper
			if (
				variant === 'default' &&
				dropdownItems &&
				((typeof dropdownItems === 'function' && dropdownItems().length > 0) ||
					dropdownItems.length > 0)
			) {
				style = style.replace('border border-border-light', '')
			}
			return style
		}

		// Legacy color-based styling
		if (color in ButtonType.ColorVariants) {
			return ButtonType.ColorVariants[color][variant]
		} else {
			return color
		}
	}

	function getSpacingClass(variant, size, spacingSize, iconOnly, unifiedSize, extendedSize) {
		// Check if using new extended sizing system (for App editor only)
		if (extendedSize) {
			const horizontalPadding = iconOnly
				? ButtonType.ExtendedIconOnlySizingClasses[extendedSize]
				: ButtonType.ExtendedSizingClasses[extendedSize]
			const height = ButtonType.ExtendedHeightClasses[extendedSize]
			return `${horizontalPadding} ${height}`
		}

		// Check if using new unified sizing system
		if (unifiedSize) {
			const horizontalPadding = iconOnly
				? ButtonType.UnifiedIconOnlySizingClasses[unifiedSize]
				: ButtonType.UnifiedSizingClasses[unifiedSize]
			const height = ButtonType.UnifiedHeightClasses[unifiedSize]
			return `${horizontalPadding} ${height}`
		}

		// Check if using new design system variants
		if (iconOnly) {
			return ButtonType.IconOnlyVariantSpacingClasses[spacingSize]
		}
		if (['accent-secondary', 'accent', 'default', 'subtle'].includes(variant)) {
			return ButtonType.VariantSpacingClasses[spacingSize]
		}

		// Legacy spacing
		return ButtonType.SpacingClasses[spacingSize][variant]
	}

	function getDividerClass(color, variant) {
		// Check if using new design system variants
		if (variant === 'default') {
			return 'border border-border-light divide-x divide-border-light'
		} else if (variant === 'accent') {
			return 'divide-x divide-blue-100 dark:divide-blue-200'
		} else if (variant === 'accent-secondary') {
			return 'divide-x divide-deep-blue-400 dark:divide-deep-blue-100'
		} else if (variant === 'subtle') {
			return 'divide-x divide-transparent'
		}

		// Legacy color-based dividers
		if (color in ButtonType.ColorVariants) {
			return ButtonType.ColorVariants[color].divider
		}

		return ''
	}

	let buttonClass = $derived(
		twMerge(
			'w-full',
			getStyleClass(color, variant),
			variant === 'border' ? 'border' : '',
			extendedSize
				? ButtonType.ExtendedFontSizeClasses[extendedSize]
				: ButtonType.FontSizeClasses[size],
			getSpacingClass(variant, size, spacingSize, iconOnly, unifiedSize, extendedSize),
			unifiedSize ? ButtonType.UnifiedFontSizes[unifiedSize] : '',
			'focus-visible:ring-2',
			dropdownItems && dropdownItems.length > 0 ? 'rounded-l-md' : 'rounded-md',
			'justify-center items-center text-center inline-flex gap-2',
			'active:opacity-80 transition-[background-color,opacity] duration-150',
			disabled
				? ['default', 'subtle'].includes(variant)
					? '!text-disabled'
					: '!bg-surface-disabled !text-disabled'
				: '',
			loading ? 'cursor-wait' : '',
			selected && ['default', 'subtle'].includes(variant)
				? '!bg-surface-accent-selected !text-accent !border-border-selected'
				: '',
			btnClasses
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

	let lucideIconSize = $derived(
		extendedSize
			? ButtonType.ExtendedIconSizes[extendedSize]
			: unifiedSize
				? ButtonType.UnifiedIconSizes[unifiedSize]
				: (iconMap[size] ?? 12)
	)

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
	$effect(() => {
		tooltipPopover && openDelay !== undefined && ($openDelay = tooltipPopover?.openDelay)
	}) //This option is reactive

	$effect(() => {
		$open !== undefined && dispatchIfMounted('tooltipOpen', $open)
	})

	const dividerClass = $derived(getDividerClass(color, variant))
</script>

<div
	class={twMerge(
		dropdownItems && dropdownItems.length > 0 ? dividerClass : '',
		'shrink-0',
		wrapperClasses,
		'flex flex-row rounded-md',
		disabled ? 'divide-text-disabled cursor-not-allowed' : ''
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
				<Loader2 class={twMerge('animate-spin shrink-0')} size={lucideIconSize} />
			{:else if startIcon?.icon}
				<startIcon.icon
					class={twMerge('shrink-0', startIcon?.classes)}
					size={lucideIconSize}
					{...startIcon.props}
				/>
			{/if}

			{#if !iconOnly}
				{@render children?.()}
			{/if}
			{#if endIcon?.icon}
				<endIcon.icon class={twMerge('shrink-0', endIcon?.classes)} size={lucideIconSize} />
			{/if}
			{#if shortCut && !shortCut.hide}
				<div
					class={twMerge(
						'flex flex-row items-center !text-md opacity-60 gap-0',
						ButtonType.UnifiedFontSizes[size]
					)}
				>
					{#if shortCut.withoutModifier !== true}{getModifierKey()}{/if}{#if shortCut.Icon}<shortCut.Icon
							class="w-4 h-4 shrink-0"
							size={lucideIconSize}
						/>{:else}{shortCut.key}{/if}
				</div>
			{/if}
		</a>
	{:else}
		<button
			bind:this={element}
			onpointerdown={bubble('pointerdown')}
			{onclick}
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
				<Loader2 class={twMerge('animate-spin shrink-0')} size={lucideIconSize} />
			{:else if startIcon?.icon}
				<startIcon.icon
					class={twMerge('shrink-0', startIcon?.classes)}
					size={lucideIconSize}
					{...startIcon.props}
				/>
			{/if}

			{#if !iconOnly}
				{@render children?.()}
			{/if}
			{#if endIcon?.icon}
				<endIcon.icon class={twMerge('shrink-0', endIcon?.classes)} size={lucideIconSize} />
			{/if}
			{#if shortCut && !shortCut.hide}
				{@const Icon = shortCut.Icon}
				<div
					class={twMerge(
						'flex flex-row items-center !text-md opacity-60 gap-0',
						ButtonType.UnifiedFontSizes[size]
					)}
				>
					{#if shortCut.withoutModifier !== true}{getModifierKey()}{/if}{#if shortCut.Icon}<Icon
							class="shrink-0"
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
			bind:open={dropdownOpen}
			enableFlyTransition
			customWidth={dropdownWidth}
		>
			{#snippet buttonReplacement()}
				<div
					class={twMerge(
						buttonClass,
						'rounded-md m-0 p-0 center-center h-full',
						variant === 'border' ? 'border-0 border-r border-y ' : 'border-0',
						'rounded-r-md !rounded-l-none',
						size === 'xs2' || size === 'xs' || unifiedSize === 'md' || unifiedSize === 'sm'
							? '!w-8'
							: '!w-10',
						dropdownBtnClasses
					)}
				>
					<ChevronDown class="shrink-0" size={lucideIconSize} />
				</div>
			{/snippet}
		</Dropdown>
	{/if}
</div>
