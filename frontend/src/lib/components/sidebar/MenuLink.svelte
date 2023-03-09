<script lang="ts">
	import { classNames } from '$lib/utils'
	import { navigating, page } from '$app/stores'

	import Icon from 'svelte-awesome'
	import type { IconDefinition } from '@fortawesome/fontawesome-common-types'

	export let label: string
	export let href: string
	export let icon: IconDefinition
	export let isCollapsed: boolean
	export let disabled: boolean = false
	export let id: string = ''

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
	<a
		{href}
		class={classNames(
			'group flex items-center px-2 py-2 text-sm font-light rounded-md h-8',
			isSelected
				? 'bg-gray-50 text-gray-900'
				: 'text-gray-600 hover:bg-gray-50 hover:text-gray-900',
			$$props.class
		)}
		target={href.includes('http') ? '_blank' : null}
		{id}
	>
		<Icon
			data={icon}
			class={classNames(
				' flex-shrink-0 h-4 w-4',
				isSelected ? ' text-gray-700' : 'text-white group-hover:text-gray-500',
				isCollapsed ? '-mr-1' : 'mr-3'
			)}
		/>

		{#if !isCollapsed}
			<span
				class={classNames(
					'whitespace-pre truncate',
					isSelected ? ' text-gray-700 font-bold' : 'text-white group-hover:text-gray-900'
				)}
			>
				{label}
			</span>
		{/if}
	</a>
{/if}
