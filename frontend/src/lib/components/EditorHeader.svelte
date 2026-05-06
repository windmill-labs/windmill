<script lang="ts">
	import { ChevronRight, Pencil } from 'lucide-svelte'
	import { Alert, Button } from '$lib/components/common'
	import EditableInput from '$lib/components/common/EditableInput.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Path from '$lib/components/Path.svelte'
	import Label from '$lib/components/Label.svelte'
	import WorkspaceItemPicker, {
		type WorkspaceItem,
		type WorkspaceItemKind
	} from '$lib/components/WorkspaceItemPicker.svelte'
	import { isOwner } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'

	const KIND_LABEL: Record<WorkspaceItemKind, string> = {
		flow: 'flows',
		script: 'scripts',
		app: 'apps'
	}

	interface Props {
		summary?: string
		path?: string
		/** The item's *saved* path on the server. When set, the breadcrumb tracks
		 * this rather than the live (possibly draft-renamed) `path`, so clicking a
		 * breadcrumb segment opens the picker scoped to where the item actually is.
		 * The user's in-progress rename remains visible in the pen popover.
		 * Falls back to `path` when omitted. */
		savedPath?: string
		/** Kind of the item being edited; used to label the first breadcrumb segment. */
		kind?: WorkspaceItemKind
		onNavigate?: (item: WorkspaceItem) => void
		/** When set, shows a "redeployed on behalf of" warning in the path-edit
		 * popover. Parents already know this from their loaded item — no API call. */
		onBehalfOfEmail?: string | undefined
		penVisibility?: 'hover' | 'always'
		disabled?: boolean
	}

	let {
		summary = $bindable(''),
		path = $bindable(''),
		savedPath,
		kind = 'flow',
		onNavigate,
		onBehalfOfEmail,
		penVisibility = 'hover',
		disabled = false
	}: Props = $props()

	let pickerTypeOpen = $state(false)
	let pickerScopeOpen = $state(false)
	let pickerSlugOpen = $state(false)
	let pathPopoverOpen = $state(false)
	/** Snapshot of `path` taken when the pen popover opens; cleared on close.
	 * While set, the breadcrumb derives from it instead of `path` so the pen
	 * anchor doesn't reflow as the user types in the popover (which would drag
	 * the popover with it via floating-ui's auto-update). */
	let snapshotPath = $state<string | undefined>(undefined)

	function setPathPopoverOpen(open: boolean) {
		pathPopoverOpen = open
		snapshotPath = open ? (path ?? '') : undefined
	}

	// Path segments. e.g. "f/demo/weather_report" → scope "f/demo", slug "weather_report".
	// Prefer `savedPath` (the item's location on the server) so the breadcrumb and
	// the picker stay coherent during a draft rename — the renamed path may not
	// correspond to any real items yet, which would leave the picker empty.
	// `snapshotPath` keeps the breadcrumb frozen while the pen popover is open
	// so the trigger doesn't drift mid-edit.
	let displayPath = $derived(snapshotPath ?? savedPath ?? path ?? '')
	let segments = $derived.by(() => {
		const parts = displayPath.split('/')
		if (parts.length < 3) return null
		return { scope: parts.slice(0, 2).join('/'), slug: parts.slice(2).join('/') }
	})

	const kindKey = (k: WorkspaceItemKind) => `kind:${k}`
	const scopeKeyOf = (k: WorkspaceItemKind, scope: string) => `scope:${k}:${scope}`
	const leafKeyOf = (k: WorkspaceItemKind, p: string) => `leaf:${k}:${p}`

	let own = $derived(isOwner(path ?? '', $userStore, $workspaceStore))

	function handleSummarySave(newValue: string) {
		const trimmed = newValue.trim()
		if (trimmed === '') return
		summary = trimmed
	}

	function handlePickerSelect(item: WorkspaceItem) {
		pickerTypeOpen = false
		pickerScopeOpen = false
		pickerSlugOpen = false
		onNavigate?.(item)
	}

	const SEGMENT_BASE_CLASS =
		'font-normal inline-flex items-center px-1 rounded hover:bg-surface-hover hover:text-primary transition-colors'
</script>

{#snippet breadcrumbSegment(
	label: string,
	getOpen: () => boolean,
	setOpen: (v: boolean) => void,
	initialOpen: string[],
	initialHighlight: string,
	withChevron: boolean,
	extraClass: string
)}
	<Popover
		placement="bottom-start"
		usePointerDownOutside
		excludeSelectors=".drawer"
		disableFocusTrap
		closeOnOtherPopoverOpen
		class="{SEGMENT_BASE_CLASS} {extraClass} {disabled ? 'cursor-default' : 'cursor-pointer'}"
		bind:isOpen={getOpen, setOpen}
		openFocus="[data-workspace-picker-search]"
		{disabled}
	>
		{#snippet trigger()}
			{#if withChevron}<ChevronRight size={10} class="shrink-0" /><span class="truncate"
					>{label}</span
				>{:else}{label}{/if}
		{/snippet}
		{#snippet content()}
			<WorkspaceItemPicker {initialOpen} {initialHighlight} onPick={handlePickerSelect} />
		{/snippet}
	</Popover>
{/snippet}

<div class="inline-block max-w-full align-top group px-2 py-0.5 leading-tight">
	<!-- Path row -->
	<div class="flex items-center max-w-full text-2xs text-secondary font-mono">
		{@render breadcrumbSegment(
			KIND_LABEL[kind],
			() => pickerTypeOpen,
			(v) => (pickerTypeOpen = v),
			[kindKey(kind)],
			kindKey(kind),
			false,
			''
		)}
		{#if segments}
			{@render breadcrumbSegment(
				segments.scope,
				() => pickerScopeOpen,
				(v) => (pickerScopeOpen = v),
				[kindKey(kind), scopeKeyOf(kind, segments.scope)],
				scopeKeyOf(kind, segments.scope),
				true,
				'gap-0.5 min-w-0 max-w-[40%]'
			)}
			{@render breadcrumbSegment(
				segments.slug,
				() => pickerSlugOpen,
				(v) => (pickerSlugOpen = v),
				[kindKey(kind), scopeKeyOf(kind, segments.scope)],
				leafKeyOf(kind, displayPath),
				true,
				'gap-0.5 min-w-0'
			)}
		{/if}

		<!-- Pen → path-edit popover -->
		<Popover
			placement="bottom-start"
			contentClasses="p-4"
			usePointerDownOutside
			excludeSelectors=".drawer"
			disableFocusTrap
			closeOnOtherPopoverOpen
			bind:isOpen={() => pathPopoverOpen, setPathPopoverOpen}
			{disabled}
		>
			{#snippet trigger()}
				<Button
					variant="subtle"
					unifiedSize="xs"
					iconOnly
					startIcon={{ icon: Pencil }}
					title="Edit path"
					aria-label="Edit path"
					{disabled}
					btnClasses={penVisibility === 'hover' && !pathPopoverOpen
						? 'opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100'
						: ''}
				/>
			{/snippet}
			{#snippet content()}
				<div class="flex flex-col gap-6 w-[480px]">
					{#if own}
						<Path
							autofocus={false}
							bind:path
							initialPath={snapshotPath ?? path ?? ''}
							namePlaceholder={kind}
							{kind}
							hideFullPath
							size="sm"
							drawerOffset={4000}
						/>
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
	</div>

	<!-- Summary -->
	<div class="max-w-full -mt-[2px]" title={summary}>
		<EditableInput
			value={summary ?? ''}
			placeholder="Add a summary..."
			editable={!disabled}
			size="sm"
			onSave={handleSummarySave}
			textClass="text-xs font-semibold text-emphasis leading-tight"
			class="max-w-full"
		/>
	</div>
</div>
