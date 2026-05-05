<script lang="ts">
	import { ChevronRight, Pencil } from 'lucide-svelte'
	import { Alert, Button } from '$lib/components/common'
	import EditableInput from '$lib/components/common/EditableInput.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Path from '$lib/components/Path.svelte'
	import Label from '$lib/components/Label.svelte'
	import WorkspaceItemPicker from '$lib/components/WorkspaceItemPicker.svelte'
	import { isOwner } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { updateItemPathAndSummary, checkFlowOnBehalfOf } from './moveRenameManager'

	type Kind = 'flow' | 'script' | 'app'
	const KIND_LABEL: Record<Kind, string> = { flow: 'flows', script: 'scripts', app: 'apps' }

	interface Props {
		summary?: string
		path?: string
		/** Kind of the item being edited; used to label the first breadcrumb segment. */
		kind?: Kind
		onNavigate?: (item: { path: string; kind: Kind }) => void
		onSaved?: (newPath: string) => void
		penVisibility?: 'hover' | 'always'
		disabled?: boolean
	}

	let {
		summary = $bindable(''),
		path = $bindable(''),
		kind = 'flow',
		onNavigate,
		onSaved,
		penVisibility = 'hover',
		disabled = false
	}: Props = $props()

	let pickerTypeOpen = $state(false)
	let pickerScopeOpen = $state(false)
	let pickerSlugOpen = $state(false)
	let pathPopoverOpen = $state(false)
	let pickerType: WorkspaceItemPicker | undefined = $state()
	let pickerScope: WorkspaceItemPicker | undefined = $state()
	let pickerSlug: WorkspaceItemPicker | undefined = $state()

	// Path segments. e.g. "f/demo/weather_report" → scope "f/demo", slug "weather_report".
	let segments = $derived.by(() => {
		const parts = (path ?? '').split('/')
		if (parts.length < 3) return null
		return { scope: parts.slice(0, 2).join('/'), slug: parts.slice(2).join('/') }
	})

	const kindKey = (k: Kind) => `kind:${k}`
	const scopeKeyOf = (k: Kind, scope: string) => `scope:${k}:${scope}`
	const leafKeyOf = (k: Kind, p: string) => `leaf:${k}:${p}`

	let editPath = $state('')
	let dirtyPath = $state(false)
	let onBehalfOfEmail = $state<string | undefined>(undefined)

	let own = $derived(isOwner(path ?? '', $userStore, $workspaceStore))
	let hasPathChanges = $derived(own && dirtyPath)

	$effect(() => {
		if (pathPopoverOpen) {
			editPath = path ?? ''
			onBehalfOfEmail = undefined
			if ($workspaceStore && path) {
				checkFlowOnBehalfOf($workspaceStore, path).then((email) => {
					onBehalfOfEmail = email
				})
			}
		}
	})

	function handleSummarySave(newValue: string) {
		const trimmed = newValue.trim()
		if (trimmed === '') return
		summary = trimmed
	}

	function handlePickerSelect(item: { path: string; kind: Kind }) {
		pickerTypeOpen = false
		pickerScopeOpen = false
		pickerSlugOpen = false
		onNavigate?.(item)
	}

	async function savePath(close: () => void) {
		const initialPath = path ?? ''
		const newPath = own ? editPath : initialPath

		try {
			await updateItemPathAndSummary({
				workspace: $workspaceStore!,
				kind: 'flow',
				initialPath,
				newPath,
				newSummary: summary ?? ''
			})
			sendUserToast('Flow updated')
			path = newPath
			close()
			onSaved?.(newPath)
		} catch (e: any) {
			sendUserToast(`Could not update flow: ${e.body ?? e.message}`, true)
		}
	}
</script>

<div class="inline-block max-w-full align-top group px-2 py-0.5 leading-tight">
	<!-- Path row -->
	<div class="flex items-center max-w-full text-2xs text-secondary font-mono">
		<!-- Type segment -->
		<Popover
			placement="bottom-start"
			usePointerDownOutside
			excludeSelectors=".drawer"
			disableFocusTrap
			closeOnOtherPopoverOpen
			class="font-normal inline-flex items-center px-1 rounded hover:bg-surface-hover hover:text-primary transition-colors {disabled
				? 'cursor-default'
				: 'cursor-pointer'}"
			openFocus={() => {
				pickerType?.focus()
				return null
			}}
			bind:isOpen={pickerTypeOpen}
			{disabled}
		>
			{#snippet trigger()}{KIND_LABEL[kind]}{/snippet}
			{#snippet content()}
				<WorkspaceItemPicker
					bind:this={pickerType}
					initialOpen={[kindKey(kind)]}
					initialHighlight={kindKey(kind)}
					onPick={handlePickerSelect}
				/>
			{/snippet}
		</Popover>
		{#if segments}<Popover
				placement="bottom-start"
				usePointerDownOutside
				excludeSelectors=".drawer"
				disableFocusTrap
				closeOnOtherPopoverOpen
				class="font-normal inline-flex items-center gap-0.5 px-1 rounded min-w-0 max-w-[40%] hover:bg-surface-hover hover:text-primary transition-colors {disabled
					? 'cursor-default'
					: 'cursor-pointer'}"
				openFocus={() => {
					pickerScope?.focus()
					return null
				}}
				bind:isOpen={pickerScopeOpen}
				{disabled}
			>
				{#snippet trigger()}<ChevronRight size={10} class="shrink-0" /><span class="truncate"
						>{segments.scope}</span
					>{/snippet}
				{#snippet content()}
					<WorkspaceItemPicker
						bind:this={pickerScope}
						initialOpen={[kindKey(kind), scopeKeyOf(kind, segments.scope)]}
						initialHighlight={scopeKeyOf(kind, segments.scope)}
						onPick={handlePickerSelect}
					/>
				{/snippet}
			</Popover><Popover
				placement="bottom-start"
				usePointerDownOutside
				excludeSelectors=".drawer"
				disableFocusTrap
				closeOnOtherPopoverOpen
				class="font-normal inline-flex items-center gap-0.5 px-1 rounded min-w-0 hover:bg-surface-hover hover:text-primary transition-colors {disabled
					? 'cursor-default'
					: 'cursor-pointer'}"
				openFocus={() => {
					pickerSlug?.focus()
					return null
				}}
				bind:isOpen={pickerSlugOpen}
				{disabled}
			>
				{#snippet trigger()}<ChevronRight size={10} class="shrink-0" /><span class="truncate"
						>{segments.slug}</span
					>{/snippet}
				{#snippet content()}
					<WorkspaceItemPicker
						bind:this={pickerSlug}
						initialOpen={[kindKey(kind), scopeKeyOf(kind, segments.scope)]}
						initialHighlight={leafKeyOf(kind, path ?? '')}
						onPick={handlePickerSelect}
					/>
				{/snippet}
			</Popover>{/if}

		<!-- Pen → path-edit popover (summary hidden) -->
		<Popover
			placement="bottom-start"
			contentClasses="p-4"
			usePointerDownOutside
			excludeSelectors=".drawer"
			disableFocusTrap
			closeOnOtherPopoverOpen
			bind:isOpen={pathPopoverOpen}
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
					btnClasses={penVisibility === 'hover'
						? 'opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100'
						: ''}
				/>
			{/snippet}
			{#snippet content({ close })}
				<div class="flex flex-col gap-6 w-[480px]">
					<Label label="Path">
						{#if own}
							<Path
								autofocus={false}
								bind:path={editPath}
								bind:dirty={dirtyPath}
								initialPath={path ?? ''}
								namePlaceholder="flow"
								kind="flow"
								hideFullPath
								size="sm"
								drawerOffset={4000}
							/>
						{:else}
							<span class="text-xs font-mono text-secondary">{path}</span>
							<p class="text-2xs text-tertiary mt-1">Only the owner can change the path</p>
						{/if}
					</Label>
					{#if onBehalfOfEmail}
						<Alert type="info" title="Run on behalf of" size="xs">
							This flow will be redeployed on behalf of you ({$userStore?.email}) instead of {onBehalfOfEmail}
						</Alert>
					{/if}
					<Button
						size="xs"
						variant="accent"
						disabled={!hasPathChanges}
						title="Save path"
						onclick={() => savePath(close)}
					>
						Save
					</Button>
				</div>
			{/snippet}
		</Popover>
	</div>

	<!-- Summary -->
	<div class="max-w-full text-xs leading-tight" title={summary}>
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
