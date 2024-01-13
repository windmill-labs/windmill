<script lang="ts">
	import { setContext } from 'svelte'
	import type { ListInputs, ListContext } from '../../types'
	import { writable } from 'svelte/store'

	export let index: number
	export let value: any
	export let disabled = false
	export let onSet: ((id: string, value: any) => void) | undefined = undefined
	export let onRemove: ((id: string) => void) | undefined = undefined
	const ctx = writable({ index, value, disabled })

	$: $ctx = { index, value, disabled }
	setContext<ListContext>('ListWrapperContext', ctx)
	setContext<ListInputs>('ListInputs', {
		set: (id: string, value: any) => {
			onSet?.(id, value)
		},
		remove(id) {
			onRemove?.(id)
		}
	})
</script>

<slot />
