<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppInputTransform } from '../../types'
	import { accessPropertyByPath } from '../../utils'

	type T = string | number | boolean | Record<string | number, any> | undefined

	export let input: AppInputTransform
	export let value: T

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: $worldStore && handleConnection()

	function handleConnection() {
		if (input.type === 'output') {
			$worldStore?.connect<any>(input, onValueChange)
		} else if (input.type === 'static') {
			setValue()
		} else {
			value = undefined
		}
	}

	function setValue() {
		if (input.type === 'static') {
			value = input.value
		}
	}

	function onValueChange(newValue: any): void {
		if (input.type === 'output') {
			if (input.name?.includes('.')) {
				const path = input.name.split('.').slice(1).join('.')

				value = accessPropertyByPath<T>(newValue, path)
			} else {
				value = newValue
			}
		} else {
			// TODO: handle disconnect
		}
	}
</script>
