<script lang="ts">
	import { setContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { GroupContext } from '../types'

	interface Props {
		context: Writable<Record<string, any>>
		id: string
		children?: import('svelte').Snippet
	}

	let { context, id, children }: Props = $props()

	setContext<GroupContext>('GroupContext', { id, context })
</script>

{@render children?.()}
