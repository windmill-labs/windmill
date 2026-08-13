<script lang="ts">
	import { untrack } from 'svelte'
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
		/** Version being viewed, or undefined for the current one. Owned by the caller — it
		 * outlives this component, so the viewer must not keep a copy of its own. */
		pinned?: number
		onPin: (version: number | undefined) => void
	}

	let { artifact, store, pinned, onPin }: Props = $props()

	const latest = $derived(currentVersion(artifact))
	// Nothing to pick between until a second version exists.
	const hasHistory = $derived(latest > 1)

	let pinnedContent = $state<ArtifactVersion | undefined>(undefined)
	// A pin that arrives with the tab has nothing honest to show until its snapshot lands: the
	// latest document is not the version asked for, and the banner saying so is keyed on the
	// snapshot itself. A pin picked while reading keeps the document already on screen instead.
	let restoringPin = $state(untrack(() => pinned !== undefined))
	let readAttempt = $state(0)

	/** Pick a version to show. Re-picking the one already pinned leaves the url unchanged, so
	 * only the attempt counter can re-read it — the reader's one way back from a failed read. */
	function selectVersion(version: number | undefined) {
		if (version === pinned) readAttempt++
		onPin(version)
	}

	$effect(() => {
		const version = pinned
		void readAttempt // dependency: see selectVersion
		if (version === undefined) {
			pinnedContent = undefined
			restoringPin = false
			return
		}
		const id = artifact.id
		void store.getVersion(id, version).then(
			(snapshot) => {
				if (pinned !== version || artifact.id !== id) return
				restoringPin = false
				// Pruned out from under the pin (history is capped): fall back to current rather
				// than showing an empty document.
				if (!snapshot) {
					onPin(undefined)
					return
				}
				pinnedContent = snapshot
			},
			() => {
				// A failed read is no evidence the version is gone, so the pin stays on the tab and
				// the document already on screen stays up — the latest, or the snapshot of whatever
				// version was pinned before this pick.
				if (pinned === version && artifact.id === id) restoringPin = false
			}
		)
	})

	const shown = $derived(pinnedContent ?? artifact)
	// Label the snapshot that is rendered, not the one just requested: until it lands the body
	// is still on the previous version, and naming the new one would misreport it.
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
				<!-- The body is blank while restoring, so the chip names the version being fetched:
				     the latest is the one version certainly not on screen. -->
				<ArtifactVersionPicker
					{artifact}
					{store}
					selected={restoringPin ? pinned : shownVersion}
					onSelect={selectVersion}
				/>
			{/if}
		</div>
		<div class="flex items-center gap-2 shrink-0">
			<!-- Copy raw markdown, with a dropdown for the download-as-file variant. Both export
			     `shown`, which is still the current document while a pin is restoring — so both
			     are disabled, the item explicitly: a Button's `disabled` does not reach it. -->
			<Button
				unifiedSize="sm"
				variant="default"
				disabled={restoringPin}
				startIcon={{ icon: copied ? Check : Copy }}
				onClick={copyRaw}
				title="Copy raw markdown"
				dropdownItems={[
					{
						label: 'Download as .md',
						icon: Download,
						onClick: downloadFile,
						disabled: restoringPin
					}
				]}
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
				<Button unifiedSize="xs" variant="default" onClick={() => selectVersion(undefined)}>
					Back to latest
				</Button>
			</div>
		</div>
	{/if}

	<div class="flex-1 min-h-0 overflow-auto px-8">
		{#if restoringPin}
			<!-- Deliberately blank until the pinned snapshot lands; see restoringPin. -->
		{:else if source}
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
