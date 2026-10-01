<script lang="ts">
	import { ChevronRight } from 'lucide-svelte'
	import EditableInput from '$lib/components/common/EditableInput.svelte'
	import PathEditPopover from '$lib/components/PathEditPopover.svelte'
	import {
		dirKey,
		KIND_LABEL_LOWER,
		kindKey,
		leafKeyFor,
		type WorkspaceItem,
		type WorkspaceItemKind
	} from '$lib/components/workspacePicker'
	import BreadcrumbSegment from '$lib/components/BreadcrumbSegment.svelte'
	import { splitItemPath } from '$lib/components/breadcrumbPath'
	interface Props {
		summary?: string
		path?: string
		/** The item's *saved* path on the server. Used so the picker knows to
		 * replace the saved entry with a virtual one at the live `path` while
		 * the user is mid-rename — keeps the breadcrumb and picker tree coherent
		 * (the breadcrumb shows the user's draft path, and the picker shows the
		 * item at that same path even though the rename hasn't been deployed). */
		savedPath?: string
		/** Kind of the item being edited; used to label the first breadcrumb segment. */
		kind?: WorkspaceItemKind
		/** True when editing a raw app — forwarded into the virtual `currentItem`
		 * so the picker / `editPathFor` can route to `/apps_raw/...`. */
		raw_app?: boolean
		onNavigate?: (item: WorkspaceItem) => void
		/** When set, shows a "redeployed on behalf of" warning in the path-edit
		 * popover. Parents already know this from their loaded item — no API call. */
		onBehalfOfEmail?: string | undefined
		penVisibility?: 'hover' | 'always'
		/** When false, the summary input becomes read-only. Breadcrumb
		 * navigation and the pen popover are unaffected. */
		summaryEditable?: boolean
		/** When false, the pen popover is hidden so the path can't be edited
		 * inline. Breadcrumb navigation still works — only the rename UI is
		 * gated. */
		pathEditable?: boolean
		/** When true, the whole path/breadcrumb row (and its pen popover) is
		 * dropped, leaving only the summary. Used by the condensed session-
		 * preview top bar to save vertical room. */
		hidePath?: boolean
		/** Workspace whose items the breadcrumb picker lists; defaults to the operating
		 * workspace (see `useOperatingWorkspace`). */
		workspaceId?: string
	}

	let {
		summary = $bindable(),
		path = $bindable(),
		savedPath,
		kind = 'flow',
		raw_app = false,
		onNavigate,
		onBehalfOfEmail,
		penVisibility = 'hover',
		summaryEditable = true,
		pathEditable = true,
		hidePath = false,
		workspaceId
	}: Props = $props()

	/** Held by the pen popover while it is open, so the trail below — and the pen anchored to its
	 * end — does not reflow as the user types. */
	let snapshotPath = $state<string | undefined>(undefined)

	// Path segments. e.g. "f/demo/weather_report" → scope "f/demo", slug "weather_report".
	// `snapshotPath` keeps the breadcrumb frozen while the pen popover is open
	// so the trigger doesn't drift mid-edit. Outside of that, the breadcrumb
	// follows the live `path` — coherent with the picker because we inject a
	// virtual current-item entry at that same live path (see `currentItem`).
	let displayPath = $derived(snapshotPath ?? path ?? '')
	let segments = $derived(splitItemPath(displayPath) ?? null)

	// Virtual entry for the picker: surfaces the currently-edited item at its
	// live path (which may differ from `savedPath` mid-rename, so the picker
	// otherwise has no leaf to expand).
	let currentItem = $derived<WorkspaceItem & { savedPath?: string }>({
		path: path ?? '',
		summary: summary ?? '',
		kind,
		raw_app,
		savedPath
	})

	function handleSummarySave(newValue: string) {
		summary = newValue.trim()
	}

	function handlePickerSelect(item: WorkspaceItem) {
		onNavigate?.(item)
	}
</script>

<div class="max-w-full group px-2 py-0.5 leading-tight inline-block align-top">
	<!-- Path row -->
	{#if !hidePath}
		<div class="flex items-center max-w-full text-2xs text-secondary font-mono">
			<nav aria-label="Breadcrumb" class="contents">
				<BreadcrumbSegment
					label={KIND_LABEL_LOWER[kind]}
					initialScope={undefined}
					initialHighlight={kindKey(kind)}
					isCurrent={!segments}
					{currentItem}
					{workspaceId}
					onPick={handlePickerSelect}
				/>
				{#if segments}
					{#each segments.dirs as dir, i (dir.fullPath)}
						{@const dKey = dirKey('all', dir.fullPath)}
						<ChevronRight size={10} class="shrink-0" />
						<BreadcrumbSegment
							label={dir.name}
							extraClass={i === 0 ? 'gap-0.5 min-w-0 max-w-[40%]' : 'gap-0.5 min-w-0'}
							initialScope={i === 0
								? { kind: 'all' }
								: { kind: 'all', dir: segments.dirs[i - 1].fullPath }}
							initialHighlight={dKey}
							{currentItem}
							{workspaceId}
							onPick={handlePickerSelect}
						/>
					{/each}
				{/if}
				{#if segments?.leaf}
					{@const leafKey = leafKeyFor(kind, segments.leaf.fullPath)}
					{@const leafParent = segments.dirs[segments.dirs.length - 1]?.fullPath}
					<ChevronRight size={10} class="shrink-0" />
					<BreadcrumbSegment
						label={segments.leaf.name}
						extraClass="gap-0.5 min-w-0"
						initialScope={leafParent ? { kind: 'all', dir: leafParent } : { kind: 'all' }}
						initialHighlight={leafKey}
						isCurrent
						{currentItem}
						{workspaceId}
						onPick={handlePickerSelect}
					/>
				{/if}
			</nav>

			<!-- Skipped entirely when path editing is disabled so the user doesn't see an inert
			     button. -->
			{#if pathEditable}
				<PathEditPopover
					bind:path
					bind:snapshotPath
					{savedPath}
					{kind}
					{workspaceId}
					{onBehalfOfEmail}
					{penVisibility}
				/>
			{/if}
		</div>
	{/if}

	<!-- Summary -->
	<div class="max-w-full -mt-[2px]" title={summary}>
		<EditableInput
			value={summary ?? ''}
			placeholder="Add a summary..."
			editable={summaryEditable}
			commitOnInput
			size="sm"
			onSave={handleSummarySave}
			textClass="text-xs font-semibold text-emphasis leading-tight"
			class="max-w-full"
		/>
	</div>
</div>
