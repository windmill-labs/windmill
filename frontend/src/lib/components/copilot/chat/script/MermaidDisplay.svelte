<script lang="ts">
	import { randomUUID } from '$lib/utils/uuid'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'

	let { code }: { code: string } = $props()

	const isDarkMode = useIsDarkMode()

	let svg = $state<string | undefined>(undefined)
	// The exact source that produced `svg`. The diagram is only shown while this
	// still matches the current `code`, so a later edit that fails to parse falls
	// back to the raw source instead of leaving a stale, mismatched diagram.
	let renderedCode = $state<string | undefined>(undefined)
	// Monotonic token so an earlier-started render that resolves late can't
	// overwrite the result of a newer one (out-of-order async on rapid code/theme changes).
	let renderSeq = 0

	async function render(source: string, dark: boolean) {
		const seq = ++renderSeq
		if (!source?.trim()) {
			svg = undefined
			renderedCode = undefined
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
			const result = await mermaid.render(`mermaid-${randomUUID()}`, source)
			if (seq !== renderSeq) return
			svg = result.svg
			renderedCode = source
		} catch {
			// Parse failure (often a partial block still streaming in): fall back to the
			// raw source. `showSvg` already hides any previous diagram since `renderedCode`
			// no longer matches the current `code`.
		}
	}

	$effect(() => {
		void render(code, isDarkMode.val)
	})

	// Only show the diagram while it corresponds to the current source.
	let showSvg = $derived(svg !== undefined && renderedCode === code)
</script>

{#if showSvg}
	<div class="p-2 flex justify-center overflow-x-auto">
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html svg}
	</div>
{:else}
	<!-- Fallback while loading or when rendering fails: show the raw source -->
	<pre class="overflow-auto max-h-screen text-xs p-2">{code}</pre>
{/if}
