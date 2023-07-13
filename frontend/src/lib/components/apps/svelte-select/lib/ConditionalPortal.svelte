<script lang="ts">
	import { getContext } from 'svelte'
	import Portal from 'svelte-portal'
	import type { AppViewerContext } from '../../types'

	export let condition = false

	const { mode } = getContext<AppViewerContext>('AppViewerContext')

	$: target = $mode === 'preview' ? '#app-editor-select' : 'body'
</script>

{#if condition}
	<Portal {target}><slot /></Portal>
{:else}
	<slot />
{/if}
