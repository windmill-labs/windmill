<script lang="ts">
	import { Badge } from '$lib/components/common'
	import Range from '$lib/components/Range.svelte'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let value: number
	let min = 0
	let max = 42

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<number | null>
	}
	$: if (value || !value) {
		// Disallow 'e' character in numbers
		// if(value && value.toString().includes('e')) {
		// 	value = +value.toString().replaceAll('e', '')
		// }
		const num = isNaN(+value) ? null : +value
		outputs?.result.set(num)
	}
</script>

<InputValue {id} input={configuration.min} bind:value={min} />
<InputValue {id} input={configuration.max} bind:value={max} />

<AlignWrapper {verticalAlignment}>
	<div class="flex w-full gap-1 px-1">
		<span>{min}</span>
		<div class="grow ">
			<Range bind:value {min} {max} />
		</div>
		<span>{max}</span>
		<span class="mx-2"><Badge large color="blue">{value}</Badge></span>
	</div>
</AlignWrapper>
