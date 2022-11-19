<script lang="ts">
	import { getContext } from 'svelte'
	import type { StaticInput, DynamicInput, AppEditorContext } from '../types'

	export let input: DynamicInput | StaticInput
	export let value: any

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: hasConnection = input.type === 'output' && input.id && input.name

	$: inputResult = hasConnection
		? $worldStore?.connect<any>(input, () => updateValue())
		: {
				peak: () => {
					if (input.type === 'static') {
						return input.value
					}
				}
		  }

	function updateValue() {
		value = inputResult?.peak()
	}

	$: !hasConnection && input && updateValue()
</script>
