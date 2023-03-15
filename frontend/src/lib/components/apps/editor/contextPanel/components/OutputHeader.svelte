<script lang="ts">
	import type { AppViewerContext, ContextPanelContext } from '$lib/components/apps/types'
	import { classNames } from '$lib/utils'
	import { ChevronDown, ChevronUp } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { slide } from 'svelte/transition'
	import { allsubIds } from '../../appUtils'

	export let id: string
	export let name: string
	export let first: boolean = false
	export let nested: boolean = false
	export let color: 'blue' | 'indigo' = 'indigo'

	const { expanded, manuallyOpened, search, hasResult } =
		getContext<ContextPanelContext>('ContextPanel')

	const { selectedComponent, app } = getContext<AppViewerContext>('AppViewerContext')

	$: subids = allsubIds($app, id)
	$: inSearch =
		$search != '' &&
		($hasResult[id] ||
			Object.entries($hasResult).some(([key, value]) => value && subids.includes(key)))
	$: open =
		$expanded || subids.includes($selectedComponent ?? '') || $manuallyOpened[id] || inSearch

	const dispatch = createEventDispatcher()

	const hoverColor = {
		blue: 'hover:bg-blue-300 hover:text-blue-600',
		indigo: 'hover:bg-indigo-300 hover:text-indigo-600'
	}

	const openBackground = {
		blue: 'bg-blue-50',
		indigo: 'bg-indigo-50'
	}

	const manuallyOpenColor = {
		blue: 'text-blue-600',
		indigo: 'text-indigo-600'
	}

	const idClass = {
		blue: 'bg-blue-500 text-white',
		indigo: 'bg-indigo-500 text-white'
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class={$search == '' || inSearch ? '' : 'invisible h-0 overflow-hidden'}>
	<div
		class={classNames(
			'flex items-center justify-between p-1 cursor-pointer border-b gap-1 truncate',
			hoverColor[color],
			$selectedComponent == id ? openBackground[color] : 'bg-white',
			first ? 'border-t' : '',
			nested ? 'border-l' : ''
		)}
		on:click={() => {
			dispatch('handleClick', { manuallyOpen: $manuallyOpened[id] })
			$manuallyOpened[id] = $manuallyOpened[id] != undefined ? !$manuallyOpened[id] : true
		}}
	>
		<div
			class={classNames(
				'text-2xs ml-0.5 font-bold px-2 py-0.5 rounded-sm',
				$selectedComponent == id ? idClass[color] : ' bg-gray-100'
			)}
		>
			{id}
		</div>
		<div
			on:click|stopPropagation={() => {
				$manuallyOpened[id] = $manuallyOpened[id] != undefined ? !$manuallyOpened[id] : true
			}}
			class="text-2xs font-bold flex flex-row gap-2 items-center truncate"
		>
			{name}
			{#if !open}
				<ChevronDown size={14} />
			{:else if $manuallyOpened[id]}
				<ChevronUp size={14} class={manuallyOpenColor[color]} strokeWidth={4} />
			{:else}
				<ChevronUp size={14} />
			{/if}
		</div>
	</div>
	<div class="py-1 border-b  {open ? '' : 'invisible h-0 overflow-hidden'} ">
		<div class={classNames(nested ? 'border-l ml-2' : '')}>
			<slot />
		</div>
	</div>
</div>
