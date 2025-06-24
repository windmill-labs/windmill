<script lang="ts">
	import { classNames, conditionalMelt } from '$lib/utils'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import { navigating, page } from '$app/stores'
	import Popover from '../Popover.svelte'
	import { base } from '$app/paths'
	import { triggerableByAI } from '$lib/actions/triggerableByAI'
	import { goto } from '$app/navigation'

	export let aiId: string | undefined = undefined
	export let aiDescription: string | undefined = undefined
	export let label: string
	export let href: string
	export let icon: any | undefined = undefined
	export let isCollapsed: boolean
	export let disabled: boolean = false
	export let lightMode: boolean = false
	export let item: MenubarMenuElements['item'] | undefined = undefined

	let isSelected = false

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
				class={classNames(
					'group flex items-center px-2 py-2 text-sm font-light rounded-md h-8 gap-3',
					isSelected
						? lightMode
							? 'bg-surface-selected hover:bg-surface-hover rounded-none data-[highlighted]:bg-surface-hover'
							: 'bg-frost-700 hover:bg-[#30404e] data-[highlighted]:bg-[#30404e]'
						: lightMode
							? 'hover:bg-surface-hover rounded-none data-[highlighted]:bg-surface-hover'
							: 'hover:bg-[#2A3648] data-[highlighted]:bg-[#2A3648]',

					'hover:transition-all',
					$$props.class
				)}
				target={href.includes('http') ? '_blank' : null}
				title={isCollapsed ? undefined : label}
				use:conditionalMelt={item}
				{...$item}
			>
				{#if icon}
					<svelte:component
						this={icon}
						size={16}
						class={classNames(
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
						class={classNames(
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
		<svelte:fragment slot="text">
			{label}
		</svelte:fragment>
	</Popover>
{/if}
