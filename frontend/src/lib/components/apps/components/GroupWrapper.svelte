<script lang="ts">
	import { setContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { GroupContext } from '../types'

	export let groupContext: Writable<Record<string, any>>

	setContext<GroupContext>('GroupContext', groupContext)
</script>

<slot />
