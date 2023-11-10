<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import CloseButton from '$lib/components/common/CloseButton.svelte'
	import { getContext, tick } from 'svelte'
	import type { AppViewerContext, RichConfiguration } from '../../types'
	import { deleteGridItem } from '../appUtils'
	import type { AppComponent } from '../component'
	import PanelSection from './common/PanelSection.svelte'
	import { dndzone } from 'svelte-dnd-action'
	import { generateRandomString } from '$lib/utils'
	import { GripVertical, Plus } from 'lucide-svelte'
	import GridTabDisabled from './GridTabDisabled.svelte'

	export let tabs: string[] = []
	export let disabledTabs: RichConfiguration[] = []

	export let canDisableTabs: boolean = false

	export let word: string = 'Tab'

	export let component: AppComponent

	$: if (disabledTabs == undefined) {
		disabledTabs = [
			{ type: 'static', value: false, fieldType: 'boolean' },
			{ type: 'static', value: false, fieldType: 'boolean' }
		]
	}

	let items = tabs.map((tab, index) => {
		return { value: tab, id: generateRandomString(), originalIndex: index }
	})

	$: tabs = items.map((item) => item.value)

	const { app, runnableComponents, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	function addTab() {
		const numberOfTabs = items.length

		if (!$app.subgrids) {
			$app.subgrids = {}
		}

		$app.subgrids[`${component.id}-${numberOfTabs}`] = []
		items = [
			...items,
			{
				value: `${word} ${items.length + 1}`,
				id: generateRandomString(),
				originalIndex: items.length
			}
		]
		component.numberOfSubgrids = items.length

		disabledTabs = [...(disabledTabs ?? []), { type: 'static', value: false, fieldType: 'boolean' }]
	}

	function deleteSubgrid(index: number) {
		let subgrid = `${component.id}-${index}`
		for (const item of $app!.subgrids![subgrid]) {
			const components = deleteGridItem($app, item.data, subgrid)
			for (const key in components) {
				delete $runnableComponents[key]
			}
		}
		$runnableComponents = $runnableComponents

		for (let i = index; i < items.length - 1; i++) {
			$app!.subgrids![`${component.id}-${i}`] = $app!.subgrids![`${component.id}-${i + 1}`]
		}

		// Remove the corresponding item from the items array
		items = items.filter((item) => item.originalIndex !== index)

		// Delete the item in the disabledTabs array
		disabledTabs = (disabledTabs ?? []).filter((_, i) => i !== index)

		component.numberOfSubgrids = items.length
		// Update the originalIndex of the remaining items
		items.forEach((item, i) => {
			item.originalIndex = i
		})
		items = items

		delete $app!.subgrids![`${component.id}-${items.length}`]
		$app = $app
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

		// if the originalIndex are not in order, we should swap the subgrids
		if (items.some((item, index) => item.originalIndex !== index)) {
			const newSubgrids = {}
			for (let i = 0; i < items.length; i++) {
				newSubgrids[`${component.id}-${i}`] =
					$app!.subgrids![`${component.id}-${items[i].originalIndex}`] ?? []
			}

			const newDisabledTabs: RichConfiguration[] = []
			for (let i = 0; i < items.length; i++) {
				newDisabledTabs.push(disabledTabs[items[i].originalIndex])
			}
			disabledTabs = newDisabledTabs

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

<PanelSection title={`${word}s ${tabs && tabs.length > 0 ? `(${tabs.length})` : ''}`}>
	{#if !tabs || tabs.length == 0}
		<span class="text-xs text-tertiary">No Tabs</span>
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
				<div class="border rounded-md p-2 mb-2 bg-surface">
					<div class="w-full flex flex-row gap-2 items-center relative my-1">
						<input
							on:keydown|stopPropagation
							on:input={(e) => updateItemValue(index, e)}
							type="text"
							bind:value={items[index].value}
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

					{#if canDisableTabs && disabledTabs}
						<GridTabDisabled bind:field={disabledTabs[index]} id={component.id} />
					{/if}
				</div>
			{/each}
		</section>
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: Plus }}
			on:click={addTab}
			iconOnly
		/>
	</div>
</PanelSection>
