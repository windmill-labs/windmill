<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { ButtonType } from './model'
	import { goto } from '$app/navigation'
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
	export let disabled: boolean = false
	export let href: string | undefined = undefined
	export let target: ButtonType.Target = '_self'
	export let iconOnly: boolean = false
	export let startIcon: ButtonType.Icon | undefined = undefined
	export let endIcon: ButtonType.Icon | undefined = undefined
	export let element: ButtonType.Element | undefined = undefined
	export let id: string = ''
	export let nonCaptureEvent: boolean = false
	export let buttonType: 'button' | 'submit' | 'reset' = 'button'
	export let loading = false
	export let title: string | undefined = undefined
	export let style: string = ''

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
				'border-frost-500 hover:border-frost-700 focus:border-frost-700 bg-white hover:bg-frost-100 focus:bg-frost-100 text-frost-500 hover:text-frost-700 focus:text-frost-700 focus:ring-frost-300',
			contained:
				'bg-frost-500 hover:bg-frost-700 focus:bg-frost-700 text-white focus:ring-frost-300',
			divider: 'divide-x divide-frost-600'
		},
		red: {
			border:
				'border-red-600 hover:border-red-700 bg-white hover:bg-red-100 text-red-600 hover:text-red-700 focus:ring-red-300',
			contained: 'bg-red-600 hover:bg-red-700 text-white focus:ring-red-300',
			divider: 'divide-x divide-red-700'
		},
		green: {
			border:
				'border-green-800 hover:border-green-700 bg-white hover:bg-green-100 text-green-600 hover:text-green-700 focus:ring-green-300',
			contained: 'bg-green-600 hover:bg-green-700 text-white focus:ring-green-300',
			divider: 'divide-x divide-green-700'
		},
		dark: {
			border:
				'border-gray-600 hover:border-gray-900 focus:border-gray-900 bg-white hover:bg-gray-200 focus:bg-gray-200 text-gray-800 hover:text-gray-900 focus:text-gray-900 focus:ring-gray-300',
			contained: 'bg-gray-700 hover:bg-gray-900 focus:bg-gray-900 text-white focus:ring-gray-300',
			divider: 'divide-x divide-gray-800'
		},
		gray: {
			border:
				'border-gray-600 hover:border-gray-900 focus:border-gray-900 bg-white hover:bg-gray-200 focus:bg-gray-200 text-gray-800 hover:text-gray-900 focus:text-gray-900 focus:ring-gray-300',
			contained:
				'bg-gray-700/90 hover:bg-gray-900/90 focus:bg-gray-900/90 text-white focus:ring-gray-300',
			divider: 'divide-x divide-gray-700'
		},
		light: {
			border:
				'border border-gray-300 bg-white hover:bg-gray-100 focus:bg-gray-100 text-gray-700 hover:text-gray-800 focus:text-gray-800 focus:ring-gray-300',
			contained:
				'bg-white border-gray-300  hover:bg-gray-100 focus:bg-gray-100 text-gray-700 focus:ring-gray-300',
			divider: 'divide-x divide-gray-100'
		}
	}

	$: buttonProps = {
		id,
		href,
		target,
		tabindex: disabled ? -1 : 0,
		type: buttonType,
		title,
		...$$restProps
	}

	async function onClick(event: MouseEvent) {
		if (!nonCaptureEvent) {
			event.preventDefault()
			event.stopPropagation()
			dispatch('click', event)
			if (href) {
				if (href.startsWith('http') || target == '_blank') {
					window.open(href, target)
				} else {
					loading = true
					await goto(href)
					loading = false
				}
			}
		}
	}

	$: isSmall = size === 'xs' || size === 'sm'
	$: startIconClass = twMerge(iconOnly ? undefined : isSmall ? 'mr-1' : 'mr-2', startIcon?.classes)
	$: endIconClass = twMerge(iconOnly ? undefined : isSmall ? 'ml-1' : 'ml-2', endIcon?.classes)

	$: buttonClass = twMerge(
		'w-full',
		colorVariants?.[color]?.[variant],
		variant === 'border' ? 'border' : '',
		ButtonType.FontSizeClasses[size],
		ButtonType.SpacingClasses[spacingSize][variant],
		'focus:ring-2 font-semibold',
		dropdownItems ? 'rounded-l-md h-full' : 'rounded-md',
		'justify-center items-center text-center whitespace-nowrap inline-flex',
		btnClasses,
		'transition-all '
	)
</script>

<div class="{dropdownItems ? colorVariants[color].divider : ''} {wrapperClasses} flex flex-row">
	<svelte:element
		this={href ? 'a' : 'button'}
		bind:this={element}
		on:pointerdown
		on:click={onClick}
		on:focus
		on:blur
		class={twMerge(buttonClass, disabled ? '!bg-gray-300 !text-gray-600 !cursor-not-allowed' : '')}
		{...buttonProps}
		disabled={disabled || loading}
		type="submit"
		{style}
	>
		{#if loading}
			<Loader2 class="animate-spin mr-1" size={14} />
		{:else if startIcon}
			<Icon data={startIcon.icon} class={startIconClass} scale={ButtonType.IconScale[size]} />
		{/if}

		{#if !iconOnly}
			<slot />
		{/if}
		{#if endIcon}
			<Icon data={endIcon.icon} class={endIconClass} scale={ButtonType.IconScale[size]} />
		{/if}
	</svelte:element>

	{#if dropdownItems}
		<div class={twMerge(buttonClass, 'rounded-r-md rounded-l-none m-0 p-0 h-auto')}>
			<ButtonDropdown>
				<svelte:fragment slot="items">
					{#each computeDropdowns() ?? [] as item}
						<MenuItem on:click={item.onClick} href={item.href}>
							<div
								class={classNames(
									'!text-gray-600 text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
								)}
							>
								{#if item.icon}
									<svelte:component this={item.icon} class="w-4 h-4" />
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
