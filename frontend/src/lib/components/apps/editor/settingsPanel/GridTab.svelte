<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import CloseButton from '$lib/components/common/CloseButton.svelte'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'
	import { deleteGridItem } from '../appUtils'
	import type { AppComponent } from '../component'
	import PanelSection from './common/PanelSection.svelte'
	import { dndzone } from 'svelte-dnd-action'
	import { generateRandomString } from '$lib/utils'
	import { GripVertical } from 'lucide-svelte'

	export let tabs: string[]
	export let component: AppComponent

	let items = tabs.map((tab, index) => {
		return { value: tab, id: generateRandomString(), originalIndex: index }
	})

	const { app, runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	function addTab() {
		const numberOfTabs = tabs.length
		tabs = [...tabs, `Tab ${numberOfTabs + 1}`]

		if (!$app.subgrids) {
			$app.subgrids = {}
		}

		$app.subgrids[`${component.id}-${numberOfTabs}`] = []
		component.numberOfSubgrids = tabs.length

		items = [
			...items,
			{
				value: tabs[tabs.length - 1],
				id: generateRandomString(),
				originalIndex: tabs.length - 1
			}
		]
	}

	function deleteSubgrid(index: number) {
		let subgrid = `${component.id}-${index}`
		for (const item of $app!.subgrids![subgrid]) {
			const components = deleteGridItem($app, item.data, subgrid, false)
			for (const key in components) {
				delete $runnableComponents[key]
			}
		}
		$runnableComponents = $runnableComponents
		for (let i = index; i < tabs.length - 1; i++) {
			$app!.subgrids![`${component.id}-${i}`] = $app!.subgrids![`${component.id}-${i + 1}`]
		}
		tabs.splice(index, 1)
		tabs = tabs
		component.numberOfSubgrids = tabs.length
		$app = $app

		// Remove the corresponding item from the items array
		items = items.filter((item) => item.originalIndex !== index)

		// Update the originalIndex of the remaining items
		items.forEach((item, i) => {
			item.originalIndex = i
		})
	}

	function handleConsider(e) {
		const { items: newItems } = e.detail
		items = newItems
	}

	function handleFinalize(e) {
		const { items: newItems } = e.detail

		items = newItems
		tabs = items.map((item) => item.value)

		const oldSubgrids = { ...$app.subgrids }
		const newSubgrids = { ...oldSubgrids }

		items.forEach((item, newIndex) => {
			const oldIndex = item.originalIndex
			const newKey = `${component.id}-${newIndex}`
			const oldKey = `${component.id}-${oldIndex}`

			if (oldSubgrids.hasOwnProperty(oldKey)) {
				newSubgrids[newKey] = oldSubgrids[oldKey]
			} else {
				delete newSubgrids[newKey]
			}
		})

		$app.subgrids = newSubgrids
		$app = $app
	}

	let dragDisabled = true

	function startDrag(e) {
		// preventing default to prevent lag on touch devices (because of the browser checking for screen scrolling)
		e.preventDefault()
		dragDisabled = false
	}

	function handleKeyDown(e) {
		if ((e.key === 'Enter' || e.key === ' ') && dragDisabled) dragDisabled = false
	}
</script>

<PanelSection title={`Tabs ${tabs.length > 0 ? `(${tabs.length})` : ''}`}>
	{#if tabs.length == 0}
		<span class="text-xs text-gray-500">No Tabs</span>
	{/if}
	<div class="w-full flex gap-2 flex-col mt-2">
		<section
			use:dndzone={{
				items,
				flipDurationMs: 200,
				dropTargetStyle: {}
			}}
			on:consider={handleConsider}
			on:finalize={handleFinalize}
		>
			{#each items as item, index (item.id)}
				<div class="w-full flex flex-row gap-2 items-center relative my-1">
					<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
					<div
						tabindex={dragDisabled ? 0 : -1}
						class="w-4 h-4"
						on:mousedown={startDrag}
						on:touchstart={startDrag}
						on:keydown={handleKeyDown}
					>
						<GripVertical size={16} />
					</div>
					<input on:keydown|stopPropagation type="text" bind:value={item.value} />

					<div class="absolute right-1">
						<CloseButton noBg on:close={() => deleteSubgrid(index)} />
					</div>
				</div>
			{/each}
		</section>
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: faPlus }}
			on:click={addTab}
			iconOnly
		/>
	</div>
</PanelSection>
