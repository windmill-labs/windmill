<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faRefresh } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'

	export let componentId: string

	const { runnableComponents } = getContext<AppEditorContext>('AppEditorContext')

	let loading = false
	async function refresh() {
		loading = true
		await $runnableComponents[componentId]?.()
		loading = false
	}
</script>

<Button
	iconOnly
	startIcon={{ icon: faRefresh, classes: loading ? 'animate-spin' : '' }}
	color="dark"
	size="xs"
	on:click={refresh}
/>
