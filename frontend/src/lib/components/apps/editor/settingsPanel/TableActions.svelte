<script lang="ts">
	import { Badge } from '$lib/components/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import { classNames } from '$lib/utils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { ButtonComponent, AppEditorContext, BaseAppComponent } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import ComponentPanel from './ComponentPanel.svelte'
	import TableActionLabel from './TableActionLabel.svelte'

	export let components: (BaseAppComponent & ButtonComponent)[]

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	function addComponent() {
		const grid = $app.grid ?? []
		const id = getNextId(grid.map((gridItem) => gridItem.data.id))

		const newComponent: BaseAppComponent & ButtonComponent = {
			id,
			type: 'buttoncomponent',
			configuration: {
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
			componentInput: {
				type: 'static',
				fieldType: 'textarea',
				defaultValue: '',
				value: ''
			},
			recomputeIds: undefined,
			card: false
		}

		components = [...components, newComponent]
	}

	let openedComponentId: string | undefined = components[0]?.id
</script>

<PanelSection title={`Table actions ${components.length > 0 ? `(${components.length})` : ''}`}>
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
				The row is passed as an argument to the runnable.
			</Alert>
		</div>
	{/if}
	{#each components as component}
		<div
			class={classNames(
				'w-full text-xs font-bold py-1.5 px-2 cursor-pointer transition-all justify-between flex items-center border border-gray-3 rounded-md',
				'bg-white border-gray-300  hover:bg-gray-100 focus:bg-gray-100 text-gray-700',
				openedComponentId === component.id ? 'outline outline-gray-500 outline-offset-1' : ''
			)}
			on:click={() => {
				if (openedComponentId === component.id) {
					openedComponentId = undefined
				} else {
					openedComponentId = component.id
				}
			}}
			on:keypress
		>
			<div>
				<TableActionLabel componentInput={component.configuration.label} />
			</div>
			<Badge color="dark-blue">
				Component: {component.id}
			</Badge>
		</div>

		{#if openedComponentId === component.id}
			<div class="w-full border">
				<ComponentPanel
					bind:component
					onDelete={() => {
						components = components.filter((c) => c.id !== component.id)
					}}
				/>
			</div>
		{/if}
	{/each}
</PanelSection>
