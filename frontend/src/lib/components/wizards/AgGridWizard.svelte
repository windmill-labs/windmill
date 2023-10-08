<script lang="ts">
	import { Popup } from '../common'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import Label from '../Label.svelte'
	import Tooltip from '../Tooltip.svelte'

	export let value: {
		width: number
		hide: boolean
		flex: number
		sort: 'asc' | 'desc'
		sortIndex: number
		aggFunc: string
		pivot: boolean
		pivotIndex: number
		pinned: 'left' | 'right' | boolean
		rowGroup: boolean
		rowGroupIndex: number
		valueFormatter: string
		valueParser: string
		field: string
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
			Column definition
			<Tooltip documentationLink="https://www.ag-grid.com/javascript-data-grid/column-definitions/">
				Column definitions are used to define columns in ag-Grid.
			</Tooltip>
		</span>

		<Label label="Field">
			<input type="text" placeholder="field" bind:value={value.field} />
		</Label>

		<Label label="Width">
			<svelte:fragment slot="header">
				<Tooltip>This property will be ignored if you set the column to flex.</Tooltip>
			</svelte:fragment>
			<input type="number" placeholder="width" bind:value={value.width} />
		</Label>

		<Label label="Hide">
			<Toggle
				on:pointerdown={(e) => {
					e?.stopPropagation()
				}}
				options={{ right: 'Hide' }}
				bind:checked={value.hide}
				size="xs"
			/>
		</Label>

		<Label label="Value Formatter">
			<svelte:fragment slot="header">
				<Tooltip documentationLink="https://www.ag-grid.com/javascript-data-grid/value-formatters/">
					Value formatters allow you to format values for display. This is useful when data is one
					type (e.g. numeric) but needs to be converted for human reading (e.g. putting in currency
					symbols and number formatting).
				</Tooltip>
			</svelte:fragment>

			<SimpleEditor autoHeight lang="javascript" bind:code={value.valueFormatter} />
		</Label>

		<Label label="Value Parser">
			<svelte:fragment slot="header">
				<Tooltip
					documentationLink="https://www.ag-grid.com/javascript-data-grid/value-parsers/#value-parser"
				>
					See the documentation for more information.
				</Tooltip>
			</svelte:fragment>

			<SimpleEditor autoHeight lang="javascript" bind:code={value.valueParser} />
		</Label>

		<Label label="Sort">
			<select bind:value={value.sort}>
				<option value={null}>None</option>
				<option value="asc">Ascending</option>
				<option value="desc">Descending</option>
			</select>
		</Label>

		<!--
		EE only

		<Label label="Aggregation Function">
			<SimpleEditor autoHeight lang="javascript" bind:code={value.aggFunc} />
		</Label>

		<Label label="Pivot">
			<Toggle bind:checked={value.pivot} size="xs" />
		</Label>

		<Label label="Pivot Index">
			<input type="number" placeholder="pivot index" bind:value={value.pivotIndex} />
		</Label>

		<Label label="Pinned">
			<select bind:value={value.pinned}>
				<option value={null}>None</option>
				<option value="left">Left</option>
				<option value="right">Right</option>
			</select>
		</Label>

		<Label label="Row Group">
			<Toggle bind:checked={value.rowGroup} size="xs" />
		</Label>

		<Label label="Row Group Index">
			<input type="number" placeholder="row group index" bind:value={value.rowGroupIndex} />
		</Label>
		 -->
	</div>
</Popup>
