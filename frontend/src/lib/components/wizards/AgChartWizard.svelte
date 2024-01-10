<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import { Popup } from '../common'
	import Label from '../Label.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import Button from '../common/button/Button.svelte'
	import OneOfInputSpecsEditor from '../apps/editor/settingsPanel/OneOfInputSpecsEditor.svelte'
	import type { AppViewerContext, GridItem, RichConfiguration } from '../apps/types'
	import { findGridItem } from '../apps/editor/appUtils'

	const { selectedComponent, app } = getContext<AppViewerContext>('AppViewerContext')

	type Dataset = {
		value: RichConfiguration
		name: string
		type: 'bar' | 'scatter' | 'line' | 'area' | 'range-bar'
	}

	let component: GridItem | undefined = undefined

	$: if (component === undefined && $selectedComponent && $app) {
		component = findGridItem($app, $selectedComponent[0])
	}

	$: isEE = component?.data.type === 'agchartscomponentee'

	export let value: Dataset | undefined = undefined

	const dispatch = createEventDispatcher()

	function removeDataset() {
		dispatch('remove')
	}
</script>

<Popup
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
	containerClasses="border rounded-lg shadow-lg bg-surface p-4"
>
	<svelte:fragment slot="button">
		<slot name="trigger" />
	</svelte:fragment>
	{#if value}
		<div class="flex flex-col w-96 p-2 gap-4">
			<Label label="Name">
				<input type="text" bind:value={value.name} />
			</Label>

			<OneOfInputSpecsEditor
				key={'Data'}
				bind:oneOf={value.value}
				id={$selectedComponent?.[0] ?? ''}
				shouldCapitalize={true}
				resourceOnly={false}
				inputSpecsConfiguration={value.value?.['configuration']}
				labels={value.value?.['labels']}
				tooltip={value.value?.['tooltip']}
				disabledOptions={isEE ? [] : ['range-bar']}
			/>

			<Button color="red" size="xs" on:click={removeDataset}>Remove dataset</Button>
		</div>
	{/if}
</Popup>
