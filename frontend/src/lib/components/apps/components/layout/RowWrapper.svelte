<script lang="ts">
	import { setContext } from 'svelte'
	import type { ListInputs, ListContext } from '../../types'
	import { writable } from 'svelte/store'

	interface Props {
		index: number
		value: any
		disabled?: boolean
		onSet: (id: string, value: any) => void
		onRemove: (id: string) => void
		children?: import('svelte').Snippet
	}

	let { index, value, disabled = false, onSet, onRemove, children }: Props = $props()

	const ctx = writable({ index, value, disabled })

	$effect(() => {
		$ctx = { index, value, disabled }
	})

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

{@render children?.()}
