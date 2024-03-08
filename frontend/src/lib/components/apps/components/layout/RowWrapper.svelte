<script lang="ts">
	import { setContext } from 'svelte'
	import type { ListInputs, ListContext } from '../../types'
	import { writable } from 'svelte/store'

	export let index: number
	export let value: any
	export let disabled = false
	export let onSet: (id: string, value: any) => void
	export let onRemove: (id: string) => void

	const ctx = writable({ index, value, disabled })

	$: $ctx = { index, value, disabled }

	setContext<ListContext>('RowWrapperContext', ctx)
	setContext<ListInputs>('RowInputs', {
		set: (id: string, value: any) => {
			onSet(id, value)
		},
		remove(id) {
			onRemove(id)
		}
	})
</script>

<slot />
