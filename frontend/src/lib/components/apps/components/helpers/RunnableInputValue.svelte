<script lang="ts">
	import { getContext } from 'svelte'
	import type { StaticInput, DynamicInput, AppEditorContext, UserInput } from '../../types'

	type T = $$Generic
	export let input: DynamicInput | StaticInput | UserInput
	export let value: T

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: input.type === 'output' && $worldStore?.connect<any>(input, onValueChange)
	$: input.type === 'static' && (value = input.value)

	function onValueChange(newValue: T): void {
		value = newValue
	}
</script>
