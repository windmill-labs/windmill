<script lang="ts">
	import { getContext } from 'svelte'
	import Portal from '$lib/components/Portal.svelte'

	import type { AppViewerContext } from '../../types'

	interface Props {
		condition?: boolean
		children?: import('svelte').Snippet
	}

	let { condition = false, children }: Props = $props()

	const { mode } = getContext<AppViewerContext>('AppViewerContext')

	let target = $derived($mode === 'preview' ? '#app-editor-select' : 'body')
</script>

{#if condition}
	<Portal name="conditional-portal-select" {target}>{@render children?.()}</Portal>
{:else}
	{@render children?.()}
{/if}
