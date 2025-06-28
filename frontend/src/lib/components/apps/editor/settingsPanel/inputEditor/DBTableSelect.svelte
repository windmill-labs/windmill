<script lang="ts">
	import type { StaticInput, StaticOptions } from '../../../inputType'
	import Select from '$lib/components/select/Select.svelte'

	interface Props {
		componentInput: StaticInput<any> | undefined
		selectOptions?: StaticOptions['selectOptions'] | undefined
		id: string | undefined
	}
	let { componentInput = $bindable(), selectOptions: _options = undefined, id }: Props = $props()
	let options = $derived.by(() => {
		if (!Array.isArray(_options)) return []
		return _options
			.map((o) => (typeof o === 'string' ? { label: o, value: o } : o))
			.sort((a, b) => a.label.localeCompare(b.label))
	})
</script>

{#if options && componentInput}
	<Select
		bind:value={componentInput.value}
		items={options}
		class="text-clip grow min-w-0"
		placeholder="Select a table"
		clearable
	/>
{:else}
	<input {id} class="w-full p-2 border rounded-md" type="text" />
{/if}
