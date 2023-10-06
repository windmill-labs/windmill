<script lang="ts">
	import { Popup } from '../common'
	import Toggle from '$lib/components/Toggle.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import Alert from '../common/alert/Alert.svelte'

	export let value: {
		width: number
		hide: boolean
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
	</div>
</Popup>
