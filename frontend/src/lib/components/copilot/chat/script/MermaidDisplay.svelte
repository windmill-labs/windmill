<script lang="ts">
	import { randomUUID } from '$lib/utils/uuid'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'

	let { code }: { code: string } = $props()

	const id = `mermaid-${randomUUID()}`
	const isDarkMode = useIsDarkMode()

	let svg = $state<string | undefined>(undefined)

	async function render(source: string, dark: boolean) {
		if (!source?.trim()) {
			svg = undefined
			return
		}
		try {
			const mermaid = (await import('mermaid')).default
			mermaid.initialize({
				startOnLoad: false,
				theme: dark ? 'dark' : 'default',
				securityLevel: 'strict',
				// Throw on parse errors instead of injecting an orphan error diagram into the DOM.
				suppressErrorRendering: true
			})
			// mermaid.render needs a fresh element id per attempt to avoid id collisions.
			const result = await mermaid.render(`${id}-${randomUUID()}`, source)
			svg = result.svg
		} catch (e) {
			console.error('Failed to render mermaid diagram', e)
			svg = undefined
		}
	}

	$effect(() => {
		void render(code, isDarkMode.val)
	})
</script>

{#if svg}
	<div class="p-2 flex justify-center overflow-x-auto">
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html svg}
	</div>
{:else}
	<!-- Fallback while loading or when rendering fails: show the raw source -->
	<pre class="overflow-auto max-h-screen text-xs p-2">{code}</pre>
{/if}
