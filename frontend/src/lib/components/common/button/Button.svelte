<script lang="ts">
	import { classNames } from '$lib/utils'
	import Icon from 'svelte-awesome'
	import { ButtonType } from './model'

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

	// Order of classes: border, border modifier, bg, bg modifier, text, text modifier, everything else
	const colorVariants: Record<ButtonType.Color, Record<ButtonType.Variant, string>> = {
		blue: {
			border:
				'border-blue-500 hover:border-blue-700 focus:border-blue-700 bg-white hover:bg-blue-100 focus:bg-blue-100 text-blue-500 hover:text-blue-700 focus:text-blue-700 focus:ring-blue-300',
			contained: 'bg-blue-500 hover:bg-blue-700 focus:bg-blue-700 text-white focus:ring-blue-300'
		},
		red: {
			border:
				'border-red-500 hover:border-red-700 focus:border-red-700 bg-white hover:bg-red-100 focus:bg-red-100 text-red-500 hover:text-red-700 focus:text-red-700 focus:ring-red-300',
			contained: 'bg-red-500 hover:bg-red-700 focus:bg-red-700 text-white focus:ring-red-300'
		},
		dark: {
			border:
				'border-gray-800 hover:border-gray-900 focus:border-gray-900 bg-white hover:bg-gray-200 focus:bg-gray-200 text-gray-800 hover:text-gray-900 focus:text-gray-900 focus:ring-gray-300',
			contained: 'bg-gray-700 hover:bg-gray-900 focus:bg-gray-900 text-white focus:ring-gray-300'
		},
		light: {
			border:
				'border bg-white hover:bg-gray-100 focus:bg-gray-100 text-gray-700 hover:text-gray-800 focus:text-gray-800 focus:ring-gray-300',
			contained: 'bg-white hover:bg-gray-100 focus:bg-gray-100 text-gray-700 focus:ring-gray-300'
		}
	}

	$: buttonProps = {
		class: classNames(
			colorVariants[color][variant],
			variant === 'border' ? 'border' : '',
			ButtonType.FontSizeClasses[size],
			ButtonType.SpacingClasses[spacingSize],
			'focus:ring-4 font-medium',
			'rounded-md',
			'flex justify-center items-center text-center whitespace-nowrap',
			btnClasses,
			disabled ? 'pointer-events-none cursor-default filter grayscale' : ''
		),
		disabled,
		href,
		target,
		tabindex: disabled ? -1 : 0
	}
</script>

<svelte:element
	this={href ? 'a' : 'button'}
	on:click|stopPropagation
	on:focus
	on:blur
	{...buttonProps}
>
	{#if startIcon}
		<Icon
			data={startIcon.icon}
			class={classNames(iconOnly ? undefined : 'mr-2', startIcon.classes)}
			scale={ButtonType.IconScale[size]}
		/>
	{/if}
	{#if !iconOnly}
		<slot />
	{/if}
	{#if endIcon}
		<Icon
			data={endIcon.icon}
			class={classNames(iconOnly ? undefined : 'ml-2', endIcon.classes)}
			scale={ButtonType.IconScale[size]}
		/>
	{/if}
</svelte:element>
