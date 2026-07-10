<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { Code, Eye, FileText, Copy, Check, ChevronDown, Download } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { copyToClipboard, download } from '$lib/utils'
	import CodeDisplay from '../script/CodeDisplay.svelte'
	import LinkRenderer from '../LinkRenderer.svelte'
	import { artifactFilename, artifactMimeType, type PersistedArtifact } from './artifactsDB'

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

<div class="flex flex-col h-full">
	<div class="flex items-center justify-between gap-2 p-2 border-b border-border-light">
		<div class="flex items-center gap-1.5 min-w-0 flex-1">
			<FileText size={14} class="shrink-0 text-tertiary" />
			<span class="truncate text-xs font-bold" title={artifact.name}>
				{artifact.name}
			</span>
		</div>
		<div class="flex items-center gap-2 shrink-0">
			<!-- Copy raw markdown, with a dropdown for the download-as-file variant. -->
			<div class="flex items-stretch h-7 rounded-md border border-border-light overflow-hidden">
				<Button
					size="xs"
					variant="subtle"
					btnClasses="rounded-none text-secondary font-medium"
					startIcon={{ icon: copied ? Check : Copy }}
					onClick={copyRaw}
					title="Copy raw markdown"
				>
					{copied ? 'Copied' : 'Copy'}
				</Button>
				<Popover
					placement="bottom-end"
					contentClasses="p-0"
					class="px-1 flex items-center justify-center text-secondary hover:bg-surface-hover border-l border-border-light"
					triggerAttrs={{ 'aria-label': 'More export options' }}
				>
					{#snippet trigger()}
						<ChevronDown size={14} />
					{/snippet}
					{#snippet content({ close })}
						<div class="w-44 py-1 text-xs">
							<button
								type="button"
								class="w-full text-left text-secondary font-medium px-3 py-2 hover:bg-surface-hover flex items-center gap-2"
								onclick={() => {
									close()
									downloadFile()
								}}
							>
								<Download size={14} class="shrink-0 text-tertiary" />
								Download as .md
							</button>
						</div>
					{/snippet}
				</Popover>
			</div>
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
			<div
				class="prose dark:prose-invert max-w-none p-4
					prose-headings:font-semibold prose-headings:text-emphasis
					prose-pre:bg-transparent prose-pre:p-0
					prose-a:break-words prose-code:break-words prose-table:overflow-x-auto"
			>
				<Markdown md={artifact.content} {plugins} />
			</div>
		{/if}
	</div>
</div>
