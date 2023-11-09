<script lang="ts">
	import { classNames } from '$lib/utils'
	import { type BadgeColor, type BadgeIconProps, ColorModifier } from './model'
	import { X } from 'lucide-svelte'

	export let color: BadgeColor = 'gray'
	export let large = false
	export let small = false
	export let href = ''
	export let rounded = false
	export let dismissable = false
	export let wrapperClass = ''
	export let baseClass = 'text-center'
	export let capitalize = false
	export let icon: BadgeIconProps | undefined = undefined

	let hidden = false
	const colors: Record<BadgeColor, string> = {
		gray: 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300',
		blue: 'bg-blue-100 text-blue-800 dark:bg-blue-200 dark:text-blue-800',
		red: 'bg-red-100 text-red-800 dark:bg-red-200 dark:text-red-900',
		green: 'bg-green-100 text-green-800 dark:bg-green-200 dark:text-green-900',
		yellow: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-200 dark:text-yellow-900',
		indigo: 'bg-indigo-100 text-indigo-800 dark:bg-indigo-200 dark:text-indigo-900',
		['dark-gray']: 'bg-gray-500 text-gray-100',
		['dark-blue']: 'bg-blue-500 text-blue-100',
		['dark-red']: 'bg-red-500 text-white',
		['dark-green']: 'bg-green-500 text-green-100',
		['dark-yellow']: 'bg-yellow-300 text-yellow-800',
		['dark-indigo']: 'bg-indigo-500 text-indigo-100'
	}
	const hovers: Partial<Record<BadgeColor, string>> = {
		gray: 'hover:bg-gray-200 dark:hover:bg-gray-300',
		blue: 'hover:bg-blue-200 dark:hover:bg-blue-300',
		red: 'hover:bg-red-200 dark:hover:bg-red-300',
		green: 'hover:bg-green-200 dark:hover:bg-green-300',
		yellow: 'hover:bg-yellow-200 dark:hover:bg-yellow-300',
		indigo: 'hover:bg-indigo-200 dark:hover:bg-indigo-300'
	}

	$: badgeClass = classNames(
		baseClass,
		small ? 'text-xs' : large ? 'text-sm font-medium' : 'text-xs font-semibold',
		colors[color],
		href &&
			(color.startsWith(ColorModifier) ? hovers[color.replace(ColorModifier, '')] : hovers[color]),
		rounded ? 'rounded-full px-2 py-1' : 'rounded px-2.5 py-0.5',
		'flex flex-row gap-1 items-center',
		$$props.class
	)
	const handleHide = () => (hidden = !hidden)
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<span
	on:click
	on:keydown
	class="inline-flex justify-center items-center whitespace-nowrap {wrapperClass}"
>
	<svelte:element
		this={href ? 'a' : 'span'}
		{href}
		{...$$restProps}
		class={badgeClass}
		class:hidden
		class:capitalize
	>
		{#if icon?.icon && icon.position === 'left'}
			<svelte:component this={icon.icon} size={14} />
		{/if}
		<slot />
		{#if icon?.icon && icon.position === 'right'}
			<svelte:component this={icon.icon} size={14} />
		{/if}
		{#if dismissable}
			<button on:click={handleHide}>
				<X size={10} />
			</button>
		{/if}
	</svelte:element>
</span>
