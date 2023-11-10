<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { ButtonType } from './model'
	import { Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import ButtonDropdown from './ButtonDropdown.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { classNames } from '$lib/utils'

	export let size: ButtonType.Size = 'md'
	export let spacingSize: ButtonType.Size = size
	export let color: ButtonType.Color = 'blue'
	export let variant: ButtonType.Variant = 'contained'
	export let btnClasses: string = ''
	export let wrapperClasses: string = ''
	export let wrapperStyle: string = ''
	export let disabled: boolean = false
	export let href: string | undefined = undefined
	export let target: '_self' | '_blank' | undefined = undefined
	export let iconOnly: boolean = false

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
				'border-frost-500 dark:border-frost-300 hover:border-frost-700 dark:hover:border-frost-400 focus:border-frost-700 bg-surface hover:bg-frost-100 dark:hover:bg-frost-900 focus:bg-frost-100 focus:dark:text-frost-100 dark:focus:bg-frost-900 text-frost-500 dark:text-frost-300 dark:hover:text-frost-400 hover:text-frost-700 focus:text-frost-700 focus:ring-frost-300',
			contained:
				'bg-frost-500 hover:bg-frost-700 focus:bg-frost-700 text-white focus:ring-frost-300',
			divider: 'divide-x divide-frost-600'
		},
		red: {
			border:
				'border-red-600 hover:border-red-700 bg-surface hover:bg-red-100 text-red-600 hover:text-red-700 focus:ring-red-300',
			contained: 'bg-red-600 hover:bg-red-700 text-white focus:ring-red-300',
			divider: 'divide-x divide-red-700'
		},
		green: {
			border:
				'border-green-600 hover:border-green-700 bg-surface hover:bg-green-100 text-green-600 hover:text-green-700 focus:ring-green-300',
			contained: 'bg-green-600 hover:bg-green-700 text-white focus:ring-green-300',
			divider: 'divide-x divide-green-700'
		},
		dark: {
			border:
				'border-surface-inverse bg-surface hover:bg-surface-hover focus:bg-surface-hover text-primary hover:text-secondary focus:text-secondary focus:ring-surface-selected-inverse',
			contained:
				'bg-surface-inverse hover:bg-surface-inverse-hover focus:bg-surface-hover-inverse text-primary-inverse focus:ring-surface-selected-inverse',
			divider: 'divide-x divide-gray-800 dark:divide-gray-200'
		},
		gray: {
			border:
				'border-gray-600 hover:border-gray-900 focus:border-gray-900 bg-surface hover:bg-gray-200 focus:bg-gray-200 text-gray-800 hover:text-primary focus:text-primary focus:ring-gray-300',
			contained:
				'bg-gray-700/90 hover:bg-gray-900/90 focus:bg-gray-900/90 text-white focus:ring-gray-300',
			divider: 'divide-x divide-gray-700 dark:divide-gray-200'
		},
		light: {
			border:
				'border  bg-surface hover:bg-surface-hover focus:bg-surface-hover text-primary hover:text-secondary focus:text-secondary focus:ring-surface-selected',
			contained:
				'bg-surface border-transparent hover:bg-surface-hover focus:bg-surface-hover text-primary focus:ring-surface-selected',
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

	$: buttonClass = twMerge(
		'w-full',
		colorVariants?.[color]?.[variant],
		variant === 'border' ? 'border' : '',
		ButtonType.FontSizeClasses[size],
		ButtonType.SpacingClasses[spacingSize][variant],
		'focus:ring-2 font-semibold',
		dropdownItems ? 'rounded-l-md h-full' : 'rounded-md',
		'justify-center items-center text-center whitespace-nowrap inline-flex gap-2',
		btnClasses,
		'transition-all'
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
			data-sveltekit-preload-code="hover"
			bind:this={element}
			on:pointerdown
			on:focus
			on:blur
			on:click={() => {
				loading = true
				dispatch('click', event)
				loading = false
			}}
			{href}
			{download}
			class={twMerge(
				buttonClass,
				disabled ? '!bg-surface-disabled !text-tertiary !cursor-not-allowed' : ''
			)}
			{id}
			{target}
			tabindex={disabled ? -1 : 0}
			{...$$restProps}
			{style}
		>
			{#if loading}
				<Loader2 class="animate-spin mr-1" size={14} />
			{:else if startIcon?.icon}
				<svelte:component this={startIcon.icon} class={startIcon.classes} size={lucideIconSize} />
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
			class={twMerge(
				buttonClass,
				disabled ? '!bg-surface-disabled !text-tertiary border !cursor-not-allowed' : ''
			)}
			{id}
			tabindex={disabled ? -1 : 0}
			{title}
			{...$$restProps}
			disabled={disabled || loading}
			{style}
		>
			{#if loading}
				<Loader2 class="animate-spin mr-1" size={14} />
			{:else if startIcon?.icon}
				<svelte:component this={startIcon.icon} class={startIcon.classes} size={lucideIconSize} />
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
