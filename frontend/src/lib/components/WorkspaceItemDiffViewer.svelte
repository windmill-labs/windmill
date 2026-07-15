<!--
@component
Inline diff renderer for a single workspace item. Mirrors the per-kind
rendering that DiffDrawer does in its body (`DiffDrawer.svelte:181-271`):

- `flow` → `<FlowDiffViewer>` (its own Graph / YAML toggle inside)
- `raw_app_file` → `<RawAppFileDiff>` (one synthesized raw-app file item: a
  single diff with a per-file size guard; the metadata item adds a full-app
  YAML expand). Raw apps are exploded into these items by `rawAppDiffToItems`.
- has `content` (scripts) → Tabs(Content | Metadata) with two Monaco diffs
- everything else (apps, resources, variables, schedules, triggers…) →
  a single Monaco YAML diff over the metadata

`inlineDiff` flips Monaco's `renderSideBySide` to false (unified view).
The component is content-sized — each Monaco block is sized to fit its
diff text (no internal scroll) using `lines * 19 + 24`; for the
Content+Metadata case we use the max of the two so switching tabs
doesn't reflow the parent.
-->
<script lang="ts">
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import FlowDiffViewer from './FlowDiffViewer.svelte'
	import RawAppFileDiff from './raw_apps/RawAppFileDiff.svelte'
	import type { RawAppFileItem } from './raw_apps/rawAppDiffUtils'
	import { Loader2 } from 'lucide-svelte'
	import { cleanValueProperties, orderedYamlStringify, replaceFalseWithUndefined } from '$lib/utils'
	import { scriptLangToEditorLang } from '$lib/scripts'

	interface Props {
		/** Any WorkspaceItemDiff['kind'], plus the synthetic `raw_app_file`.
		 * `flow` and `raw_app_file` are special-cased. */
		kind: string
		/** Raw value from `getItemValue(kind, path, parentWorkspace)`. Undefined
		 * for "added" items (don't exist in the parent). */
		originalRaw?: unknown
		/** Raw value from `getItemValue(kind, path, forkWorkspace)`. Undefined
		 * for "removed" items (don't exist in the fork). */
		currentRaw?: unknown
		/** Force unified diff (Monaco renderSideBySide=false). Default false. */
		inlineDiff?: boolean
		/** Opt out of Monaco's auto-inline fallback so the parent's inlineDiff/width
		 * decision fully controls the view (used by the drawer's toggle). Default false. */
		disableAutoInline?: boolean
		/** For `raw_app_file`: the synthesized per-file diff item to render. */
		rawFile?: RawAppFileItem
		/** Cap (px) on the rendered diff height; the drawer owns the rationale.
		 * Ignored while ≤ 0 (unmeasured). */
		maxHeight?: number
	}

	let {
		kind,
		originalRaw,
		currentRaw,
		inlineDiff = false,
		disableAutoInline = false,
		rawFile,
		maxHeight
	}: Props = $props()

	// Applied alongside each block's content-sized `height` so the block never
	// exceeds the drawer's visible height — Monaco then scrolls internally.
	function capStyle(reserved: number): string {
		if (!maxHeight || maxHeight <= 0) return ''
		return `max-height: ${Math.max(0, maxHeight - reserved)}px;`
	}
	const maxHeightStyle = $derived(capStyle(0))
	// FlowDiffViewer enforces its own `min-h-[500px]`; capping the flow below that
	// makes its pane spill out of the card instead of scrolling, so never cap the
	// flow diff below that minimum (a very short drawer just scrolls to it).
	const FLOW_MIN_HEIGHT = 500
	const flowMaxHeightStyle = $derived(
		maxHeight && maxHeight > 0 ? `max-height: ${Math.max(FLOW_MIN_HEIGHT, maxHeight)}px;` : ''
	)
	// The Content|Metadata tab bar sits above the script editor and eats into the
	// viewer's height; measure it so the editor's cap leaves room for it and the
	// whole viewer still fits `maxHeight`.
	let tabsChromeH = $state(0)
	const contentMaxHeightStyle = $derived(capStyle(tabsChromeH))

	type Prepared = { lang?: string; content?: string; metadata: string }

	function prepareValue(raw: unknown): Prepared {
		if (!raw || typeof raw !== 'object') {
			return { metadata: raw == null ? '' : String(raw) }
		}
		const cleaned = structuredClone(
			cleanValueProperties(replaceFalseWithUndefined(raw as Record<string, unknown>))
		)
		const content = (cleaned as Record<string, unknown>)['content']
		if (content !== undefined) {
			delete (cleaned as Record<string, unknown>)['content']
		}
		const language = (raw as Record<string, unknown>).language
		return {
			lang:
				typeof language === 'string'
					? scriptLangToEditorLang(language as Parameters<typeof scriptLangToEditorLang>[0])
					: undefined,
			content: typeof content === 'string' ? content : undefined,
			metadata: orderedYamlStringify(cleaned)
		}
	}

	const original = $derived(prepareValue(originalRaw))
	const current = $derived(prepareValue(currentRaw))
	const hasContent = $derived(original.content !== undefined || current.content !== undefined)

	// For added / removed flows, the missing side feeds an empty YAML so
	// the YAML-mode editor shows the whole new (or removed) flow as a
	// single-sided diff. FlowGraphDiffViewer uses the *Missing flag to
	// swap in its own OpenFlow stub for parsing and to draw a placeholder
	// pane in side-by-side mode.
	const beforeFlowYaml = $derived(originalRaw == null ? '' : original.metadata)
	const afterFlowYaml = $derived(currentRaw == null ? '' : current.metadata)

	let contentTab: 'content' | 'metadata' = $state('content')

	// Per-tab height: each Monaco block sizes to its own content. Switching
	// tabs reflows the row, which is the expected tab behavior; we don't
	// over-allocate to the larger tab the way the previous max() did.
	const LINE_HEIGHT = 19
	const EDITOR_CHROME = 24
	function linesIn(s?: string): number {
		return Math.max((s ?? '').split('\n').length, 1)
	}
	const contentHeight = $derived(
		`${Math.max(linesIn(original.content), linesIn(current.content)) * LINE_HEIGHT + EDITOR_CHROME}px`
	)
	const metadataHeight = $derived(
		`${Math.max(linesIn(original.metadata), linesIn(current.metadata)) * LINE_HEIGHT + EDITOR_CHROME}px`
	)
	const activeTabHeight = $derived(contentTab === 'content' ? contentHeight : metadataHeight)
</script>

{#if kind === 'flow'}
	<div class="h-[600px]" style={flowMaxHeightStyle}>
		<FlowDiffViewer
			beforeYaml={beforeFlowYaml}
			afterYaml={afterFlowYaml}
			beforeMissing={originalRaw == null}
			afterMissing={currentRaw == null}
			{inlineDiff}
			{disableAutoInline}
		/>
	</div>
{:else if kind === 'raw_app_file' && rawFile}
	<RawAppFileDiff
		original={rawFile.original}
		current={rawFile.current}
		lang={rawFile.lang}
		isMetadata={rawFile.isMetadata}
		fullYamlOriginal={rawFile.fullYamlOriginal}
		fullYamlCurrent={rawFile.fullYamlCurrent}
		{inlineDiff}
		{disableAutoInline}
		{maxHeight}
	/>
{:else if hasContent}
	<div class="flex flex-col">
		<div bind:clientHeight={tabsChromeH}>
			<Tabs bind:selected={contentTab}>
				<Tab value="content" label="Content" />
				<Tab value="metadata" label="Metadata" />
			</Tabs>
		</div>
		<div style="height: {activeTabHeight}; {contentMaxHeightStyle}">
			{#if contentTab === 'content'}
				{#await import('$lib/components/DiffEditor.svelte')}
					<div class="p-3"><Loader2 class="w-3.5 h-3.5 animate-spin" /></div>
				{:then Module}
					<Module.default
						open={true}
						automaticLayout
						className="h-full"
						defaultLang={original.lang ?? current.lang}
						defaultOriginal={original.content ?? ''}
						defaultModified={current.content ?? ''}
						{inlineDiff}
						{disableAutoInline}
						readOnly
					/>
				{/await}
			{:else}
				{#await import('$lib/components/DiffEditor.svelte')}
					<div class="p-3"><Loader2 class="w-3.5 h-3.5 animate-spin" /></div>
				{:then Module}
					<Module.default
						open={true}
						automaticLayout
						className="h-full"
						defaultLang="yaml"
						defaultOriginal={original.metadata}
						defaultModified={current.metadata}
						{inlineDiff}
						{disableAutoInline}
						readOnly
					/>
				{/await}
			{/if}
		</div>
	</div>
{:else}
	{#await import('$lib/components/DiffEditor.svelte')}
		<div class="p-3"><Loader2 class="w-3.5 h-3.5 animate-spin" /></div>
	{:then Module}
		<div style="height: {metadataHeight}; {maxHeightStyle}">
			<Module.default
				open={true}
				automaticLayout
				className="h-full"
				defaultLang="yaml"
				defaultOriginal={original.metadata}
				defaultModified={current.metadata}
				{inlineDiff}
				{disableAutoInline}
				readOnly
			/>
		</div>
	{/await}
{/if}
