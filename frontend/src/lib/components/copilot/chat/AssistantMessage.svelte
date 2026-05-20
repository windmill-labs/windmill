<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import type { DisplayMessage } from './shared'
	import CodeDisplay from './script/CodeDisplay.svelte'
	import LinkRenderer from './LinkRenderer.svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		extractCandidatePaths,
		remarkWindmillPaths,
		workspaceItemRegistry
	} from './workspaceItems.svelte'

	interface Props {
		message: DisplayMessage
	}

	let { message }: Props = $props()

	const candidatePaths = $derived(extractCandidatePaths(message.content))
	const rendererPlugin = {
		renderer: {
			pre: CodeDisplay,
			a: LinkRenderer
		}
	}

	// Only populate the registry for messages that contain path-shaped tokens. The
	// registry still dedups concurrent calls across messages and workspaces.
	$effect(() => {
		const ws = $workspaceStore
		if (ws && candidatePaths.length > 0) workspaceItemRegistry.ensureLoaded(ws)
	})

	const plugins = $derived.by(() => {
		const ws = $workspaceStore ?? ''
		if (!ws || candidatePaths.length === 0) {
			return [gfmPlugin(), rendererPlugin]
		}

		if (!workspaceItemRegistry.isLoaded(ws)) {
			return [gfmPlugin(), rendererPlugin]
		}

		return [
			gfmPlugin(),
			{
				remarkPlugin: remarkWindmillPaths({
					resolve: (path) => workspaceItemRegistry.resolve(ws, path),
					workspace: ws || undefined
				}),
				renderer: {}
			},
			rendererPlugin
		]
	})
</script>

<div
	class="prose prose-sm dark:prose-invert w-full max-w-full leading-snug space-y-2 prose-ul:!pl-6
		prose-p:text-xs prose-li:text-xs prose-code:text-xs prose-pre:text-xs
		prose-code:break-words prose-a:break-words
		prose-headings:font-medium prose-headings:text-emphasis prose-headings:mt-3 prose-headings:mb-1
		prose-h1:text-sm prose-h2:text-xs prose-h3:text-xs prose-h4:text-xs prose-h5:text-xs prose-h6:text-xs
		prose-table:block prose-table:max-w-full prose-table:overflow-x-auto prose-table:text-xs"
>
	<Markdown md={message.content} {plugins} />
</div>
