<script lang="ts">
	import { Badge } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import { classNames } from '$lib/utils'
	import { faPlus, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import type { ButtonComponent, AppEditorContext, BaseAppComponent } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import TableActionLabel from './TableActionLabel.svelte'

	export let components: (BaseAppComponent & ButtonComponent)[]
	export let id: string

	const { selectedComponent, staticOutputs } = getContext<AppEditorContext>('AppEditorContext')

	function addComponent() {
		const actionId = getNextId(components.map((x) => x.id.split('-')[1]))

		const newComponent: BaseAppComponent & ButtonComponent = {
			id: `${id}-${actionId}`,
			type: 'buttoncomponent',
			configuration: {
				label: {
					type: 'static',
					value: 'Action',
					fieldType: 'text'
				},
				color: {
					fieldType: 'select',
					type: 'static',
					value: 'dark',
					optionValuesKey: 'buttonColorOptions'
				},
				size: {
					fieldType: 'select',
					type: 'static',
					value: 'xs',
					optionValuesKey: 'buttonSizeOptions'
				}
			},
			componentInput: {
				type: 'runnable',
				fieldType: 'any',
				fields: {},
				runnable: undefined,
				value: undefined
			},
			recomputeIds: undefined,
			card: false
		}

		components = [...components, newComponent]
	}

	function deleteComponent(cid: string) {
		components = components.filter((x) => x.id !== cid)
		delete $staticOutputs[cid]
		$staticOutputs = $staticOutputs
		$selectedComponent = id
	}
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

	{#each components as component}
		<div
			class={classNames(
				'w-full text-xs font-bold py-1.5 px-2 cursor-pointer transition-all justify-between flex items-center border border-gray-3 rounded-md',
				'bg-white border-gray-300  hover:bg-gray-100 focus:bg-gray-100 text-gray-700',
				$selectedComponent === component.id ? 'outline outline-blue-500 bg-red-400' : ''
			)}
			on:click={() => {
				$selectedComponent = component.id
			}}
			on:keypress
		>
			<div>
				<Button variant="border" color="red" on:click={() => deleteComponent(component.id)}>
					<Icon class="h-3" data={faTrashAlt} />
				</Button>
			</div>
			<div>
				<TableActionLabel componentInput={component.configuration.label} />
			</div>
			<Badge color="dark-blue">
				Component: {component.id}
			</Badge>
		</div>
	{/each}
</PanelSection>
