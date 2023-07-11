<script lang="ts">
	import { createEventDispatcher, setContext } from 'svelte'
	import type { ListInputs, ListContext } from '../../types'
	import { writable } from 'svelte/store'

	export let index: number
	export let value: any
	export let disabled = false
	export let inputs: Record<string, Record<number, any>> = {}

	const dispatch = createEventDispatcher()
	const ctx = writable({ index, value, disabled })

	$: $ctx = { index, value, disabled }
	setContext<ListContext>('ListWrapperContext', ctx)
	setContext<ListInputs>('ListInputs', (id: string, value: any) => {
		if (!inputs[id]) {
			inputs[id] = { [index]: value }
		} else {
			inputs[id][index] = value
		}
		console.log('foo')
		dispatch('inputsChange')
	})
</script>

<slot />
