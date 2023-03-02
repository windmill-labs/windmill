<script lang="ts">
	import { Badge } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { getNextId } from '$lib/components/flows/flowStateUtils'
	import { classNames } from '$lib/utils'
	import { faPlus, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import type { AppEditorContext, BaseAppComponent } from '../../types'
	import type { ButtonComponent } from '../component'
	import PanelSection from './common/PanelSection.svelte'
	import TableActionLabel from './TableActionLabel.svelte'

	export let components: (BaseAppComponent & ButtonComponent)[]
	export let id: string

	const { selectedComponent, staticOutputs } = getContext<AppEditorContext>('AppEditorContext')

	function addComponent() {
		const actionId = getNextId(components.map((x) => x.id.split('_')[1]))

		const newComponent: BaseAppComponent & ButtonComponent = {
			id: `${id}_${actionId}`,
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
				},
				disabled: {
					fieldType: 'boolean',
					type: 'eval',
					expr: 'false'
				},
				goto: {
					tooltip: 'Go to an url on success if not empty',
					fieldType: 'text',
					type: 'static',
					value: ''
				},
				gotoNewTab: {
					tooltip: 'Go to create a new tab',
					fieldType: 'boolean',
					type: 'static',
					value: true
				},
				beforeIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select',
					onlyStatic: true
				},
				afterIcon: {
					type: 'static',
					value: undefined,
					fieldType: 'icon-select',
					onlyStatic: true
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
			customCss: {
				button: { style: '', class: '' }
			} as const,
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

	{#if components.length == 0}
		<span class="text-xs text-gray-500">No action buttons</span>
	{/if}
	{#each components as component}
		<div
			class={classNames(
				'w-full text-xs font-bold gap-1 truncate py-1.5 px-2 cursor-pointer transition-all justify-between flex items-center border border-gray-3 rounded-md',
				'bg-white border-gray-300  hover:bg-gray-100 focus:bg-gray-100 text-gray-700',
				$selectedComponent === component.id ? 'outline outline-blue-500 bg-red-400' : ''
			)}
			on:click={() => {
				$selectedComponent = component.id
			}}
			on:keypress
		>
			<Badge color="dark-indigo">
				{component.id}
			</Badge>

			<div>
				<TableActionLabel componentInput={component.configuration.label} />
			</div>
			<div>
				<Button variant="border" color="red" on:click={() => deleteComponent(component.id)}>
					<Icon class="h-3" data={faTrashAlt} />
				</Button>
			</div>
		</div>
	{/each}
	<div class="w-full">
		<Button
			btnClasses="w-full"
			color="light"
			variant="border"
			startIcon={{ icon: faPlus }}
			on:click={addComponent}
			iconOnly
		/>
	</div>
</PanelSection>
