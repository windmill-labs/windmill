<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import CloseButton from '$lib/components/common/CloseButton.svelte'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { getContext, tick } from 'svelte'
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

	const { app, runnableComponents, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

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

	function handleConsider(e: CustomEvent): void {
		const { items: newItems } = e.detail

		items = newItems
	}

	function updateItemValue(index: number, e: Event): void {
		const newValue = (e.target as HTMLInputElement).value
		items[index].value = newValue
		items = [...items]
	}
	function handleFinalize(e: CustomEvent) {
		const { items: newItems } = e.detail

		items = newItems
		tabs = items.map((item) => item.value)

		// if the originalIndex are not in order, we should swap the subgrids
		if (items.some((item, index) => item.originalIndex !== index)) {
			const newSubgrids = {}
			for (let i = 0; i < tabs.length; i++) {
				newSubgrids[`${component.id}-${i}`] =
					$app!.subgrids![`${component.id}-${items[i].originalIndex}`] ?? []
			}

			// update originalIndex
			items.forEach((item, i) => {
				item.originalIndex = i
			})

			$app!.subgrids = {
				...$app!.subgrids,
				...newSubgrids
			}
			$app = $app

			tick().then(() => {
				const targetIndex = items.findIndex((i) => i.id === e.detail.info.id)
				$componentControl[component.id]?.setTab?.(targetIndex)
			})
		}

		dragDisabled = true
	}

	let dragDisabled = true

	function startDrag(event) {
		event.preventDefault()
		dragDisabled = false
	}

	function handleKeyDown(event: KeyboardEvent): void {
		if ((event.key === 'Enter' || event.key === ' ') && dragDisabled) {
			dragDisabled = false
		}
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
					<input
						on:keydown|stopPropagation
						on:input={(e) => updateItemValue(index, e)}
						type="text"
						bind:value={tabs[index]}
					/>
					<div class="absolute right-6">
						<CloseButton noBg on:close={() => deleteSubgrid(index)} />
					</div>
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
