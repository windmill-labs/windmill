<script lang="ts">
	import type { TableColumns } from '../apps/editor/component/components'
	import InputsSpecsEditor from '../apps/editor/settingsPanel/InputsSpecsEditor.svelte'
	import { Popup } from '../common'
	import Tooltip from '../Tooltip.svelte'

	export let columns: TableColumns
	export let id: string
</script>

<Popup
	floatingConfig={{ strategy: 'fixed', placement: 'left-end' }}
	containerClasses="border rounded-lg shadow-lg bg-surface p-4"
>
	<svelte:fragment slot="button">
		<slot name="trigger" />
	</svelte:fragment>
	<div class="flex flex-col w-96 p-2 gap-4">
		<span class="text-sm mb-2 leading-6 font-semibold">
			Table Column
			<Tooltip documentationLink="https://www.ag-grid.com/javascript-data-grid/column-definitions/">
				Column definitions are used to define columns in ag-Grid.
			</Tooltip>
		</span>

		<InputsSpecsEditor
			{id}
			shouldCapitalize={false}
			bind:inputSpecs={columns}
			userInputEnabled={false}
		/>
	</div>
</Popup>
