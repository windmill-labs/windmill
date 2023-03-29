<script lang="ts">
	import type { AppViewerContext, ContextPanelContext } from '$lib/components/apps/types'
	import { classNames } from '$lib/utils'
	import { ChevronDown, ChevronUp, Pointer } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { allsubIds } from '../../appUtils'

	export let id: string
	export let name: string
	export let first: boolean = false
	export let nested: boolean = false
	export let color: 'blue' | 'indigo' = 'indigo'
	export let selectable: boolean = true

	const { manuallyOpened, search, hasResult } = getContext<ContextPanelContext>('ContextPanel')

	const { selectedComponent, app, hoverStore, allIdsInPath, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	$: subids = $search != '' ? allsubIds($app, id) : []
	$: inSearch =
		$search != '' &&
		($hasResult[id] ||
			Object.entries($hasResult).some(([key, value]) => value && subids.includes(key)))
	$: open = $allIdsInPath.includes(id) || $manuallyOpened[id] || inSearch

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
	<!-- svelte-ignore a11y-mouse-events-have-key-events -->
	<div
		on:mouseover|stopPropagation={() => {
			if (id !== $hoverStore) {
				$hoverStore = id
			}
		}}
		on:mouseout|stopPropagation={() => {
			if ($hoverStore !== undefined) {
				$hoverStore = undefined
			}
		}}
		class={classNames(
			'flex items-center justify-between p-1 cursor-pointer border-b gap-1 truncate',
			hoverColor[color],
			$selectedComponent?.includes(id)
				? openBackground[color]
				: $connectingInput.hoveredComponent === id
				? 'bg-orange-300 '
				: 'bg-white',
			first ? 'border-t' : '',
			nested ? 'border-l' : ''
		)}
		on:click={() => {
			$manuallyOpened[id] = $manuallyOpened[id] != undefined ? !$manuallyOpened[id] : true
		}}
	>
		<button
			disabled={!(selectable && !$selectedComponent?.includes(id)) || $connectingInput?.opened}
			title="Select component"
			on:click|stopPropagation={() => ($selectedComponent = [id])}
			class="flex items-center ml-0.5 rounded-sm bg-gray-100 hover:text-black text-gray-600"
		>
			<div
				class={classNames(
					'text-2xs  font-bold px-2 py-0.5 rounded-sm',
					$selectedComponent?.includes(id) ? idClass[color] : ''
				)}
			>
				{id}
			</div>
			{#if selectable && !$selectedComponent?.includes(id)}
				<div class=" px-1 ">
					<Pointer size={14} />
				</div>
			{/if}
		</button>
		<div class="text-2xs font-bold flex flex-row gap-2 items-center truncate">
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
	<div
		class="border-b {open ? 'h-full' : 'h-0 overflow-hidden'} {$connectingInput.hoveredComponent ===
			id && !$selectedComponent?.includes(id)
			? '  bg-orange-100/40'
			: ''}"
	>
		<div class={classNames(nested ? 'border-l ml-2' : '')}>
			<slot />
		</div>
	</div>
</div>
