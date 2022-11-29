<script lang="ts">
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppComponent, AppEditorContext } from '../../types'
	import { defaultProps } from '../componentsPanel/componentDefaultProps'
	import PanelSection from './common/PanelSection.svelte'
	import ComponentPanel from './ComponentPanel.svelte'

	export let components: AppComponent[]

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	function addComponent() {
		const grid = $app.grid ?? []
		const id = getNextId(grid.map((gridItem) => gridItem.data.id))

		const newComponent: AppComponent = {
			...defaultProps,
			id,
			type: 'buttoncomponent',
			componentInputs: {
				label: {
					type: 'static',
					visible: true,
					value: 'Action',
					fieldType: 'textarea',
					defaultValue: 'Action'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'dark',
					optionValuesKey: 'buttonColorOptions',
					defaultValue: 'dark'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'xs',
					optionValuesKey: 'buttonSizeOptions',
					defaultValue: 'xs'
				}
			},
			runnable: true
		}

		components = [...components, newComponent]
	}
</script>

<PanelSection title="Table actions">
	<svelte:fragment slot="action">
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: faPlus }}
			on:click={addComponent}
			iconOnly
		/>
	</svelte:fragment>
	{#if components.length > 0}
		<div class="w-full">
			<Alert title="Special argument" size="xs">
				A "row" argument is automatically added to the script. It contains the row data.
			</Alert>
		</div>
	{/if}
	{#each components as component}
		<div class="w-full border">
			<div class="border-b py-1 px-2 text-sm text-white bg-gray-500">Component: {component.id}</div>

			<ComponentPanel
				bind:component
				onDelete={() => {
					components = components.filter((c) => c.id !== component.id)
				}}
			/>
		</div>
	{/each}
</PanelSection>
