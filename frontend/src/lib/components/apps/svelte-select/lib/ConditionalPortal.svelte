<script lang="ts">
	import { getContext } from 'svelte'
	import Portal from '$lib/components/Portal.svelte'

	import type { AppViewerContext } from '../../types'

	export let condition = false

	const { mode } = getContext<AppViewerContext>('AppViewerContext')

	$: target = $mode === 'preview' ? '#app-editor-select' : 'body'
</script>

{#if condition}
	<Portal name="conditional-portal-select" {target}><slot /></Portal>
{:else}
	<slot />
{/if}
