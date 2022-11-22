<script lang="ts">
	import { getContext } from 'svelte'
	import type { StaticInput, DynamicInput, AppEditorContext, UserInput } from '../../types'

	export let input: DynamicInput | StaticInput | UserInput
	export let value: any

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: hasConnection = input.type === 'output' && input.id !== undefined && input.name !== undefined

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

	$: !hasConnection && input && input.type === 'static' && updateValue()
</script>
