<script lang="ts">
	import { Popup } from '../common'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import Label from '../Label.svelte'
	import Tooltip from '../Tooltip.svelte'

	import Button from '../common/button/Button.svelte'

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
		valueFormatter: string | null
		valueParser: string
		field: string
		headerName: string
	}

	const presets = [
		{
			label: 'None',
			value: null
		},
		{
			label: 'Currency CHF',
			value: 'value + " CHF"'
		},
		{
			label: 'Currency USD',
			value: '"$ " + value'
		},
		{
			label: 'Date',
			value: 'new Date(value).toLocaleDateString()'
		},
		{
			label: 'Percentage',
			value: 'value + " %"'
		},
		{
			label: 'Currency GBP',
			value: 'value + " £"'
		},
		{
			label: 'Currency EUR',
			value: 'value + " €"'
		},
		{
			label: 'Currency JPY',
			value: 'value + " ¥"'
		},
		{
			label: 'Decimal places (2)',
			value: 'parseFloat(value).toFixed(2)'
		},
		{
			label: 'Uppercase',
			value: 'value.toUpperCase()'
		},
		{
			label: 'Lowercase',
			value: 'value.toLowerCase()'
		},
		{
			label: 'Boolean (True/False)',
			value: 'value ? "True" : "False"'
		}
	]

	let renderCount = 0
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

		<Label label="Header name">
			<input type="text" placeholder="Header name" bind:value={value.headerName} />
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

		<Label label="Value formatter">
			<svelte:fragment slot="header">
				<Tooltip documentationLink="https://www.ag-grid.com/javascript-data-grid/value-formatters/">
					Value formatters allow you to format values for display. This is useful when data is one
					type (e.g. numeric) but needs to be converted for human reading (e.g. putting in currency
					symbols and number formatting).
				</Tooltip>
			</svelte:fragment>
			<svelte:fragment slot="action">
				<Button
					size="xs"
					color="light"
					variant="border"
					on:click={() => {
						value.valueFormatter = null
						renderCount++
					}}
				>
					Clear
				</Button>
			</svelte:fragment>
		</Label>
		<div>
			{#key renderCount}
				<div class="flex flex-col gap-4">
					<div>
						<div class="text-xs font-semibold">Presets</div>
						<select
							bind:value={value.valueFormatter}
							on:change={() => {
								renderCount++
							}}
						>
							{#each presets as preset}
								<option value={preset.value}>{preset.label}</option>
							{/each}
						</select>
					</div>

					{#if value.valueFormatter}
						<SimpleEditor autoHeight lang="javascript" bind:code={value.valueFormatter} />
					{/if}
				</div>
			{/key}
		</div>

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
