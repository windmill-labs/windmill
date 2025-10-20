<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { type BadgeColor, type BadgeIconProps, ColorModifier } from './model'
	import { X } from 'lucide-svelte'

	interface Props {
		color?: BadgeColor
		large?: boolean
		small?: boolean
		href?: string
		rounded?: boolean
		dismissable?: boolean
		wrapperClass?: string
		baseClass?: string
		capitalize?: boolean
		icon?: BadgeIconProps | undefined
		verySmall?: boolean
		class?: string | undefined
		children?: import('svelte').Snippet
		[key: string]: any
		clickable?: boolean
		selected?: boolean
		onkeydown?: (event: KeyboardEvent) => void
		onclick?: (event: MouseEvent) => void
	}

	let {
		color = 'gray',
		large = false,
		small = false,
		href = '',
		rounded = false,
		dismissable = false,
		wrapperClass = '',
		baseClass = 'text-center text-primary font-normal',
		capitalize = false,
		icon = undefined,
		verySmall = false,
		class: classNames = undefined,
		clickable = false,
		selected = false,
		children,
		onkeydown,
		onclick,
		...rest
	}: Props = $props()

	let hidden = $state(false)
	const colors: Record<BadgeColor, string> = {
		gray: 'bg-surface-sunken text-primary',
		blue: 'bg-blue-100 text-blue-800 dark:bg-blue-200 dark:text-blue-800',
		red: 'bg-red-100 text-red-800 dark:bg-red-200 dark:text-red-900',
		green: 'bg-green-100 text-green-700 dark:bg-green-200 dark:text-green-900',
		yellow: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-100 dark:text-yellow-900',
		orange: 'bg-orange-100 text-orange-800 dark:bg-orange-100 dark:text-orange-900',
		indigo: 'bg-indigo-100 text-indigo-800 dark:bg-indigo-200 dark:text-indigo-900',
		violet: 'bg-violet-100 text-violet-800 dark:bg-violet-200 dark:text-violet-900',
		['dark-gray']: 'bg-gray-500 text-gray-100',
		['dark-blue']: 'bg-blue-500 text-blue-100',
		['dark-red']: 'bg-red-500 text-white',
		['dark-green']: 'bg-green-500 text-green-100',
		['dark-yellow']: 'bg-yellow-300 text-yellow-800',
		['dark-indigo']: 'bg-indigo-500 text-indigo-100',
		['dark-orange']: 'bg-orange-500 text-orange-100',
		['dark-violet']: 'bg-violet-500 text-violet-100',
		transparent: 'bg-transparent border'
	}

	const selectedColors: Record<BadgeColor, string> = {
		gray: 'bg-surface-accent-selected/30 text-accent',
		blue: 'bg-blue-500 text-white dark:bg-blue-600',
		red: 'bg-red-500 text-white dark:bg-red-600',
		green: 'bg-green-500 text-white dark:bg-green-600',
		yellow: 'bg-yellow-500 text-white dark:bg-yellow-600',
		orange: 'bg-orange-500 text-white dark:bg-orange-600',
		indigo: 'bg-indigo-500 text-white dark:bg-indigo-600',
		violet: 'bg-violet-500 text-white dark:bg-violet-600',
		['dark-gray']: 'bg-gray-700 text-gray-100',
		['dark-blue']: 'bg-blue-700 text-blue-100',
		['dark-red']: 'bg-red-700 text-white',
		['dark-green']: 'bg-green-700 text-green-100',
		['dark-yellow']: 'bg-yellow-600 text-yellow-100',
		['dark-indigo']: 'bg-indigo-700 text-indigo-100',
		['dark-orange']: 'bg-orange-700 text-orange-100',
		['dark-violet']: 'bg-violet-700 text-violet-100',
		transparent: 'bg-surface-accent-selected/30 text-accent'
	}

	const hovers: Partial<Record<BadgeColor, string>> = {
		gray: 'hover:bg-surface-hover',
		blue: 'hover:bg-blue-200 dark:hover:bg-blue-300',
		red: 'hover:bg-red-200 dark:hover:bg-red-300',
		green: 'hover:bg-green-200 dark:hover:bg-green-300',
		yellow: 'hover:bg-yellow-200 dark:hover:bg-yellow-300',
		indigo: 'hover:bg-indigo-200 dark:hover:bg-indigo-300',
		orange: 'hover:bg-orange-200 dark:hover:bg-orange-300',
		violet: 'hover:bg-violet-200 dark:hover:bg-violet-300',
		transparent: 'hover:bg-surface-hover/50'
	}

	let badgeClass = $derived(
		twMerge(
			baseClass,
			small ? 'text-xs' : verySmall ? 'text-2xs' : large ? 'text-xs' : 'text-2xs',
			selected ? selectedColors[color] : colors[color],
			clickable &&
				!selected &&
				(color.startsWith(ColorModifier)
					? hovers[color.replace(ColorModifier, '')]
					: hovers[color]),
			rounded ? 'rounded-full px-2 py-1' : 'rounded-md px-2 py-0.5',
			verySmall ? 'px-0.5 py-0.5' : '',
			'flex flex-row gap-1 items-center',
			classNames
		)
	)
	const handleHide = () => (hidden = !hidden)
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span
	{onclick}
	{onkeydown}
	class="inline-flex justify-center items-center whitespace-nowrap {wrapperClass}"
>
	<svelte:element
		this={href ? 'a' : 'span'}
		{href}
		{...rest}
		class={badgeClass}
		class:hidden
		class:capitalize
		role={clickable ? 'button' : undefined}
	>
		{#if icon?.icon && icon.position === 'left'}
			<icon.icon size={12} />
		{/if}
		{@render children?.()}
		{#if icon?.icon && icon.position === 'right'}
			<icon.icon size={12} />
		{/if}
		{#if dismissable}
			<button onclick={handleHide}>
				<X size={10} />
			</button>
		{/if}
	</svelte:element>
</span>
