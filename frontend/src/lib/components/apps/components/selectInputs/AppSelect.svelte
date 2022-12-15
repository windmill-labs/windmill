<script lang="ts">
	import { getContext } from 'svelte'
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

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let label: string
	let items: string[]
	let itemKey: string
	
	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string | undefined>
	}

	function onChange({detail}: CustomEvent) {
		console.log(detail);
		outputs?.result.set(detail?.[itemKey] || undefined)
	}
</script>

<InputValue input={configuration.label} bind:value={label} />
<InputValue input={configuration.items} bind:value={items} />
<InputValue input={configuration.itemKey} bind:value={itemKey} />

<AlignWrapper {horizontalAlignment} {verticalAlignment}>
	<!-- svelte-ignore a11y-label-has-associated-control -->
	<label class="block w-full">
		<div>
			{label}
		</div>
		<div>
			<Select
				on:clear={onChange}
				on:change={onChange}
			
				{items}
				class="w-full"
				placeholder="Select an item"
			/>
		</div>
	</label>
</AlignWrapper>
