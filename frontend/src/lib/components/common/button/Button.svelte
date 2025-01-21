<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { ButtonType } from './model'
	import { twMerge } from 'tailwind-merge'
	import ButtonDropdown from './ButtonDropdown.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { classNames, getModifierKey } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'

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
	export let id: string = ''
	export let nonCaptureEvent: boolean = false
	export let propagateEvent: boolean = false
	export let loading = false
	export let title: string | undefined = undefined
	export let style: string = ''
	export let download: string | undefined = undefined
	export let portalTarget: string | undefined = undefined
	export let startIcon: ButtonType.Icon | undefined = undefined
	export let endIcon: ButtonType.Icon | undefined = undefined
	export let shortCut:
		| { key?: string; hide?: boolean; Icon?: any; withoutModifier?: boolean }
		| undefined = undefined

	type MenuItem = {
		label: string
		onClick?: (e?: Event) => void
		href?: string
		icon?: any
	}
	export let dropdownItems: MenuItem[] | (() => MenuItem[]) | undefined = undefined

	function computeDropdowns(): MenuItem[] | undefined {
		if (typeof dropdownItems === 'function') {
			return dropdownItems()
		} else {
			return dropdownItems
		}
	}

	export function focus() {
		element?.focus({})
	}

	const dispatch = createEventDispatcher()
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
>
	{#if href}
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
		>
			{#if loading}
				<Loader2 class={twMerge('animate-spin', iconOnlyPadding[size])} size={lucideIconSize} />
			{:else if startIcon?.icon}
				<svelte:component
					this={startIcon.icon}
					class={twMerge(startIcon?.classes, iconOnlyPadding[size])}
					size={lucideIconSize}
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
	{/if}

	{#if dropdownItems && dropdownItems.length > 0}
		<div
			class={twMerge(
				buttonClass,
				'rounded-md m-0 p-0 h-auto !w-10',
				variant === 'border' ? 'border-0 border-r border-y ' : 'border-0',
				'rounded-r-md !rounded-l-none'
			)}
		>
			<ButtonDropdown target={portalTarget}>
				<svelte:fragment slot="items">
					{#each computeDropdowns() ?? [] as item}
						<MenuItem on:click={item.onClick} href={item.href}>
							<div
								class={classNames(
									'!text-secondary text-left px-4 py-2 gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold'
								)}
							>
								{#if item.icon}
									<svelte:component this={item.icon} class="w-4 h-4" size={lucideIconSize} />
								{/if}
								{item.label}
							</div>
						</MenuItem>
					{/each}
				</svelte:fragment>
			</ButtonDropdown>
		</div>
	{/if}
</div>
