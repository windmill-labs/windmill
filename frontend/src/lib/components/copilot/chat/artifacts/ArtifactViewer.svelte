<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { Code, Eye, FileText, Copy, Check, Download } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { copyToClipboard, download } from '$lib/utils'
	import CodeDisplay from '../script/CodeDisplay.svelte'
	import LinkRenderer from '../LinkRenderer.svelte'
	import { artifactFilename, artifactMimeType, type PersistedArtifact } from './artifactsDB'
	import { markdownProse } from '$lib/components/markdownProse'

	interface Props {
		artifact: PersistedArtifact
	}

	let { artifact }: Props = $props()

	// Markdown is the only rendered kind in v1; anything else shows source only.
	const canPreview = $derived(artifact.kind === 'md')
	let showSource = $state(false)
	const source = $derived(!canPreview || showSource)

	let copied = $state(false)
	async function copyRaw() {
		if (!(await copyToClipboard(artifact.content))) return
		copied = true
		setTimeout(() => (copied = false), 1500)
	}
	function downloadFile() {
		download(artifactFilename(artifact), artifact.content, artifactMimeType(artifact.kind))
	}

	const plugins = [gfmPlugin(), { renderer: { pre: CodeDisplay, a: LinkRenderer } }]
</script>

<div class="flex flex-col h-full bg-surface-tertiary">
	<div class="flex items-center justify-between gap-2 px-8 py-2">
		<div class="flex items-center gap-1.5 min-w-0 flex-1">
			<FileText size={14} class="shrink-0 text-secondary" />
			<span class="truncate text-xs font-normal text-emphasis" title={artifact.name}>
				{artifact.name}
			</span>
		</div>
		<div class="flex items-center gap-2 shrink-0">
			<!-- Copy raw markdown, with a dropdown for the download-as-file variant. -->
			<Button
				size="xs"
				variant="default"
				startIcon={{ icon: copied ? Check : Copy }}
				onClick={copyRaw}
				title="Copy raw markdown"
				dropdownItems={[{ label: 'Download as .md', icon: Download, onClick: downloadFile }]}
			>
				{copied ? 'Copied' : 'Copy'}
			</Button>
			{#if canPreview}
				<ToggleButtonGroup
					noWFull
					selected={showSource ? 'source' : 'preview'}
					onSelected={(v) => (showSource = v === 'source')}
				>
					{#snippet children({ item })}
						<ToggleButton {item} value="preview" icon={Eye} iconOnly tooltip="Preview" size="sm" />
						<ToggleButton
							{item}
							value="source"
							icon={Code}
							iconOnly
							tooltip="View source"
							size="sm"
						/>
					{/snippet}
				</ToggleButtonGroup>
			{/if}
		</div>
	</div>

	<div class="flex-1 min-h-0 overflow-auto">
		{#if source}
			<!-- key: SimpleEditor reads `code` only on init; remount on id or content change. -->
			{#key `${artifact.id}:${artifact.updatedAt}`}
				<SimpleEditor lang="markdown" code={artifact.content} readOnly class="h-full" />
			{/key}
		{:else}
			<!-- Pinned under the header, fades scrolled-under content instead of hard-clipping it.
			     The negative margin cancels its flow height so it overlays instead of pushing. -->
			<div class="sticky top-0 z-10 h-4 -mb-4 bg-gradient-to-b from-surface-tertiary to-transparent"
			></div>
			<div class="px-8 pb-4 pt-2 {markdownProse.doc}">
				<Markdown md={artifact.content} {plugins} />
			</div>
		{/if}
	</div>
</div>
