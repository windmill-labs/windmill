<script lang="ts">
	import type { AppInputSpec } from '../../inputType'
	import Button from '$lib/components/common/button/Button.svelte'
	import PanelSection from './common/PanelSection.svelte'
	import { dndzone } from 'svelte-dnd-action'
	import { GripVertical, Plus, X } from 'lucide-svelte'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import { generateRandomString } from '$lib/utils'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { AppViewerContext, RichConfiguration } from '../../types'
	import { getContext, tick } from 'svelte'
	import { deleteGridItem } from '../appUtils'
	import type { AppComponent } from '../component'

	export let conditions: RichConfiguration[] = []
	export let component: AppComponent
	let dragDisabled = true

	let items = conditions.map((condition, index) => {
		return { value: condition, id: generateRandomString(), originalIndex: index }
	})

	$: conditions = items.map((item) => item.value)

	const { app, runnableComponents, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	function handleConsider(e: CustomEvent): void {
		const { items: newItems } = e.detail
		items = newItems
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

	function startDrag(event) {
		event.preventDefault()
		dragDisabled = false
	}

	function handleKeyDown(event: KeyboardEvent): void {
		if ((event.key === 'Enter' || event.key === ' ') && dragDisabled) {
			dragDisabled = false
		}
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

		component.numberOfSubgrids = items.length
		// Update the originalIndex of the remaining items
		items.forEach((item, i) => {
			item.originalIndex = i
		})
		items = items

		delete $app!.subgrids![`${component.id}-${items.length}`]
		$app = $app
	}

	function addCondition(): void {
		const numberOfConditions = items.length

		if (!$app.subgrids) {
			$app.subgrids = {}
		}

		const lastSubgrid = JSON.parse(
			JSON.stringify($app.subgrids[`${component.id}-${numberOfConditions - 1}`])
		)

		$app.subgrids[`${component.id}-${numberOfConditions - 1}`] = []
		$app.subgrids[`${component.id}-${numberOfConditions}`] = lastSubgrid
		component.numberOfSubgrids = items.length

		const newCondition: AppInputSpec<'boolean', boolean> = {
			type: 'eval',
			expr: 'false',
			fieldType: 'boolean'
		}

		items.splice(conditions.length - 1, 0, {
			value: newCondition,
			id: generateRandomString(),
			originalIndex: items.length - 1
		})

		component.numberOfSubgrids = items.length
	}
</script>

<PanelSection title={'Conditions'}>
	<Alert title="Evaluation" size="xs">
		Conditions are evaluated in order. The first condition that evaluates to true will render its
		subgrid.
	</Alert>
	{#if items.length == 0}
		<span class="text-xs text-tertiary">No Tabs</span>
	{/if}
	<div class="w-full flex flex-col mt-2">
		<section
			use:dndzone={{
				items: items,
				flipDurationMs: 200,
				dropTargetStyle: {}
			}}
			on:consider={handleConsider}
			on:finalize={handleFinalize}
		>
			{#each items as item, index (item.id)}
				{#if index < items.length - 1}
					{@const condition = item.value}
					<div class="w-full flex flex-row gap-2 items-center relative">
						<div class={twMerge('grow border p-3 my-2 rounded-md bg-surface')}>
							<InputsSpecEditor
								key={`Condition ${index + 1}`}
								bind:componentInput={item.value}
								id={component.id}
								userInputEnabled={false}
								shouldCapitalize={true}
								resourceOnly={false}
								fieldType={condition?.['fieldType']}
								subFieldType={condition?.['subFieldType']}
								format={condition?.['format']}
								selectOptions={condition?.['selectOptions']}
								tooltip={condition?.['tooltip']}
								fileUpload={condition?.['fileUpload']}
								placeholder={condition?.['placeholder']}
								customTitle={condition?.['customTitle']}
								displayType={false}
							/>
						</div>

						<div class="flex flex-col justify-center gap-2">
							<!-- svelte-ignore a11y-click-events-have-key-events -->
							<div on:click={() => deleteSubgrid(index)}>
								<X size={16} />
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
					</div>
				{/if}
			{/each}
		</section>
		<div class="border rounded-md p-3 mb-2">
			<div class="flex">
				<span class="font-bold text-xs">Default</span>
			</div>
			<Alert title="Default" size="xs">
				The default container is rendered if no other conditions evaluate to true.
			</Alert>
		</div>
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: Plus }}
			on:click={addCondition}
			iconOnly
		/>
	</div>
</PanelSection>
