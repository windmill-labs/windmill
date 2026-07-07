<script lang="ts">
	import { Pencil } from 'lucide-svelte'
	import { Alert, Button } from '$lib/components/common'
	import EditableInput from '$lib/components/common/EditableInput.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Path from '$lib/components/Path.svelte'
	import Label from '$lib/components/Label.svelte'
	import {
		dirKey,
		KIND_LABEL_LOWER,
		kindKey,
		leafKeyFor,
		type WorkspaceItem,
		type WorkspaceItemKind
	} from '$lib/components/workspacePicker'
	import BreadcrumbSegment from '$lib/components/BreadcrumbSegment.svelte'
	import { isOwner } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'

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
		/** Workspace whose items the breadcrumb picker lists. Session live
		 * editors pass their acting workspace so the picker isn't scoped to the
		 * navigation workspace; falls back to $workspaceStore in the picker. */
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
		workspaceId
	}: Props = $props()

	let pathPopoverOpen = $state(false)
	/** Snapshot of `path` taken when the pen popover opens; cleared on close.
	 * While set, the breadcrumb derives from it instead of `path` so the pen
	 * anchor doesn't reflow as the user types in the popover (which would drag
	 * the popover with it via floating-ui's auto-update). */
	let snapshotPath = $state<string | undefined>(undefined)

	function setPathPopoverOpen(open: boolean) {
		pathPopoverOpen = open
		// Only snapshot when there's a path to freeze. New flows / scripts open
		// the popover with `path === ''` and rely on `Path.reset()` to seed a
		// path; if we snapshot the empty value the breadcrumb stays blank for
		// the whole popover lifetime. Leaving `snapshotPath` undefined lets
		// `displayPath` track the live seeded path instead.
		snapshotPath = open && path ? path : undefined
	}

	// Path segments. e.g. "f/demo/weather_report" → scope "f/demo", slug "weather_report".
	// `snapshotPath` keeps the breadcrumb frozen while the pen popover is open
	// so the trigger doesn't drift mid-edit. Outside of that, the breadcrumb
	// follows the live `path` — coherent with the picker because we inject a
	// virtual current-item entry at that same live path (see `currentItem`).
	let displayPath = $derived(snapshotPath ?? path ?? '')
	/** Breakdown for breadcrumb rendering. Top-level dirs (scopes) come first
	 * — `f/<folder>` or `u/<user>` — then any number of subfolders, then the
	 * item name. Each dir entry's `fullPath` is the cumulative path up to
	 * that level, used to key the picker's `initialOpen` / `initialHighlight`. */
	let segments = $derived.by(() => {
		const parts = displayPath.split('/')
		if (parts.length < 3) return null
		const scope = parts.slice(0, 2).join('/')
		const slug = parts.slice(2)
		const dirs: { name: string; fullPath: string }[] = []
		// Top-level scope is a dir.
		dirs.push({ name: scope, fullPath: scope })
		// Intermediate slug parts (everything except the last one).
		let acc = scope
		for (let i = 0; i < slug.length - 1; i++) {
			acc = `${acc}/${slug[i]}`
			dirs.push({ name: slug[i], fullPath: acc })
		}
		const leaf = { name: slug[slug.length - 1], fullPath: displayPath }
		return { dirs, leaf }
	})

	// Treat an empty path as ownable so the pen popover lets a user pick the
	// path for a brand-new item. `Path.reset()` then synthesizes a default
	// under their own user/folder scope.
	let own = $derived(!path || isOwner(path, $userStore, $workspaceStore))

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

<div class="inline-block max-w-full align-top group px-2 py-0.5 leading-tight">
	<!-- Path row -->
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
					<BreadcrumbSegment
						label={dir.name}
						withChevron
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
				{@const leafKey = leafKeyFor(kind, segments.leaf.fullPath)}
				{@const leafParent = segments.dirs[segments.dirs.length - 1]?.fullPath}
				<BreadcrumbSegment
					label={segments.leaf.name}
					withChevron
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

		<!-- Pen → path-edit popover. Skipped entirely when path editing is
		     disabled so the user doesn't see an inert button. -->
		{#if pathEditable}
			<Popover
				placement="bottom-start"
				contentClasses="p-4"
				usePointerDownOutside
				excludeSelectors=".drawer"
				disableFocusTrap
				closeOnOtherPopoverOpen
				bind:isOpen={() => pathPopoverOpen, setPathPopoverOpen}
			>
				{#snippet trigger()}
					<Button
						variant="subtle"
						unifiedSize="xs"
						iconOnly
						startIcon={{ icon: Pencil }}
						title="Edit path"
						aria-label="Edit path"
						btnClasses={penVisibility === 'hover' && !pathPopoverOpen
							? 'opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100'
							: ''}
					/>
				{/snippet}
				{#snippet content()}
					<div class="flex flex-col gap-6 w-[480px]">
						{#if own}
							<Path
								autofocus
								bind:path
								initialPath={snapshotPath ?? path ?? ''}
								namePlaceholder={kind}
								{kind}
								size="sm"
								drawerOffset={4000}
							/>
							{#if savedPath && path && path !== savedPath}
								<Alert
									type="info"
									size="xs"
									title="Deploy the {kind} to make the path change effective."
								/>
							{/if}
							{#if onBehalfOfEmail}
								<Alert type="info" title="Run on behalf of" size="xs">
									This flow will be redeployed on behalf of you ({$userStore?.email}) instead of {onBehalfOfEmail}
								</Alert>
							{/if}
						{:else}
							<Label label="Path">
								<span class="text-xs font-mono text-secondary">{path}</span>
								<p class="text-2xs text-tertiary mt-1">Only the owner can change the path</p>
							</Label>
						{/if}
					</div>
				{/snippet}
			</Popover>
		{/if}
	</div>

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
