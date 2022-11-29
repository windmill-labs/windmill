<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { ButtonComponent, AppEditorContext, BaseAppComponent } from '../../types'
	import { defaultProps } from '../componentsPanel/componentDefaultProps'
	import PanelSection from './common/PanelSection.svelte'
	import ComponentPanel from './ComponentPanel.svelte'

	export let components: (BaseAppComponent & ButtonComponent)[]

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	function addComponent() {
		const grid = $app.grid ?? []
		const id = getNextId(grid.map((gridItem) => gridItem.data.id))

		const newComponent: (BaseAppComponent & ButtonComponent) = {
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
		<Button size="xs" color="dark" startIcon={{ icon: faPlus }} on:click={addComponent}>
			Create an action
		</Button>
	</svelte:fragment>
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
