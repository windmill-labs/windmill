<script lang="ts">
	import { Alert } from '../common'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import Label from '../Label.svelte'
	import Tooltip from '../Tooltip.svelte'
	import Button from '../common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'

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
		editable: boolean
		filter: boolean
		cellRendererType: 'text' | 'badge' | 'link'
	}

	interface Props {
		value: Column | undefined
		trigger?: import('svelte').Snippet
	}

	let { value = $bindable(), trigger: trigger_render }: Props = $props()

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

	let renderCount = $state(0)

	$effect(() => {
		if (value && value.cellRendererType === null) {
			value.cellRendererType = 'text'
		}
	})
</script>

<Popover
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
	closeOnOtherPopoverOpen
>
	{#snippet trigger()}
		{@render trigger_render?.()}
	{/snippet}
	{#snippet content()}
		{#if value}
			<div class="flex flex-col w-96 p-4 gap-4 max-h-[70vh] overflow-y-auto">
				<span class="text-sm mb-2 leading-6 font-semibold">
					Column definitions
					<Tooltip
						documentationLink="https://www.ag-grid.com/javascript-data-grid/column-definitions/"
					>
						Column definitions are used to define columns in ag-Grid.
					</Tooltip>
				</span>

				<Label label="Header name">
					<input type="text" placeholder="Header name" bind:value={value.headerName} />
				</Label>

				<Label label="Editable value">
					<Toggle
						on:pointerdown={(e) => {
							e?.stopPropagation()
						}}
						options={{ right: 'Editable' }}
						bind:checked={value.editable}
						size="xs"
					/>
				</Label>

				<Label label="Min width (px)">
					<input type="number" placeholder="width" bind:value={value.minWidth} />
				</Label>

				<Label label="Flex">
					{#snippet header()}
						<Tooltip
							documentationLink="https://www.ag-grid.com/javascript-data-grid/column-sizing/#column-flex"
						>
							It's often required that one or more columns fill the entire available space in the
							grid. For this scenario, it is possible to use the flex config. Some columns could be
							set with a regular width config, while other columns would have a flex config. Flex
							sizing works by dividing the remaining space in the grid among all flex columns in
							proportion to their flex value. For example, suppose the grid has a total width of
							450px and it has three columns: the first with width: 150; the second with flex: 1;
							and third with flex: 2. The first column will be 150px wide, leaving 300px remaining.
							The column with flex: 2 has twice the size with flex: 1. So final sizes will be:
							150px, 100px, 200px.
						</Tooltip>
					{/snippet}

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
					{#snippet header()}
						<Tooltip
							documentationLink="https://www.ag-grid.com/javascript-data-grid/value-formatters/"
						>
							Value formatters allow you to format values for display. This is useful when data is
							one type (e.g. numeric) but needs to be converted for human reading (e.g. putting in
							currency symbols and number formatting).
						</Tooltip>
					{/snippet}
					{#snippet action()}
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
					{/snippet}
				</Label>
				<div>
					{#key renderCount}
						<div class="flex flex-col gap-4">
							<div class="relative">
								{#if !presets.find((preset) => preset.value === value?.valueFormatter)}
									<div class="z-50 absolute bg-opacity-50 bg-surface top-0 left-0 bottom-0 right-0"
									></div>
								{/if}
								<div class="text-xs font-semibold">Presets</div>
								<select
									bind:value={value.valueFormatter}
									onchange={() => {
										renderCount++
									}}
									placeholder="Code"
								>
									{#each presets as preset}
										<option value={preset.value}>{preset.label}</option>
									{/each}
								</select>
							</div>

							<SimpleEditor
								extraLib={'declare const value: any'}
								autoHeight
								lang="javascript"
								bind:code={value.valueFormatter}
							/>
							<div class="text-xs text-secondary -mt-4">Use `value` in the formatter</div>
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

				<Label label="Filter">
					{#snippet header()}
						<Tooltip documentationLink="https://www.ag-grid.com/javascript-data-grid/filtering/">
							Filtering allows you to limit the rows displayed in your grid to those that match
							criteria you specify.
						</Tooltip>
					{/snippet}
					<Toggle
						on:pointerdown={(e) => {
							e?.stopPropagation()
						}}
						options={{ right: 'Enable filter' }}
						bind:checked={value.filter}
						size="xs"
					/>
				</Label>

				<!--
			EE only

			<Label label="Aggregation function">
				<SimpleEditor autoHeight lang="javascript" bind:code={value.aggFunc} />
			</Label>

			<Label label="Pivot">
				<Toggle bind:checked={value.pivot} size="xs" />
			</Label>

			<Label label="Pivot index">
				<input type="number" placeholder="pivot index" bind:value={value.pivotIndex} />
			</Label>

			<Label label="Pinned">
				<select bind:value={value.pinned}>
					<option value={null}>None</option>
					<option value="left">Left</option>
					<option value="right">Right</option>
				</select>
			</Label>

			<Label label="Row group">
				<Toggle bind:checked={value.rowGroup} size="xs" />
			</Label>

			<Label label="Row group index">
				<input type="number" placeholder="row group index" bind:value={value.rowGroupIndex} />
			</Label>
			 -->

				<Label label="Type">
					<select bind:value={value.cellRendererType}>
						<option value="text">Text</option>
						<option value="link">Link</option>
					</select>
				</Label>

				{#if value.cellRendererType === 'link'}
					<Alert type="info" title="Label" size="xs">
						They are two ways to define a link:
						<ul class="list-disc list-inside">
							<li>
								<strong>String</strong>: The string will be used as the link and the label.
							</li>
							<li>
								<strong>Object</strong>: The object must have a <code>href</code> and a
								<code>label</code> property.
							</li>
						</ul>
					</Alert>
				{/if}
			</div>
		{/if}
	{/snippet}
</Popover>
