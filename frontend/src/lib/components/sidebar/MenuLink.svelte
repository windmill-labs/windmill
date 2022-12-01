<script lang="ts">
	import { classNames } from '$lib/utils'
	import { navigating } from '$app/stores'

	import type { IconDefinition } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	export let label: string
	export let href: string
	export let icon: IconDefinition
	export let isCollapsed: boolean
	export let disabled: boolean = false

	let isSelected = false

	navigating.subscribe(() => {
		if (href === '/') {
			isSelected = window.location.pathname === href
		} else {
			isSelected = window.location.pathname.includes(href)
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
					'whitespace-pre text-white truncate',
					isSelected ? ' text-gray-700 font-bold' : 'text-white group-hover:text-gray-900'
				)}
			>
				{label}
			</span>
		{/if}
	</a>
{/if}
