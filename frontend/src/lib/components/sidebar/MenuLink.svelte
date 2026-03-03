<script lang="ts">
	import { conditionalMelt } from '$lib/utils'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import { navigating, page } from '$app/stores'
	import Popover from '../Popover.svelte'
	import { base } from '$app/paths'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import { goto } from '$app/navigation'
	import { twMerge } from 'tailwind-merge'
	import { sidebarClasses } from './MenuButton.svelte'

	interface Props {
		aiId?: string | undefined
		aiDescription?: string | undefined
		label: string
		href: string
		icon?: any | undefined
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
		href,
		icon = undefined,
		isCollapsed,
		disabled = false,
		lightMode = false,
		item = undefined,
		class: classNames = '',
		onclick = undefined
	}: Props = $props()

	let isSelected = $state(false)

	navigating.subscribe(() => {
		if (href === `${base}/`) {
			isSelected = $page.url.pathname === href
		} else {
			isSelected = $page.url.pathname.startsWith(href)
		}
	})
</script>

{#if !disabled}
	<Popover appearTimeout={0} disappearTimeout={0} class="w-full" disablePopup={!isCollapsed}>
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
				'group flex items-center px-2 py-2 text-sm font-light rounded-md h-8 gap-2',
				isSelected
					? sidebarClasses.selectedBg
					: sidebarClasses.hoverBg,
				isSelected ? sidebarClasses.selectedText : sidebarClasses.text,
				classNames
			)}
			data-light-mode={lightMode}
			target={href.includes('http') ? '_blank' : null}
			aria-label={label}
			title={isCollapsed ? undefined : label}
			use:conditionalMelt={item}
			{...$item}
		>
			{#if icon}
				{@const SvelteComponent = icon}
				<SvelteComponent size={16} class={twMerge('flex-shrink-0 transition-all', isSelected ? sidebarClasses.selectedText : sidebarClasses.iconText)} />
			{/if}

			{#if !isCollapsed}
				<span
					data-light-mode={lightMode}
					class={twMerge('whitespace-pre truncate transition-all duration-75')}
				>
					{label}
				</span>
			{/if}
		</a>
		{#snippet text()}
			{label}
		{/snippet}
	</Popover>
{/if}
