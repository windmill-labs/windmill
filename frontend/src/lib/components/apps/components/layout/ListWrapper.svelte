<script lang="ts">
	import { setContext } from 'svelte'
	import type { ListInputs, ListContext } from '../../types'
	import { writable } from 'svelte/store'

	export let index: number
	export let value: any
	export let disabled = false
	export let inputs: Record<string, Record<number, any>> = {}
	export let onInputsChange: () => void

	const ctx = writable({ index, value, disabled })

	$: $ctx = { index, value, disabled }
	setContext<ListContext>('ListWrapperContext', ctx)
	setContext<ListInputs>('ListInputs', {
		set: (id: string, value: any) => {
			if (!inputs[id]) {
				inputs[id] = { [index]: value }
			} else {
				inputs[id][index] = value
			}
			onInputsChange()
		},
		remove(id) {
			if (inputs?.[id] == undefined) {
				return
			}
			if (index == 0) {
				delete inputs[id]
			} else {
				delete inputs[id][index]
			}
			inputs = inputs
		}
	})
</script>

<slot />
