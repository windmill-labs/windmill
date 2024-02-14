<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { ButtonType } from './model'
	import { twMerge } from 'tailwind-merge'
	import ButtonDropdown from './ButtonDropdown.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { classNames } from '$lib/utils'
	import CenteredLoader from './CenteredLoader.svelte'

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

	export let clickableWhileLoading = false

	export let element: ButtonType.Element | undefined = undefined
	export let id: string = ''
	export let nonCaptureEvent: boolean = false
	export let propagateEvent: boolean = false
	export let loading = false
	export let title: string | undefined = undefined
	export let style: string = ''
	export let download: string | undefined = undefined

	export let startIcon: ButtonType.Icon | undefined = undefined
	export let endIcon: ButtonType.Icon | undefined = undefined

	type MenuItem = {
		label: string
		onClick?: () => void
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
	const colorVariants: Record<ButtonType.Color, Record<ButtonType.Variant, string>> = {
		none: {
			border: '',
			contained: '',
			divider: ''
		},

		blue: {
			border:
				'border-frost-500 dark:border-frost-300 hover:border-frost-700 dark:hover:border-frost-400 focus-visible:border-frost-700 bg-surface hover:bg-frost-100 dark:hover:bg-frost-900 focus-visible:bg-frost-100 focus-visible:dark:text-frost-100 dark:focus-visible:bg-frost-900 text-frost-500 dark:text-frost-300 dark:hover:text-frost-400 hover:text-frost-700 focus-visible:text-frost-700 focus-visible:ring-frost-300',
			contained:
				'bg-frost-500 hover:bg-frost-700 focus-visible:bg-frost-700 text-white focus-visible:ring-frost-300',
			divider: 'divide-x divide-frost-600'
		},
		red: {
			border:
				'border-red-600/60 hover:border-red-600 bg-surface hover:bg-red-100 text-red-600 hover:text-red-700 focus-visible:ring-red-300 ',
			contained: 'bg-red-600 hover:bg-red-600 text-white focus-visible:ring-red-300',
			divider: 'divide-x divide-red-700'
		},
		green: {
			border:
				'border-green-600 hover:border-green-700 bg-surface hover:bg-green-100 text-green-600 hover:text-green-700 focus-visible:ring-green-300',
			contained: 'bg-green-600 hover:bg-green-700 text-white focus-visible:ring-green-300',
			divider: 'divide-x divide-green-700'
		},
		dark: {
			border:
				'border-surface-inverse bg-surface hover:bg-surface-hover focus-visible:bg-surface-hover text-primary hover:text-secondary focus-visible:text-secondary focus-visible:ring-surface-selected-inverse',
			contained:
				'bg-surface-inverse hover:bg-surface-inverse-hover focus-visible:bg-surface-hover-inverse text-primary-inverse focus-visible:ring-surface-selected-inverse',
			divider: 'divide-x divide-gray-800 dark:divide-gray-200'
		},
		gray: {
			border:
				'border-gray-600 hover:border-gray-900 focus-visible:border-gray-900 bg-surface hover:bg-gray-200 focus-visible:bg-gray-200 text-gray-800 hover:text-primary focus-visible:text-primary focus-visible:ring-gray-300',
			contained:
				'bg-gray-700/90 hover:bg-gray-900/90 focus-visible:bg-gray-900/90 text-white focus-visible:ring-gray-300',
			divider: 'divide-x divide-gray-700 dark:divide-gray-200'
		},
		light: {
			border:
				'border  bg-surface hover:bg-surface-hover focus-visible:bg-surface-hover text-primary hover:text-secondary focus-visible:text-secondary focus-visible:ring-surface-selected',
			contained:
				'bg-surface border-transparent hover:bg-surface-hover focus-visible:bg-surface-hover text-primary focus-visible:ring-surface-selected',
			divider: 'divide-x divide-gray-200 dark:divide-gray-700'
		}
	}

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
		if (color in colorVariants) {
			return colorVariants[color][variant]
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
		dropdownItems ? 'rounded-l-md h-full' : 'rounded-md',
		'justify-center items-center text-center whitespace-nowrap inline-flex gap-2',
		btnClasses,
		'active:opacity-80 transition-all',
		disabled ? '!bg-surface-disabled !text-tertiary cursor-not-allowed' : '',
		loading ? 'cursor-wait' : ''
	)

	const iconMap = {
		xs: 14,
		sm: 16,
		md: 16,
		lg: 18
	}

	$: lucideIconSize = iconMap[size] ?? 12
</script>

<div
	class="{dropdownItems && variant === 'contained'
		? colorVariants[color].divider
		: ''} {wrapperClasses} flex flex-row"
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
				loading = false
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
				<CenteredLoader class="animate-spin" height={lucideIconSize} width={lucideIconSize} />
			{:else if startIcon?.icon}
				<svelte:component this={startIcon.icon} class={startIcon?.classes} size={lucideIconSize} />
			{/if}

			{#if !iconOnly}
				<slot />
			{/if}
			{#if endIcon?.icon}
				<svelte:component this={endIcon.icon} class={endIcon.classes} size={lucideIconSize} />
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
				<CenteredLoader class="animate-spin" height={lucideIconSize} width={lucideIconSize} />
			{:else if startIcon?.icon}
				<svelte:component this={startIcon.icon} class={startIcon?.classes} size={lucideIconSize} />
			{/if}

			{#if !iconOnly}
				<slot />
			{/if}
			{#if endIcon?.icon}
				<svelte:component this={endIcon.icon} class={endIcon.classes} size={lucideIconSize} />
			{/if}
		</button>
	{/if}

	{#if dropdownItems}
		<div
			class={twMerge(buttonClass, 'rounded-r-md rounded-l-none m-0 p-0 h-auto !w-10 border-l-0')}
		>
			<ButtonDropdown>
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
