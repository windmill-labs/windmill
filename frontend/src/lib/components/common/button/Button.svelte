<script lang="ts">
	import { goto } from '$app/navigation'
	import { classNames } from '$lib/utils'
	import Icon from 'svelte-awesome'
	import type { Button } from './model'

	export let size: Button.Size = 'md'
	export let spacingSize: Button.Size = size
	export let color: Button.Color = 'blue'
	export let variant: Button.Variant = 'contained'
	export let btnClasses: string = ''
	export let disabled: boolean = false
	export let href: string | undefined = undefined
	export let target: Button.Target = '_self'
	export let iconOnly: boolean = false

	export let startIcon: { icon: any; classes?: string } | undefined = undefined
	export let endIcon: { icon: any; classes?: string } | undefined = undefined

	// Order of classes: border, border modifier, bg, bg modifier, text, text modifier, everything else
	const colorVariants: Record<Button.Color, Record<Button.Variant, string>> = {
		blue: {
			border:
				'border-blue-500 hover:border-blue-700 bg-white hover:bg-blue-100 text-blue-500 hover:text-blue-700 focus:ring-blue-300',
			contained: 'bg-blue-500 hover:bg-blue-700 text-white focus:ring-blue-300'
		},
		red: {
			border:
				'border-red-600 hover:border-red-700 bg-white hover:bg-red-100 text-red-600 hover:text-red-700 focus:ring-red-300',
			contained: 'bg-red-600 hover:bg-red-700 text-white focus:ring-red-300'
		},
		dark: {
			border:
				'border-gray-800 hover:border-gray-900 bg-white hover:bg-gray-200 text-gray-800 hover:text-gray-900 focus:ring-gray-300',
			contained: 'bg-gray-700 hover:bg-gray-900 text-white focus:ring-gray-300'
		},
		light: {
			border:
				'border bg-white hover:bg-gray-100 text-gray-700 hover:text-gray-800 focus:ring-gray-300',
			contained: 'bg-white hover:bg-gray-100 text-gray-700 focus:ring-gray-300'
		}
	}

	const fontSizeClasses: Record<Button.Size, string> = {
		xs: 'text-xs',
		sm: 'text-sm',
		md: 'text-md',
		lg: 'text-lg',
		xl: 'text-xl'
	}

	const spacingClasses: Record<Button.Size, string> = {
		xs: 'px-3 py-1.5',
		sm: 'px-3 py-1.5',
		md: 'px-4 py-2',
		lg: 'px-4 py-2',
		xl: 'px-4 py-2'
	}
	const iconScale: Record<Button.Size, number> = {
		xs: 0.6,
		sm: 0.8,
		md: 1,
		lg: 1.1,
		xl: 1.2
	}

	$: buttonProps = {
		class: classNames(
			colorVariants[color][variant],
			variant === 'border' ? 'border' : '',
			fontSizeClasses[size],
			spacingClasses[spacingSize],
			'focus:ring-4 font-medium',
			'rounded-md',
			'flex justify-center items-center text-center whitespace-nowrap',
			btnClasses,
			disabled ? 'pointer-events-none cursor-default filter grayscale' : ''
		),
		disabled
	}

	$: isSmall = size === 'xs' || size === 'sm'
	$: startIconClass = classNames(isSmall ? 'mr-1' : 'mr-2', startIcon?.classes)
	$: endIconClass = classNames(isSmall ? 'ml-1' : 'ml-2', endIcon?.classes)
</script>

{#if href}
	<button
		type="button"
		on:click|stopPropagation={() => goto(href ?? '#')}
		{target}
		tabindex={disabled ? -1 : 0}
		{...buttonProps}
	>
		{#if startIcon}
			<Icon
				data={startIcon.icon}
				class={classNames(iconOnly ? undefined : 'mr-2', startIcon.classes)}
				scale={iconScale[size]}
			/>
		{/if}
		{#if !iconOnly}
			<slot />
		{/if}
		{#if endIcon}
			<Icon
				data={endIcon.icon}
				class={classNames(iconOnly ? undefined : 'ml-2', endIcon.classes)}
				scale={iconScale[size]}
			/>
		{/if}
	</button>
{:else}
	<button type="button" on:click|stopPropagation {...buttonProps} {...$$restProps}>
		{#if startIcon}
			<Icon
				data={startIcon.icon}
				class={classNames(iconOnly ? undefined : 'mr-2', startIcon.classes)}
				scale={iconScale[size]}
			/>
		{/if}
		{#if !iconOnly}
			<slot />
		{/if}
		{#if endIcon}
			<Icon
				data={endIcon.icon}
				class={classNames(iconOnly ? undefined : 'ml-2', endIcon.classes)}
				scale={iconScale[size]}
			/>
		{/if}
	</button>
{/if}
