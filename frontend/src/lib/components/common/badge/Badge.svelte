<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { IconDefinition } from '@fortawesome/free-solid-svg-icons'
	import { CloseButton } from 'flowbite-svelte'
	import type { BadgeColor } from './model'

	export let color: BadgeColor = 'blue'
	export let large = false
	export let href = ''
	export let rounded = false
	export let index = false
	export let dismissable = false
	export let baseClass = 'inline-flex items-center justify-center -mb-0.5'

	let hidden = false
	const colors = {
		blue: 'bg-blue-100 text-blue-800 dark:bg-blue-200 dark:text-blue-800',
		dark: 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300',
		red: 'bg-red-100 text-red-800 dark:bg-red-200 dark:text-red-900',
		green: 'bg-green-100 text-green-800 dark:bg-green-200 dark:text-green-900',
		yellow: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-200 dark:text-yellow-900',
		indigo: 'bg-indigo-100 text-indigo-800 dark:bg-indigo-200 dark:text-indigo-900',
		purple: 'bg-purple-100 text-purple-800 dark:bg-purple-200 dark:text-purple-900',
		pink: 'bg-pink-100 text-pink-800 dark:bg-pink-200 dark:text-pink-900',
		['!blue']: 'bg-blue-500 text-blue-100',
		['!dark']: 'bg-gray-500 text-gray-100',
		['!red']: 'bg-red-500 text-white',
		['!green']: 'bg-green-500 text-green-100',
		['!yellow']: 'bg-yellow-300 text-yellow-800',
		['!indigo']: 'bg-indigo-500 text-indigo-100',
		['!purple']: 'bg-purple-500 text-purple-100',
		['!pink']: 'bg-pink-500 text-pink-100'
	}
	const hovers = {
		blue: 'hover:bg-blue-200 dark:hover:bg-blue-300',
		dark: 'hover:bg-gray-200 dark:hover:bg-gray-300',
		red: 'hover:bg-red-200 dark:hover:bg-red-300',
		green: 'hover:bg-green-200 dark:hover:bg-green-300',
		yellow: 'hover:bg-yellow-200 dark:hover:bg-yellow-300',
		indigo: 'hover:bg-indigo-200 dark:hover:bg-indigo-300',
		purple: 'hover:bg-purple-200 dark:hover:bg-purple-300',
		pink: 'hover:bg-pink-200 dark:hover:bg-pink-300'
	}

	$: badgeClass = classNames(
		baseClass,
		large ? 'text-sm font-medium' : 'text-xs font-semibold',
		colors[color],
		href && (hovers[color] ?? hovers.blue),
		rounded ? 'rounded-full p-1' : 'rounded px-2.5 py-0.5',
		index
			? 'absolute font-bold border-2 border-white dark:border-gray-900' +
					(large ? 'w-7 h-7 -top-3 -right-3' : 'w-6 h-6 -top-2 -right-2')
			: '',
		$$props.class
	)
	const handleHide = () => (hidden = !hidden)
</script>

<svelte:element this={href ? 'a' : 'span'} {href} {...$$restProps} class={badgeClass} class:hidden>
	<slot />
	{#if dismissable}
		<CloseButton {color} on:click={handleHide} size={large ? 'sm' : 'xs'} class="ml-1.5 -mr-1.5" />
	{/if}
</svelte:element>
