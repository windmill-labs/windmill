<script lang="ts">
	import { Tab, TabGroup, TabList } from '@rgossiaux/svelte-headlessui'
	import { Eye, Pen } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { AppViewerContext } from '../types'

	export let loading: boolean = false

	const { mode } = getContext<AppViewerContext>('AppViewerContext')

	let tabs = [
		{ label: 'Editor', value: 'code', icon: Pen },
		{ label: 'Preview', value: 'preview', icon: Eye }
	]
</script>

<TabGroup
	class="h-[30px] flex"
	let:selectedIndex
	on:change={(e) => {
		if (e.detail === 0) {
			mode.set('dnd')
		} else {
			mode.set('preview')
		}
	}}
>
	<TabList class="flex bg-gray-100 rounded-md p-0.5 gap-1 h-full">
		{#each tabs as tab, index}
			<Tab
				disabled={loading}
				class={({ selected }) =>
					twMerge(
						'px-2 py-1 rounded-md transition-all text-xs flex gap-1 flex-row items-center ',
						selected ? 'bg-white shadow-md text-gray-800' : 'text-gray-600 hover:bg-gray-200'
					)}
			>
				{#if tab.icon}
					<svelte:component
						this={tab.icon}
						size={12}
						color={selectedIndex === index ? '#3b82f6' : '#9CA3AF'}
					/>
				{/if}
				{tab.label}
			</Tab>
		{/each}
	</TabList>
</TabGroup>
