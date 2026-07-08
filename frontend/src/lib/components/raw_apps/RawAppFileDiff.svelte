<!--
@component
Diff body for a single raw-app file item (one `RawAppFileItem`). Renders one
content-sized Monaco diff, with a per-file size guard: a file whose line count
exceeds `lineBudget` shows a "Load diff" card instead of auto-mounting.

The synthesized `app.yaml` metadata item (isMetadata) additionally offers an
"Expand to full app YAML" toggle that swaps in the whole serialized app diff —
the escape hatch, shown only on demand.
-->
<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { Button } from '$lib/components/common'

	interface Props {
		original?: string
		current?: string
		lang: string
		inlineDiff?: boolean
		/** Forward Monaco's auto-inline opt-out to the underlying DiffEditor. */
		disableAutoInline?: boolean
		lineBudget?: number
		isMetadata?: boolean
		fullYamlOriginal?: string
		fullYamlCurrent?: string
	}

	let {
		original,
		current,
		lang,
		inlineDiff = false,
		disableAutoInline = false,
		lineBudget = 1500,
		isMetadata = false,
		fullYamlOriginal,
		fullYamlCurrent
	}: Props = $props()

	let showFullYaml = $state(false)
	let forceLoad = $state(false)

	const canExpandYaml = $derived(
		isMetadata && (fullYamlOriginal !== undefined || fullYamlCurrent !== undefined)
	)

	// Active diff sides depend on the metadata expand toggle.
	const activeOriginal = $derived(showFullYaml ? (fullYamlOriginal ?? '') : (original ?? ''))
	const activeModified = $derived(showFullYaml ? (fullYamlCurrent ?? '') : (current ?? ''))
	const activeLang = $derived(showFullYaml ? 'yaml' : lang)

	const LINE_HEIGHT = 19
	const EDITOR_CHROME = 24
	function linesIn(s: string): number {
		return Math.max(s.split('\n').length, 1)
	}
	// The larger side's line count: it's what drives Monaco's mount cost (and the
	// editor height), so it's the right metric for the size guard.
	const lineCount = $derived(Math.max(linesIn(activeOriginal), linesIn(activeModified)))
	const height = $derived(`${lineCount * LINE_HEIGHT + EDITOR_CHROME}px`)
	// The full-YAML view is opt-in, so never guard it; only guard the default
	// per-file view.
	const guarded = $derived(!showFullYaml && !forceLoad && lineCount > lineBudget)
</script>

<div class="flex flex-col">
	{#if canExpandYaml}
		<div class="flex items-center justify-end px-3 py-1 border-b border-light">
			<Button variant="subtle" unifiedSize="sm" onclick={() => (showFullYaml = !showFullYaml)}>
				{showFullYaml ? 'Show metadata only' : 'Expand to full app YAML'}
			</Button>
		</div>
	{/if}
	{#if guarded}
		<div class="flex items-center gap-2 px-3 py-3">
			<span class="text-2xs text-secondary">
				Large file — {lineCount.toLocaleString()} lines.
			</span>
			<Button variant="subtle" unifiedSize="sm" onclick={() => (forceLoad = true)}>Load diff</Button
			>
		</div>
	{:else}
		{#await import('$lib/components/DiffEditor.svelte')}
			<div class="p-3"><Loader2 class="w-3.5 h-3.5 animate-spin" /></div>
		{:then Module}
			<div style="height: {height}">
				<Module.default
					open={true}
					automaticLayout
					className="h-full"
					defaultLang={activeLang}
					defaultOriginal={activeOriginal}
					defaultModified={activeModified}
					{inlineDiff}
					{disableAutoInline}
					readOnly
				/>
			</div>
		{/await}
	{/if}
</div>
