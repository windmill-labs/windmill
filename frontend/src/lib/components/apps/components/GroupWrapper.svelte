<script lang="ts">
	import { setContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { GroupContext } from '../types'

	export let context: Writable<Record<string, any>>
	export let id: string

	setContext<GroupContext>('GroupContext', { id, context })
</script>

<slot />
