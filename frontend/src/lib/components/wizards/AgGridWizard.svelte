<script lang="ts">
	import { Popup } from '../common'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import Alert from '../common/alert/Alert.svelte'

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
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface "
>
	<svelte:fragment slot="button">
		<slot name="trigger" />
	</svelte:fragment>
	<div class="flex flex-col w-96 p-2">
		<span class="text-sm mb-2 leading-6 font-semibold">AG Grid Settings</span>

		<span class="text-xs mb-1 pt-2 leading-6">Field</span>
		<input type="text" placeholder="field" bind:value={value.field} />

		<span class="text-xs mb-1 pt-2 leading-6">Width</span>
		<Alert type="warning" title="Not applied?" size="xs" class="mb-2">
			This property will be ignored if you set the column to flex.
		</Alert>
		<input type="number" placeholder="width" bind:value={value.width} />

		<span class="text-xs mb-1 leading-6">Hide</span>
		<Toggle
			on:pointerdown={(e) => {
				e?.stopPropagation()
			}}
			options={{ right: 'Hide' }}
			bind:checked={value.hide}
			size="xs"
		/>

		<span class="text-xs mb-1 leading-6">Value Formatter</span>
		<SimpleEditor autoHeight lang="javascript" bind:code={value.valueFormatter} />

		<span class="text-xs mb-1 leading-6">Value Parser</span>
		<SimpleEditor autoHeight lang="javascript" bind:code={value.valueParser} />

		<!-- Flex -->
		<span class="text-xs mb-1 leading-6">Flex</span>
		<input type="number" placeholder="flex" bind:value={value.flex} />

		<!-- Sort -->
		<span class="text-xs mb-1 leading-6">Sort</span>
		<select bind:value={value.sort}>
			<option value={null}>None</option>
			<option value="asc">Ascending</option>
			<option value="desc">Descending</option>
		</select>

		<!-- Sort Index -->
		<span class="text-xs mb-1 leading-6">Sort Index</span>
		<input type="number" placeholder="sort index" bind:value={value.sortIndex} />

		<!-- Aggregation Function -->
		<span class="text-xs mb-1 leading-6">Aggregation Function</span>
		<SimpleEditor autoHeight lang="javascript" bind:code={value.aggFunc} />

		<!-- Pivot -->
		<span class="text-xs mb-1 leading-6">Pivot</span>
		<Toggle bind:checked={value.pivot} size="xs" />

		<!-- Pivot Index -->
		<span class="text-xs mb-1 leading-6">Pivot Index</span>
		<input type="number" placeholder="pivot index" bind:value={value.pivotIndex} />

		<!-- Pinned -->
		<span class="text-xs mb-1 leading-6">Pinned</span>
		<select bind:value={value.pinned}>
			<option value={null}>None</option>
			<option value="left">Left</option>
			<option value="right">Right</option>
		</select>

		<!-- Row Group -->
		<span class="text-xs mb-1 leading-6">Row Group</span>
		<Toggle bind:checked={value.rowGroup} size="xs" />

		<!-- Row Group Index -->
		<span class="text-xs mb-1 leading-6">Row Group Index</span>
		<input type="number" placeholder="row group index" bind:value={value.rowGroupIndex} />
	</div>
</Popup>
