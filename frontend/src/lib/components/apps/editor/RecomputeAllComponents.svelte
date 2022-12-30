<script lang="ts">
	import { RefreshCw } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../types'

	const { runnableComponents } = getContext<AppEditorContext>('AppEditorContext')

	let loading: boolean = false

	async function onRefresh() {
		loading = true
		await Promise.all(
			Object.keys($runnableComponents).map((id) => {
				return $runnableComponents?.[id]?.()
			})
		)
		loading = false
	}
</script>

<button
	on:click|preventDefault|stopPropagation={onRefresh}
	class="center-center p-1 rounded border bg-white/60  inline-flex gap-2"
>
	<RefreshCw class={loading ? 'animate-spin' : ''} size={20} /> ({Object.keys($runnableComponents)
		.length})
</button>
