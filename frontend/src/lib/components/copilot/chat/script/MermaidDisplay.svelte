<script lang="ts">
	import { randomUUID } from '$lib/utils/uuid'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'

	let { code }: { code: string } = $props()

	const id = `mermaid-${randomUUID()}`
	const isDarkMode = useIsDarkMode()

	let svg = $state<string | undefined>(undefined)
	// Monotonic token so an earlier-started render that resolves late can't
	// overwrite the result of a newer one (out-of-order async on rapid code/theme changes).
	let renderSeq = 0

	async function render(source: string, dark: boolean) {
		const seq = ++renderSeq
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
			if (seq !== renderSeq) return
			svg = result.svg
		} catch (e) {
			if (seq !== renderSeq) return
			// Keep the last good diagram so transient parse failures while the block
			// is still streaming don't collapse it; only fall back to raw source if
			// nothing has rendered yet.
			if (svg === undefined) {
				console.error('Failed to render mermaid diagram', e)
			}
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
