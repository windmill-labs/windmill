<script lang="ts">
	import { setContext } from 'svelte'
	import type { ListContext } from '../../types'
	import { writable } from 'svelte/store'

	export let index: number
	export let value: any
	export let disabled = false

	const ctx = writable({ index, value, disabled })

	$: $ctx = { index, value, disabled }
	setContext<ListContext>('ListWrapperContext', ctx)
</script>

<slot />
