<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext } from '../../../types'
	import type { TableAction } from '$lib/components/apps/editor/component'

	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'

	import AppButton from '../../buttons/AppButton.svelte'
	import { AppCheckbox, AppSelect } from '../..'
	import Portal from 'svelte-portal'

	export let id: string
	export let render: boolean
	export let actions: TableAction[] = []
	export let rowIndex: number
	export let row: { original: Record<string, any> }

	const context = getContext<AppViewerContext>('AppViewerContext')

	const dispatch = createEventDispatcher()

	const { selectedComponent } = context
</script>

<div id={`aggrid-action-${rowIndex}`} />

<Portal target={`#aggrid-action-${rowIndex}`}>
	<div class="flex flex-row relative">
		{#each actions as action, actionIndex}
			{@const controls = {
				left: () => {
					if (actionIndex === 0) {
						$selectedComponent = [id]
						return true
					} else if (actionIndex > 0) {
						$selectedComponent = [actions[actionIndex - 1].id]
						return true
					}
					return false
				},
				right: () => {
					if (actionIndex === actions.length - 1) {
						return id
					} else if (actionIndex < actions.length - 1) {
						$selectedComponent = [actions[actionIndex + 1].id]
						return true
					}
					return false
				}
			}}
			{#if action.type == 'buttoncomponent'}
				<AppButton
					noInitialize
					extraKey={'idx' + rowIndex}
					{render}
					noWFull
					preclickAction={async () => {
						dispatch('toggleRow')
					}}
					id={action.id}
					customCss={action.customCss}
					configuration={action.configuration}
					recomputeIds={action.recomputeIds}
					extraQueryParams={{ row: row.original }}
					componentInput={action.componentInput}
					{controls}
				/>
			{:else if action.type == 'checkboxcomponent'}
				<AppCheckbox
					noInitialize
					extraKey={'idx' + rowIndex}
					{render}
					id={action.id}
					customCss={action.customCss}
					configuration={action.configuration}
					recomputeIds={action.recomputeIds}
					onToggle={action.onToggle}
					preclickAction={async () => {
						dispatch('toggleRow')
					}}
					{controls}
				/>
			{:else if action.type == 'selectcomponent'}
				<div class="w-40">
					<AppSelect
						noDefault
						noInitialize
						extraKey={'idx' + rowIndex}
						{render}
						id={action.id}
						customCss={action.customCss}
						configuration={action.configuration}
						recomputeIds={action.recomputeIds}
						onSelect={action.onSelect}
						preclickAction={async () => {
							dispatch('toggleRow')
						}}
						{controls}
					/>
				</div>
			{/if}
		{/each}
	</div>
</Portal>
