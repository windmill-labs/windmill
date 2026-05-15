<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import type { DisplayMessage } from './shared'
	import CodeDisplay from './script/CodeDisplay.svelte'
	import LinkRenderer from './LinkRenderer.svelte'
	import { workspaceStore } from '$lib/stores'
	import { remarkWindmillPaths, workspaceItemRegistry } from './workspaceItems.svelte'

	interface Props {
		message: DisplayMessage
	}

	let { message }: Props = $props()

	// Trigger the lazy load once we mount in a workspace context. The registry dedups
	// concurrent calls across messages, so this is cheap to call repeatedly.
	$effect(() => {
		const ws = $workspaceStore
		if (ws) workspaceItemRegistry.ensureLoaded(ws)
	})

	const plugins = $derived.by(() => {
		const ws = $workspaceStore ?? ''
		// Subscribe to the registry's reactive state so this derivation re-runs once the
		// workspace items finish loading — otherwise messages that mounted before the load
		// completed would stay as plain text forever (the underlying `parse` in
		// svelte-exmarkdown only re-runs when the `plugins` reference changes).
		workspaceItemRegistry.isLoaded(ws)
		return [
			gfmPlugin(),
			{
				remarkPlugin: remarkWindmillPaths({
					resolve: (path) => workspaceItemRegistry.resolve(ws, path),
					workspace: ws || undefined
				}),
				renderer: {}
			},
			{
				renderer: {
					pre: CodeDisplay,
					a: LinkRenderer
				}
			}
		]
	})
</script>

<div
	class="prose prose-sm dark:prose-invert w-full max-w-full leading-snug space-y-2 prose-ul:!pl-6"
>
	<Markdown md={message.content} {plugins} />
</div>
