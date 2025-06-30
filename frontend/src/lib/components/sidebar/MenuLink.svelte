<script lang="ts">
	import { conditionalMelt } from '$lib/utils'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import { navigating, page } from '$app/stores'
	import Popover from '../Popover.svelte'
	import { base } from '$app/paths'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import { goto } from '$app/navigation'
	import { twMerge } from 'tailwind-merge'

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
		class: classNames = ''
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
			class={twMerge(
				'group flex items-center px-2 py-2 text-sm font-light rounded-md h-8 gap-3',
				isSelected
					? lightMode
						? 'bg-surface-selected hover:bg-surface-hover rounded-none data-[highlighted]:bg-surface-hover'
						: 'bg-frost-700 hover:bg-[#30404e] data-[highlighted]:bg-[#30404e]'
					: lightMode
						? 'hover:bg-surface-hover rounded-none data-[highlighted]:bg-surface-hover'
						: 'hover:bg-[#2A3648] data-[highlighted]:bg-[#2A3648]',

				'hover:transition-all',
				classNames
			)}
			target={href.includes('http') ? '_blank' : null}
			title={isCollapsed ? undefined : label}
			use:conditionalMelt={item}
			{...$item}
		>
			{#if icon}
				{@const SvelteComponent = icon}
				<SvelteComponent
					size={16}
					class={twMerge(
						'flex-shrink-0',
						isSelected
							? lightMode
								? 'text-primary group-hover:text-secondary'
								: 'text-frost-200 group-hover:text-white'
							: lightMode
								? 'text-primary group-hover:text-secondary'
								: 'text-gray-100 group-hover:text-white',
						'transition-all'
					)}
				/>
			{/if}

			{#if !isCollapsed}
				<span
					class={twMerge(
						'whitespace-pre truncate',
						isSelected
							? lightMode
								? 'text-primary group-hover:text-secondary'
								: 'text-frost-200 group-hover:text-white'
							: lightMode
								? 'text-primary group-hover:text-secondary'
								: 'text-gray-100 group-hover:text-white',
						'transition-all duration-75'
					)}
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
