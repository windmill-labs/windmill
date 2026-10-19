<script lang="ts">
	import { WandSparkles } from 'lucide-svelte'
	import Button from '../button/Button.svelte'
	import {
		homeComposerContext,
		itemContextElement
	} from '$lib/components/sessions/homeComposerContext.svelte'

	let {
		kind,
		path,
		summary,
		rawApp
	}: { kind: string; path: string; summary?: string; rawApp?: boolean } = $props()

	let element = $derived(itemContextElement({ kind, path, summary: summary || undefined, rawApp }))
</script>

{#if element && homeComposerContext.available}
	<Button
		variant="subtle"
		unifiedSize="sm"
		btnClasses="!text-ai"
		startIcon={{ icon: WandSparkles }}
		title="Add to the chat above"
		onClick={() => element && homeComposerContext.attach(element)}
	>
		Ask AI
	</Button>
{/if}
