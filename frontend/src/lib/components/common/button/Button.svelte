<script lang="ts">
	import { classNames } from '$lib/utils'
	import Icon from 'svelte-awesome'
	import type { Button } from './model'

	export let size: Button.Size = 'md'
	export let color: Button.Color = 'blue'
	export let variant: Button.Variant = 'contained'
	export let btnClasses: string = ''
	export let disabled: boolean = false
	export let href: string | undefined = undefined
	export let target: Button.Target = '_self'

	export let startIcon: { icon: any; classes?: string } | undefined = undefined
	export let endIcon: { icon: any; classes?: string } | undefined = undefined

	// Order of classes: border, border modifier, bg, bg modifier, text, text modifier, everything else
	const colorVariants: Record<Button.Color, Record<Button.Variant, string>> = {
		blue: {
			border:
				'border-blue-500 hover:border-blue-700 bg-white hover:bg-blue-100 text-blue-500 hover:text-blue-700 focus:ring-blue-300',
			contained: 'bg-blue-500 hover:bg-blue-700 text-white focus:ring-blue-300'
		},
		dark: {
			border:
				'border-gray-800 hover:border-gray-900 bg-white hover:bg-gray-200 text-gray-800 hover:text-gray-900 focus:ring-gray-300',
			contained: 'bg-gray-700 hover:bg-gray-900 text-white focus:ring-gray-300'
		},
		light: {
			border:
				'border-gray-700 hover:border-gray-800 bg-white hover:bg-gray-100 text-gray-700 hover:text-gray-800 focus:ring-gray-300',
			contained: 'bg-white hover:bg-gray-100 text-gray-700 focus:ring-gray-300'
		}
	}

	const sizeClass: Record<Button.Size, string> = {
		xs: 'text-xs px-2.5 py-1',
		sm: 'text-sm px-2.5 py-1',
		md: 'text-md px-3 py-1',
		lg: 'text-lg px-4 py-2',
		xl: 'text-xl px-5 py-2'
	}

	const iconScale: Record<Button.Size, number> = {
		xs: 0.7,
		sm: 0.8,
		md: 1,
		lg: 1.1,
		xl: 1.2
	}

	const buttonProps = {
		class: classNames(
			colorVariants[color][variant],
			variant === 'border' ? 'border' : '',
			sizeClass[size],
			'focus:ring-4 font-medium',
			'rounded-md',
			'flex justify-center items-center text-center',
			btnClasses,
			disabled ? 'pointer-events-none cursor-default filter grayscale' : ''
		),
		disabled
	}
</script>

{#if href}
	<a {href} {target} tabindex={disabled ? -1 : 0} on:click|stopPropagation {...buttonProps}>
		{#if startIcon}
			<Icon
				data={startIcon.icon}
				class={classNames('mr-2', startIcon.classes)}
				scale={iconScale[size]}
			/>
		{/if}
		<slot />
		{#if endIcon}
			<Icon
				data={endIcon.icon}
				class={classNames('ml-2', endIcon.classes)}
				scale={iconScale[size]}
			/>
		{/if}
	</a>
{:else}
	<button type="button" on:click|stopPropagation {...buttonProps}>
		{#if startIcon}
			<Icon
				data={startIcon.icon}
				class={classNames('mr-2', startIcon.classes)}
				scale={iconScale[size]}
			/>
		{/if}
		<slot />
		{#if endIcon}
			<Icon
				data={endIcon.icon}
				class={classNames('ml-2', endIcon.classes)}
				scale={iconScale[size]}
			/>
		{/if}
	</button>
{/if}
