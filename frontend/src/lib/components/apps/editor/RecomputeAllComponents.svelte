<script lang="ts">
	import Dropdown from '$lib/components/Dropdown.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
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
	let timeout: NodeJS.Timeout | undefined = undefined
</script>

<div class="flex gap-4 items-center">
	<button
		on:click|preventDefault|stopPropagation={onRefresh}
		class="center-center p-1 rounded border bg-white/60 hover:bg-gray-200 inline-flex gap-2"
	>
		<RefreshCw class={loading ? 'animate-spin' : ''} size={20} /> ({Object.keys($runnableComponents)
			.length})
	</button>
	<Toggle
		size="xs"
		on:change={(e) => {
			if (e.detail) {
				timeout = setInterval(onRefresh, 15000)
			} else {
				timeout && clearInterval(timeout)
			}
		}}
		options={{ right: 'auto (15s)' }}
	/>
</div>
