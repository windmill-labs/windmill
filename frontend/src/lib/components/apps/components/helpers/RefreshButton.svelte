<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faRefresh } from '@fortawesome/free-solid-svg-icons'
	import { RefreshCcw, RefreshCw } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'

	export let componentId: string

	const { runnableComponents, worldStore } = getContext<AppEditorContext>('AppEditorContext')

	async function refresh() {
		window.dispatchEvent(new Event('pointerup'))

		await $runnableComponents[componentId]?.()
	}
	let loading = false
	$: $worldStore?.outputsById[componentId]?.['loading']?.subscribe({
		next: (value) => {
			loading = value
		}
	})
</script>

<button
	on:pointerdown|preventDefault|stopPropagation
	on:click|preventDefault|stopPropagation={refresh}
	class="center-center p-1 rounded border bg-white/60 z-40"
>
	<RefreshCw class={loading ? 'animate-spin' : ''} size={16} />
</button>
