<script lang="ts">
	import { classNames } from '$lib/utils'
	import Icon from 'svelte-awesome'
	import type { IconDefinition } from '@fortawesome/fontawesome-common-types'

	export let label: string
	export let icon: any | undefined = undefined
	export let isCollapsed: boolean
	export let disabled: boolean = false
	export let faIcon: IconDefinition | undefined = undefined

	let isSelected = false
</script>

{#if !disabled}
	<button
		class={classNames(
			'group flex items-center px-2 py-2 text-sm font-light rounded-md h-8 gap-3 w-full',
			isSelected ? 'bg-[#30404e] hover:bg-[#30404e]' : 'hover:bg-[#34363c]',
			'transition-all',
			$$props.class
		)}
		title={label}
	>
		{#if icon}
			<svelte:component
				this={icon}
				size={16}
				class={classNames(
					'flex-shrink-0',
					isSelected
						? 'text-blue-100 group-hover:text-white'
						: 'text-gray-100 group-hover:text-white',
					'transition-all'
				)}
			/>
		{:else if faIcon}
			<Icon
				data={faIcon}
				class={classNames(
					'flex-shrink-0',
					isSelected
						? 'text-blue-100 group-hover:text-white'
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
					'transition-all'
				)}
			>
				{label}
			</span>
		{/if}
	</button>
{/if}
