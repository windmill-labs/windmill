<script lang="ts">
	import { classNames } from '$lib/utils'
	import Icon from 'svelte-awesome'

	export let size: 'xs' | 'sm' | 'md' | 'lg' | 'xl' = 'md'
	export let color: 'blue' | 'dark' | 'light' = 'blue'
	export let variant: 'contained' | 'border' = 'contained'
	export let btnClasses: string = ''
	export let disabled: boolean = false
	export let href: string | undefined = undefined
	export let target: '_self' | '_blank' = '_self'

	export let startIcon: { icon: any; classes?: string } | undefined = undefined
	export let endIcon: { icon: any; classes?: string } | undefined = undefined

	function getColorClasses() {
		switch (color) {
			case 'blue':
				return 'bg-blue-500 hover:bg-blue-700 focus:ring-blue-300'
			case 'light':
				return 'text-gray-800 bg-white hover:bg-gray-100 focus:ring-gray-300'
			default:
				return ''
		}
	}

	function getSizeClasses() {
		switch (size) {
			case 'xs':
				return 'text-xs'
			case 'sm':
				return 'text-sm'
			case 'md':
				return 'text-md'
			case 'lg':
				return 'text-lg'
			case 'xl':
				return 'text-xl'
			default:
				return ''
		}
	}

	const buttonProps = {
		class: classNames(
			getColorClasses(),
			getSizeClasses(),
			'text-white focus:ring-4 font-medium',
			'px-4 py-2',
			'rounded-md',
			'flex justify-center items-center',
			variant === 'border' ? 'border' : '',
			btnClasses
		),
		disabled
	}
</script>

{#if href}
	<a {href} {target} on:click|stopPropagation {...buttonProps}>
		{#if startIcon}
			<Icon data={startIcon.icon} class={classNames('mr-2', startIcon.classes)} />
		{/if}
		<slot />
		{#if endIcon}
			<Icon data={endIcon.icon} class={classNames('ml-2', endIcon.classes)} />
		{/if}
	</a>
{:else}
	<button type="button" on:click|stopPropagation {...buttonProps}>
		{#if startIcon}
			<Icon data={startIcon.icon} class={classNames('mr-2', startIcon.classes)} />
		{/if}
		<slot />
		{#if endIcon}
			<Icon data={endIcon.icon} class={classNames('ml-2', endIcon.classes)} />
		{/if}
	</button>
{/if}
