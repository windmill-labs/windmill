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
	import {
		artifactFilename,
		artifactMimeType,
		currentVersion,
		type ArtifactVersion,
		type PersistedArtifact
	} from './artifactsDB'
	import { markdownProse } from '$lib/components/markdownProse'
	import ArtifactVersionPicker from './ArtifactVersionPicker.svelte'
	import type { SessionArtifactsStore } from './artifactsState.svelte'
	import { History } from 'lucide-svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'

	interface Props {
		artifact: PersistedArtifact
		store: SessionArtifactsStore
	}

	let { artifact, store }: Props = $props()

	const latest = $derived(currentVersion(artifact))
	// Nothing to pick between until a second version exists.
	const hasHistory = $derived(latest > 1)

	// undefined = following the current version. An explicit pick survives later edits, so
	// the AI writing v8 does not yank the reader out of the v3 they chose to read.
	let pinned = $state<number | undefined>(undefined)
	let pinnedContent = $state<ArtifactVersion | undefined>(undefined)

	// The store hands us a fresh object on every edit, so this effect reruns constantly.
	// Compare the id against the last one seen: clearing on every rerun would drop the
	// reader's pin the moment the assistant writes a new version.
	let pinnedFor: string | undefined
	$effect(() => {
		if (artifact.id === pinnedFor) return
		pinnedFor = artifact.id
		pinned = undefined
	})

	$effect(() => {
		const version = pinned
		if (version === undefined) {
			pinnedContent = undefined
			return
		}
		const id = artifact.id
		void store.getVersion(id, version).then((snapshot) => {
			if (pinned !== version || artifact.id !== id) return
			// Pruned out from under the pin (history is capped): fall back to current rather
			// than showing an empty document.
			if (!snapshot) {
				pinned = undefined
				return
			}
			pinnedContent = snapshot
		})
	})

	const shown = $derived(pinnedContent ?? artifact)
	// Label from the snapshot that is rendered, not from the one just requested: the read is
	// async, so `pinned` names a version the body has not swapped to yet.
	const shownVersion = $derived(pinnedContent?.version)

	// Markdown is the only rendered kind in v1; anything else shows source only.
	const canPreview = $derived(artifact.kind === 'md')
	let showSource = $state(false)
	const source = $derived(!canPreview || showSource)

	let copied = $state(false)
	async function copyRaw() {
		if (!(await copyToClipboard(shown.content))) return
		copied = true
		setTimeout(() => (copied = false), 1500)
	}
	function downloadFile() {
		download(
			artifactFilename({ name: shown.name, kind: artifact.kind }),
			shown.content,
			artifactMimeType(artifact.kind)
		)
	}

	const plugins = [gfmPlugin(), { renderer: { pre: CodeDisplay, a: LinkRenderer } }]
</script>

<div class="flex flex-col h-full bg-surface-tertiary">
	<div class="flex items-center justify-between gap-2 px-8 py-2">
		<div class="flex items-center gap-1.5 min-w-0 flex-1">
			<FileText size={14} class="shrink-0 text-secondary" />
			<span class="truncate text-xs font-normal text-emphasis" title={shown.name}>
				{shown.name}
			</span>
			{#if hasHistory}
				<ArtifactVersionPicker
					{artifact}
					{store}
					selected={shownVersion}
					onSelect={(v) => (pinned = v)}
				/>
			{/if}
		</div>
		<div class="flex items-center gap-2 shrink-0">
			<!-- Copy raw markdown, with a dropdown for the download-as-file variant. -->
			<Button
				unifiedSize="sm"
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

	{#if pinnedContent}
		<!-- Everything below is stale text; say so where it cannot be scrolled past unnoticed. -->
		<div
			class="flex items-center gap-2 px-8 py-1 text-2xs font-normal
				bg-orange-100 text-orange-800 dark:bg-orange-950 dark:text-orange-300"
		>
			<History size={12} class="shrink-0" />
			<span class="truncate">
				<!-- compact renders the magnitude only ("8m"); the phrasing belongs to the sentence. -->
				Viewing v{shownVersion} of {latest} · saved
				<TimeAgo date={new Date(pinnedContent.savedAt).toISOString()} compact /> ago
			</span>
			<div class="ml-auto shrink-0">
				<Button unifiedSize="xs" variant="default" onClick={() => (pinned = undefined)}>
					Back to latest
				</Button>
			</div>
		</div>
	{/if}

	<div class="flex-1 min-h-0 overflow-auto px-8">
		{#if source}
			<!-- key: SimpleEditor reads `code` only on init; remount on id or content change. -->
			{#key `${artifact.id}:${pinnedContent ? `v${pinnedContent.version}` : artifact.updatedAt}`}
				<SimpleEditor lang="markdown" code={shown.content} readOnly class="h-full" />
			{/key}
		{:else}
			<!-- Pinned under the header, fades scrolled-under content instead of hard-clipping it.
			     The negative margin cancels its flow height so it overlays instead of pushing. -->
			<div class="sticky top-0 z-10 h-4 -mb-4 bg-gradient-to-b from-surface-tertiary to-transparent"
			></div>
			<div class="pb-4 pt-2 {markdownProse.doc}">
				<Markdown md={shown.content} {plugins} />
			</div>
		{/if}
	</div>
</div>
