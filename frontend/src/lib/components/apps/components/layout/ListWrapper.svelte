<script lang="ts">
	import { createEventDispatcher, setContext } from 'svelte'
	import type { ListInputs, ListContext } from '../../types'
	import { writable } from 'svelte/store'

	export let index: number
	export let value: any
	export let disabled = false

	const ctx = writable({ index, value, disabled })

	const dispatch = createEventDispatcher()

	$: $ctx = { index, value, disabled }
	setContext<ListContext>('ListWrapperContext', ctx)
	setContext<ListInputs>('ListInputs', {
		set: (id: string, value: any) => {
			dispatch('set', { id, value })
		},
		remove(id) {
			dispatch('remove', id)
		}
	})
</script>

<slot />
