<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext } from '../../types'
	import { accessPropertyByPath } from '../../utils'

	type T = string | number | boolean | Record<string | number, any> | undefined

	export let input: AppInput
	export let value: T

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: input && $worldStore && handleConnection()

	function handleConnection() {
		if (input.type === 'connected') {
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
		if (input.type === 'connected' && newValue) {
			const { connection } = input

			if (!connection) {
				// No connection
				return
			}

			const { componentId, path } = connection

			const hasSubPath = ['.', '['].includes(path)

			if (hasSubPath) {
				// Must remove top level property from path
				// Which was manually added, i.e. result
				const realPath = path.split('.').slice(1).join('.')

				value = accessPropertyByPath<T>(newValue, realPath)
			} else {
				value = newValue
			}
		} else {
			// TODO: handle disconnect
		}
	}
</script>
