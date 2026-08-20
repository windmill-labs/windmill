<script lang="ts">
	import { untrack } from 'svelte'
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { Code, Eye, FileText, Copy, Check, Download, ClipboardList } from 'lucide-svelte'
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
	import { twMerge } from 'tailwind-merge'
	import { planBadge, planVersionView, PLAN_MODE_TEXT_COLOR } from '../planMode'

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
		// A pin not behind the document is cleared rather than shown: the stale bar below keys on
		// the snapshot alone, so it would label current text as history.
		// `latest` untracked — as a dependency it would re-read the snapshot on every version added.
		if (version === undefined || version >= untrack(() => latest)) {
			pinnedContent = undefined
			restoringPin = false
			if (version !== undefined) onPin(undefined)
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
	const isPlan = $derived(artifact.role === 'plan')
	// Which pill and which bar this version earns — one place, so the list and this header
	// cannot disagree about what counts as the plan.
	// Silent until the snapshot lands, like the body below: `shownVersion` is still the head
	// then, so a plan opened at the version its reader approved would wear the draft's badge
	// and warning for the length of the read. Judging `pinned` instead would print the
	// approved signal over text that is still the draft, which is worse.
	const view = $derived(
		restoringPin
			? { badge: undefined, bar: undefined, backToPlan: undefined }
			: planVersionView(artifact, shownVersion)
	)
	const badge = $derived(planBadge(view.badge))
	// Browsing history offers the plan rather than the newest text, since that is what the
	// user settled on — and the bar on the plan leads on to the draft, so neither needs a
	// second button. The approved version is never pruned, so it is always still reachable.
	const backTo = $derived(view.backToPlan)
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
			{#if isPlan}
				<ClipboardList size={14} class={twMerge('shrink-0', PLAN_MODE_TEXT_COLOR)} />
			{:else}
				<FileText size={14} class="shrink-0 text-secondary" />
			{/if}
			<span class="truncate text-xs font-normal text-emphasis" title={shown.name}>
				{shown.name}
			</span>
			{#if badge}
				<!-- Sized against the title beside it, not the Copy button opposite: at the
				     list's text-2xs it reads as heavy as the 12px name it annotates.
				     `pt-px pb-0` is optical, not a typo: an all-caps word leaves the line
				     box's descender space empty, so symmetric padding sits it 1px high. -->
				<span
					class={twMerge(
						'shrink-0 rounded px-1 pt-px pb-0 text-3xs uppercase tracking-wide',
						badge.class
					)}
				>
					{badge.label}
				</span>
			{/if}
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

	{#if view.bar === 'approved-with-newer'}
		<!-- Before the stale bar below, because this version is reached by pinning too — and
		     it is the one pinned version that is not stale. -->
		<div
			class="flex items-center gap-2 px-8 py-1 text-2xs font-normal
				bg-teal-600/10 text-teal-700 dark:bg-teal-500/10 dark:text-teal-500"
		>
			<ClipboardList size={12} class="shrink-0" />
			<span class="truncate">
				This is the plan you approved · a newer draft (v{latest}) is not approved
			</span>
			<div class="ml-auto shrink-0">
				<Button unifiedSize="xs" variant="default" onClick={() => selectVersion(undefined)}>
					View last draft
				</Button>
			</div>
		</div>
	{:else if pinnedContent}
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
				<Button unifiedSize="xs" variant="default" onClick={() => selectVersion(backTo)}>
					{backTo === undefined ? 'Back to latest' : `Back to the plan (v${backTo})`}
				</Button>
			</div>
		</div>
	{:else if view.bar === 'unapproved-head'}
		<!-- Same bar as above rather than a second signal: both say "this is not the text you
		     settled on", and they are mutually exclusive, so only one is ever on screen. -->
		<div
			class="flex items-center gap-2 px-8 py-1 text-2xs font-normal
				bg-orange-100 text-orange-800 dark:bg-orange-950 dark:text-orange-300"
		>
			<ClipboardList size={12} class="shrink-0" />
			<span class="truncate">
				This revision is not approved · the plan you approved is v{view.backToPlan}
			</span>
			<div class="ml-auto shrink-0">
				<Button unifiedSize="xs" variant="default" onClick={() => selectVersion(view.backToPlan)}>
					View the plan
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
