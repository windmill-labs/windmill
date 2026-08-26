<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import {
		badgeColors,
		badgeHovers,
		badgeSelectedColors,
		type BadgeColor,
		type BadgeIconProps,
		ColorModifier
	} from './model'
	import { X } from 'lucide-svelte'

	interface Props {
		color?: BadgeColor
		large?: boolean
		small?: boolean
		href?: string
		rounded?: boolean
		roundedFull?: boolean
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
		roundedFull = false,
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

	let badgeClass = $derived(
		twMerge(
			baseClass,
			small ? 'text-2xs' : verySmall ? 'text-2xs' : large ? 'text-sm' : 'text-2xs',
			selected ? badgeSelectedColors[color] : badgeColors[color],
			clickable &&
				!selected &&
				(color.startsWith(ColorModifier)
					? badgeHovers[color.replace(ColorModifier, '')]
					: badgeHovers[color]),

			roundedFull
				? 'rounded-full p-2 w-fit h-fit'
				: rounded
					? 'rounded-full px-2 py-1'
					: 'rounded-md px-2 py-0.5',
			verySmall ? 'px-0.5 py-0.5' : '',
			'flex flex-row gap-1 items-center justify-center',
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
		this={href ? 'a' : clickable ? 'button' : 'span'}
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
