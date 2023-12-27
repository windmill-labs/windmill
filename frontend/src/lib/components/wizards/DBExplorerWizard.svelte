<script lang="ts">
	import { Badge, Popup } from '../common'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import Label from '../Label.svelte'
	import Section from '../Section.svelte'
	import Tooltip from '../Tooltip.svelte'
	import Button from '../common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'

	type Column = {
		minWidth: number
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
		headerName: string
		// DBExplorer
		ignored: boolean
		insert: boolean
		defaultValue: any
	}

	export let value: Column | undefined

	import { tableMetadataShared } from '../apps/components/display/dbtable/AppDbExplorer.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import { onMount } from 'svelte'
	import { ColumnIdentity } from '../apps/components/display/dbtable/utils'

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
	$: columnMetadata = tableMetadataShared?.find((x) => x.columnname === value?.field)

	let shouldHaveDefaultValue = false

	onMount(() => {
		const col = tableMetadataShared?.find((x) => x.columnname === value?.field)
		if (col?.isnullable === 'NO' && !col?.defaultvalue && col.isidentity !== ColumnIdentity.No) {
			shouldHaveDefaultValue = true
		}
	})
</script>

<Popup
	floatingConfig={{ strategy: 'fixed', placement: 'left-end' }}
	containerClasses="border rounded-lg shadow-lg bg-surface p-4 h-96 max-h-96 overflow-y-auto"
>
	<svelte:fragment slot="button">
		<slot name="trigger" />
	</svelte:fragment>
	{#if value}
		<div class="flex flex-col w-96 p-2 gap-4">
			<Section label="Column settings">
				<svelte:fragment slot="header">
					<Badge color="blue">
						{value.headerName}
					</Badge>
				</svelte:fragment>
				<Label label="Skip for select and update">
					<svelte:fragment slot="header">
						<Tooltip>
							By default, all columns are included in the select and update queries. If you want to
							exclude a column from the select and update queries, you can set this property to
							true.
						</Tooltip>
					</svelte:fragment>
					<svelte:fragment slot="action">
						<Toggle
							on:pointerdown={(e) => {
								e?.stopPropagation()
							}}
							bind:checked={value.ignored}
							size="xs"
							disabled={columnMetadata?.isprimarykey}
						/>
					</svelte:fragment>
					{#if columnMetadata?.isprimarykey}
						<Alert type="warning" size="xs" title="Primary key" class="mt-2">
							You cannot skip a primary key.
						</Alert>
					{/if}
				</Label>

				<Label label="Should hide from insert">
					<svelte:fragment slot="header">
						<Tooltip>
							By default, all columns are used to generate the submit form. If you want to exclude a
							column from the submit form, you can set this property to true. If the column is not
							nullable or doesn't have a default value, a default value will be required.
						</Tooltip>
					</svelte:fragment>
					<svelte:fragment slot="action">
						<Toggle
							on:pointerdown={(e) => {
								e?.stopPropagation()
							}}
							bind:checked={value.insert}
							size="xs"
						/>
					</svelte:fragment>

					{#if tableMetadataShared?.find((x) => x.columnname === value?.field)?.datatype}
						{@const type = tableMetadataShared?.find(
							(x) => x.columnname === value?.field
						)?.datatype}

						<span class="text-xs font-semibold">
							Type:
							{type}
						</span>
						{#if type === 'integer'}
							<input
								type="number"
								placeholder="123"
								class="mt-2"
								bind:value={value.defaultValue}
								disabled={!value.insert}
							/>
						{:else}
							<input
								type="text"
								placeholder="Default value"
								class="mt-2"
								bind:value={value.defaultValue}
								disabled={!value.insert}
							/>
						{/if}
					{/if}

					{#if tableMetadataShared?.find((x) => x.columnname === value?.field)?.defaultvalue}
						<Alert type="info" size="xs" title="Default value" class="mt-2">
							The column has a default value defined in the database. You can override it here. The
							default value is: <span class="font-semibold">
								{tableMetadataShared?.find((x) => x.columnname === value?.field)?.defaultvalue}
							</span>
						</Alert>
					{/if}

					{#if shouldHaveDefaultValue && value.insert && !value.defaultValue}
						<Alert type="warning" size="xs" title="No default value" class="mt-2">
							The column is not nullable and doesn't have a default value. A default value is
							required.
						</Alert>
					{/if}
				</Label>
			</Section>

			<Section label="AG Grid configuration">
				<div
					class={twMerge('flex flex-col gap-4', value.ignored ? 'opacity-50 cursor-none ' : '')}
					on:pointerdown={(e) => {
						if (value?.ignored) {
							e?.stopPropagation()
						}
					}}
				>
					<Label label="Header name">
						<input type="text" placeholder="Header name" bind:value={value.headerName} />
					</Label>

					<Label label="Min width (px)">
						<input type="number" placeholder="width" bind:value={value.minWidth} />
					</Label>

					<Label label="Flex">
						<svelte:fragment slot="header">
							<Tooltip
								documentationLink="https://www.ag-grid.com/javascript-data-grid/column-sizing/#column-flex"
							>
								It's often required that one or more columns fill the entire available space in the
								grid. For this scenario, it is possible to use the flex config. Some columns could
								be set with a regular width config, while other columns would have a flex config.
								Flex sizing works by dividing the remaining space in the grid among all flex columns
								in proportion to their flex value. For example, suppose the grid has a total width
								of 450px and it has three columns: the first with width: 150; the second with flex:
								1; and third with flex: 2. The first column will be 150px wide, leaving 300px
								remaining. The column with flex: 2 has twice the size with flex: 1. So final sizes
								will be: 150px, 100px, 200px.
							</Tooltip>
						</svelte:fragment>

						<input type="range" step="1" bind:value={value.flex} min={1} max={12} />
						<div class="text-xs">{value.flex}</div>
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
							<Tooltip
								documentationLink="https://www.ag-grid.com/javascript-data-grid/value-formatters/"
							>
								Value formatters allow you to format values for display. This is useful when data is
								one type (e.g. numeric) but needs to be converted for human reading (e.g. putting in
								currency symbols and number formatting).
							</Tooltip>
						</svelte:fragment>
						<svelte:fragment slot="action">
							<Button
								size="xs"
								color="light"
								variant="border"
								on:click={() => {
									// @ts-ignore
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
								<div class="relative">
									{#if !presets.find((preset) => preset.value === value?.valueFormatter)}
										<div
											class="z-50 absolute bg-opacity-50 bg-surface top-0 left-0 bottom-0 right-0"
										/>
									{/if}
									<div class="text-xs font-semibold">Presets</div>
									<select
										bind:value={value.valueFormatter}
										on:change={() => {
											renderCount++
										}}
										placeholder="Code"
									>
										{#each presets as preset}
											<option value={preset.value}>{preset.label}</option>
										{/each}
									</select>
								</div>

								<SimpleEditor autoHeight lang="javascript" bind:code={value.valueFormatter} />
							</div>
						{/key}
					</div>

					<Label label="Sort">
						<select bind:value={value.sort}>
							<option value={null}>None</option>
							<option value="asc">Ascending</option>
							<option value="desc">Descending</option>
						</select>
					</Label>
				</div>
			</Section>
		</div>
	{/if}
</Popup>
