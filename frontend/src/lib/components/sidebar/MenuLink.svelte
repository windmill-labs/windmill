<script lang="ts">
	import { classNames } from '$lib/utils'
	import { navigating, page } from '$app/stores'
	import Popover from '../Popover.svelte'

	export let label: string
	export let href: string
	export let icon: any | undefined = undefined
	export let isCollapsed: boolean
	export let disabled: boolean = false

	let isSelected = false

	navigating.subscribe(() => {
		if (href === '/') {
			isSelected = $page.url.pathname === href
		} else {
			isSelected = $page.url.pathname.includes(href)
		}
	})
</script>

{#if !disabled}
	<Popover appearTimeout={0} disappearTimeout={0} class="w-full" disablePopup={!isCollapsed}>
		<a
			{href}
			class={classNames(
				'group flex items-center px-2 py-2 text-sm font-light rounded-md h-8 gap-3',
				isSelected ? 'bg-frost-700 hover:bg-[#30404e]' : 'hover:bg-[#34363c]',
				'hover:transition-all',
				$$props.class
			)}
			target={href.includes('http') ? '_blank' : null}
			title={isCollapsed ? undefined : label}
		>
			{#if icon}
				<svelte:component
					this={icon}
					size={16}
					class={classNames(
						'flex-shrink-0',
						isSelected
							? 'text-frost-200 group-hover:text-white'
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
							? 'text-blue-100 group-hover:text-white font-semibold'
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
