<script lang="ts">
	import { conditionalMelt } from '$lib/utils'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import { navigating, page } from '$app/stores'
	import Popover from '../Popover.svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import { goto } from '$app/navigation'
	import { twMerge } from 'tailwind-merge'
	import { sidebarClasses } from './MenuButton.svelte'

	interface Props {
		aiId?: string | undefined
		aiDescription?: string | undefined
		label: string
		// Native tooltip in every mode, for a row whose label does not say where it goes.
		hint?: string | undefined
		href: string
		icon?: any | undefined
		iconProps?: Record<string, unknown>
		// Shown in place of the icon and label while the row is hovered or focused, for a row
		// whose resting look (a brand mark) does not say what it does.
		hoverIcon?: any | undefined
		hoverLabel?: string | undefined
		isCollapsed: boolean
		disabled?: boolean
		lightMode?: boolean
		item?: MenubarMenuElements['item'] | undefined
		class?: string
		onclick?: (ev: MouseEvent) => any
	}

	let {
		aiId = undefined,
		aiDescription = undefined,
		label,
		hint = undefined,
		href,
		icon = undefined,
		iconProps = {},
		hoverIcon = undefined,
		hoverLabel = undefined,
		isCollapsed,
		disabled = false,
		lightMode = false,
		item = undefined,
		class: classNames = '',
		onclick = undefined
	}: Props = $props()

	let isSelected = $state(false)

	navigating.subscribe(() => {
		if (href === '/') {
			isSelected = $page.url.pathname === href
		} else {
			isSelected = $page.url.pathname.startsWith(href)
		}
	})

	let iconClass = $derived(
		twMerge(
			'flex-shrink-0 transition-all',
			isSelected ? sidebarClasses.selectedText : sidebarClasses.iconText
		)
	)
	const restingClass = 'group-hover:hidden group-focus-visible:hidden'
	const hoverClass = 'hidden group-hover:block group-focus-visible:block'
</script>

{#if !disabled}
	<Popover
		appearTimeout={0}
		disappearTimeout={0}
		class="w-full"
		disablePopup={!isCollapsed || !!hint}
	>
		<a
			{href}
			use:triggerableByAI={{
				id: aiId,
				description: aiDescription,
				callback: () => {
					goto(href)
				}
			}}
			{onclick}
			class={twMerge(
				'group flex items-center px-2 py-1 text-sm font-light rounded-md gap-2',
				// One height in the rail and the open sidebar, so a row keeps its place in every mode.
				'h-[30px]',
				isSelected
					? sidebarClasses.selectedBg
					: item
						? 'transition-colors'
						: sidebarClasses.hoverBg,
				// Inside a menu, melt moves data-highlighted with both the keyboard and the
				// pointer, so it is the only pointer state: a hover rule as well lights two rows.
				item ? 'data-[highlighted]:bg-surface-hover' : '',
				isSelected ? sidebarClasses.selectedText : sidebarClasses.text,
				classNames
			)}
			data-light-mode={lightMode}
			target={href.includes('http') ? '_blank' : null}
			aria-label={label}
			title={hint ?? (isCollapsed ? undefined : label)}
			use:conditionalMelt={item}
			{...$item}
		>
			{#if icon}
				{@const SvelteComponent = icon}
				<SvelteComponent
					{...iconProps}
					size={16}
					class={twMerge(iconClass, hoverIcon ? restingClass : '')}
				/>
			{/if}
			{#if hoverIcon}
				{@const HoverComponent = hoverIcon}
				<HoverComponent size={16} class={twMerge(iconClass, hoverClass)} />
			{/if}

			{#if !isCollapsed}
				<span
					data-light-mode={lightMode}
					class={twMerge(
						'whitespace-pre truncate transition-all duration-75',
						hoverLabel ? restingClass : ''
					)}
				>
					{label}
				</span>
				{#if hoverLabel}
					<span data-light-mode={lightMode} class={twMerge('whitespace-pre truncate', hoverClass)}>
						{hoverLabel}
					</span>
				{/if}
			{/if}
		</a>
		{#snippet text()}
			{label}
		{/snippet}
	</Popover>
{/if}
