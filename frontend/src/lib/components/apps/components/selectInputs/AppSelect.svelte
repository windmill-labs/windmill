<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import Select from 'svelte-select'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export const staticOutputs: string[] = ['loading', 'result']
	export let id: string
	export let configuration: Record<string, AppInput>
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	const { worldStore, connectingInput, selectedComponent } =
		getContext<AppEditorContext>('AppEditorContext')
	let label: string
	let items: string[]
	let itemKey: string

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string | undefined>
	}

	function onChange({ detail }: CustomEvent) {
		outputs?.result.set(detail?.[itemKey] || undefined)
	}

	const dispatch = createEventDispatcher()
</script>

<InputValue {id} input={configuration.label} bind:value={label} />
<InputValue {id} input={configuration.items} bind:value={items} />
<InputValue {id} input={configuration.itemKey} bind:value={itemKey} />

<AlignWrapper {horizontalAlignment} {verticalAlignment}>
	<Select
		--height="34px"
		class="select"
		on:clear={onChange}
		on:change={onChange}
		{items}
		placeholder="Select an item"
		on:click={() => {
			if (!$connectingInput.opened) {
				$selectedComponent = id
			}
		}}
	/>
</AlignWrapper>
