<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppComponent, AppEditorContext } from '../../types'
	import { defaultProps } from '../componentsPanel/componentDefaultProps'
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
					value: 'Lorem ipsum',
					fieldType: 'textarea'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'blue',
					optionValuesKey: 'buttonColorOptions'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					visible: true,
					value: 'md',
					optionValuesKey: 'buttonSizeOptions'
				}
			},
			runnable: true
		}

		components = [...components, newComponent]
	}
</script>

<Button size="xs" color="dark" startIcon={{ icon: faPlus }} on:click={addComponent}>
	Create an action
</Button>

{#each components as component}
	<ComponentPanel {component} />
{/each}
