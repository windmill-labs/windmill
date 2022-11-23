<script lang="ts">
	import { getContext } from 'svelte'
	import type { StaticInput, DynamicInput, AppEditorContext, UserInput } from '../../types'

	type T = $$Generic
	export let input: DynamicInput | StaticInput | UserInput
	export let value: T

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: input && handleConnection()

	function handleConnection() {
		if (input.type === 'output') {
			$worldStore?.connect<any>(input, onValueChange)
		} else if (input.type === 'static') {
			setValue()
		}
	}

	function setValue() {
		if (input.type === 'static') {
			value = input.value
		}
	}

	function onValueChange(newValue: T): void {
		if (input.type === 'output') {
			value = newValue
		} else {
			// TODO: handle disconnect
		}
	}
</script>
