<script lang="ts">
	import Button from '../button/Button.svelte'
	import {
		homeComposerContext,
		itemContextElement
	} from '$lib/components/sessions/homeComposerContext.svelte'
	import type { BulkKind } from '$lib/components/home/homeSelection.svelte'

	let {
		kind,
		path,
		summary,
		rawApp
	}: { kind: BulkKind; path: string; summary?: string; rawApp?: boolean } = $props()

	let element = $derived(itemContextElement({ kind, path, summary: summary || undefined, rawApp }))
</script>

{#if element && homeComposerContext.available}
	<Button
		variant="subtle"
		unifiedSize="sm"
		btnClasses="!text-ai font-medium"
		title="Add to context"
		aria-label="Add to context"
		onClick={() => element && homeComposerContext.attach(element)}
	>
		+@
	</Button>
{/if}
