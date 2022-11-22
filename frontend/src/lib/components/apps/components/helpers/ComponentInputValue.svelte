<script lang="ts">
	import { getContext } from 'svelte'
	import type { StaticInput, DynamicInput, AppEditorContext } from '../../types'

	type T = $$Generic

	export let input: DynamicInput | StaticInput
	export let value: T

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: input.type === 'static' && (value = input.value)
	$: input.type === 'output' && $worldStore?.connect<any>(input, onValueChange)

	function onValueChange(newValue: T): void {
		value = newValue
	}
</script>
