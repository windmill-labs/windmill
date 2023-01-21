<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { classNames } from '$lib/utils'
	import Icon from 'svelte-awesome'
	import { ButtonType } from './model'
	import { goto } from '$app/navigation'
	import { Loader2 } from 'lucide-svelte'

	export let size: ButtonType.Size = 'md'
	export let spacingSize: ButtonType.Size = size
	export let color: ButtonType.Color = 'blue'
	export let variant: ButtonType.Variant = 'contained'
	export let btnClasses: string = ''
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

	const dispatch = createEventDispatcher()
	// Order of classes: border, border modifier, bg, bg modifier, text, text modifier, everything else
	const colorVariants: Record<ButtonType.Color, Record<ButtonType.Variant, string>> = {
		blue: {
			border:
				'border-frost-500 hover:border-frost-700 focus:border-frost-700 bg-white hover:bg-frost-100 focus:bg-frost-100 text-frost-500 hover:text-frost-700 focus:text-frost-700 focus:ring-frost-300',
			contained:
				'bg-frost-500 hover:bg-frost-700 focus:bg-frost-700 text-white focus:ring-frost-300'
		},
		red: {
			border:
				'border-red-600 hover:border-red-700 bg-white hover:bg-red-100 text-red-600 hover:text-red-700 focus:ring-red-300',
			contained: 'bg-red-600 hover:bg-red-700 text-white focus:ring-red-300'
		},
		green: {
			border:
				'border-green-600 hover:border-green-700 bg-white hover:bg-green-100 text-green-600 hover:text-green-700 focus:ring-green-300',
			contained: 'bg-green-600 hover:bg-green-700 text-white focus:ring-green-300'
		},
		dark: {
			border:
				'border-gray-800 hover:border-gray-900 focus:border-gray-900 bg-white hover:bg-gray-200 focus:bg-gray-200 text-gray-800 hover:text-gray-900 focus:text-gray-900 focus:ring-gray-300',
			contained: 'bg-gray-700 hover:bg-gray-900 focus:bg-gray-900 text-white focus:ring-gray-300'
		},
		gray: {
			border:
				'border-gray-600 hover:border-gray-900 focus:border-gray-900 bg-white hover:bg-gray-200 focus:bg-gray-200 text-gray-800 hover:text-gray-900 focus:text-gray-900 focus:ring-gray-300',
			contained:
				'bg-gray-700/90 hover:bg-gray-900/90 focus:bg-gray-900/90 text-white focus:ring-gray-300'
		},
		light: {
			border:
				'border border-gray-300 bg-white hover:bg-gray-100 focus:bg-gray-100 text-gray-700 hover:text-gray-800 focus:text-gray-800 focus:ring-gray-300',
			contained:
				'bg-white border-gray-300  hover:bg-gray-100 focus:bg-gray-100 text-gray-700 focus:ring-gray-300'
		}
	}

	$: buttonProps = {
		id,
		class: classNames(
			colorVariants?.[color]?.[variant],
			variant === 'border' ? 'border' : '',
			ButtonType.FontSizeClasses[size],
			ButtonType.SpacingClasses[spacingSize],
			'focus:ring-2 font-semibold',
			'duration-200 rounded-md',
			'justify-center items-center text-center whitespace-nowrap inline-flex',
			btnClasses,
			disabled ? '!bg-gray-300 !text-gray-600 !cursor-not-allowed' : ''
		),
		href,
		target,
		tabindex: disabled ? -1 : 0,
		type: buttonType,
		title
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
	$: startIconClass = classNames(
		iconOnly ? undefined : isSmall ? 'mr-1' : 'mr-2',
		startIcon?.classes
	)
	$: endIconClass = classNames(iconOnly ? undefined : isSmall ? 'ml-1' : 'ml-2', endIcon?.classes)
</script>

<svelte:element
	this={href ? 'a' : 'button'}
	bind:this={element}
	on:pointerdown
	on:click={onClick}
	on:focus
	on:blur
	{...buttonProps}
	disabled={disabled || loading}
	type="submit"
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
