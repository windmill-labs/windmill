<script lang="ts">
	import { setContext } from 'svelte'
	import type { ListContext } from '../../types'
	import { writable } from 'svelte/store'

	export let index: number
	export let value: any

	const ctx = writable({ index, value })

	$: $ctx = { index, value }
	setContext<ListContext>('ListWrapperContext', ctx)
</script>

<slot />
