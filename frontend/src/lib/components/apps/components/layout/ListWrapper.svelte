<script lang="ts">
	import { setContext } from 'svelte'
	import type { ListContext } from '../../types'

	export let index: number
	export let value: any

	setContext<ListContext>('ListWrapperContext', { index, value })
</script>

<slot />
