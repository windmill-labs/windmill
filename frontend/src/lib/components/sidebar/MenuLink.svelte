<script lang="ts">
	import { classNames } from '$lib/utils'
	import { navigating } from '$app/stores'

	import type { IconDefinition } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	export let label: string
	export let href: string
	export let icon: IconDefinition
	export let isCollapsed: boolean

	let isSelected = false

	navigating.subscribe(() => {
		if (href === '/') {
			isSelected = window.location.pathname === href
		} else {
			isSelected = window.location.pathname.includes(href)
		}
	})
</script>

<a
	{href}
	class={classNames(
		'group flex items-center px-2 py-2 text-sm font-medium rounded-md h-8',
		isSelected ? 'bg-blue-100 text-blue-900' : 'text-gray-600 hover:bg-gray-50 hover:text-gray-900'
	)}
	target={href.includes('http') ? '_blank' : null}
>
	<Icon
		data={icon}
		class={classNames(
			' flex-shrink-0 h-4 w-4',
			isSelected ? 'text-blue-500' : 'text-gray-400 group-hover:text-gray-500',
			isCollapsed ? '-mr-1' : 'mr-3'
		)}
	/>

	{#if !isCollapsed}
		<span class="whitespace-pre"> {label}</span>
	{/if}
</a>
