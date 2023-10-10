<script lang="ts">
	import { Popup } from '../common'
	import Label from '../Label.svelte'
	import Toggle from '../Toggle.svelte'
	import Tooltip from '../Tooltip.svelte'

	export let column: {
		headerName: string
		hideColumn: boolean
		type: 'text' | 'badge'
	}
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

		<Label label="Header name">
			<input placeholder="field" bind:value={column.headerName} />
		</Label>

		<Label label="Show column">
			<Toggle
				on:pointerdown={(e) => {
					e?.stopPropagation()
				}}
				options={{ right: 'Hide column' }}
				bind:checked={column.hideColumn}
				size="xs"
			/>
		</Label>

		<Label label="Type">
			<select bind:value={column.type}>
				<option value="text">Text</option>
				<option value="badge">Badge</option>
			</select>
		</Label>
	</div>
</Popup>
