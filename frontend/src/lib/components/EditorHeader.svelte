<script lang="ts">
	import { Folder, Pencil, Plus } from 'lucide-svelte'
	import { Alert, Button } from '$lib/components/common'
	import EditableInput from '$lib/components/common/EditableInput.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Path from '$lib/components/Path.svelte'
	import Label from '$lib/components/Label.svelte'
	import LabelsInput from '$lib/components/LabelsInput.svelte'
	import { isOwner } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { updateItemPathAndSummary, checkFlowOnBehalfOf } from './moveRenameManager'

	interface Props {
		summary?: string
		path?: string
		labels?: string[] | undefined
		isNew?: boolean
		onNavigate?: (path: string) => void
		onSaved?: (newPath: string) => void
		penVisibility?: 'hover' | 'always'
		disabled?: boolean
	}

	let {
		summary = $bindable(''),
		path = $bindable(''),
		labels = $bindable(),
		isNew = false,
		onNavigate,
		onSaved,
		penVisibility = 'hover',
		disabled = false
	}: Props = $props()

	let pickerOpen = $state(false)
	let pathPopoverOpen = $state(false)

	let editPath = $state('')
	let dirtyPath = $state(false)
	let labelsDirty = $state(false)
	let onBehalfOfEmail = $state<string | undefined>(undefined)

	let own = $derived(isOwner(path ?? '', $userStore, $workspaceStore))
	let hasPathChanges = $derived((own && dirtyPath) || labelsDirty)

	$effect(() => {
		if (pathPopoverOpen) {
			editPath = path ?? ''
			labelsDirty = false
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

	// Will be called by the file picker (TODO) when the user selects a flow.
	function handlePickerSelect(newPath: string) {
		pickerOpen = false
		onNavigate?.(newPath)
	}
	void handlePickerSelect

	async function savePath(close: () => void) {
		const initialPath = path ?? ''
		const newPath = own ? editPath : initialPath

		try {
			await updateItemPathAndSummary({
				workspace: $workspaceStore!,
				kind: 'flow',
				initialPath,
				newPath,
				newSummary: summary ?? '',
				labels
			})
			sendUserToast('Flow updated')
			labelsDirty = false
			path = newPath
			close()
			onSaved?.(newPath)
		} catch (e: any) {
			sendUserToast(`Could not update flow: ${e.body ?? e.message}`, true)
		}
	}
</script>

<div class="inline-block max-w-full align-top group px-2 py-1 leading-tight">
	<!-- Summary -->
	<div class="max-w-full" title={summary}>
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

	<!-- Path row -->
	<div class="flex items-center gap-0.5 max-w-full">
		{#if isNew}
			<Popover
				placement="bottom-start"
				contentClasses="p-4"
				usePointerDownOutside
				excludeSelectors=".drawer"
				disableFocusTrap
				bind:isOpen={pickerOpen}
				{disabled}
			>
				{#snippet trigger()}
					<button
						type="button"
						class="inline-flex items-center gap-1 rounded text-2xs text-tertiary hover:bg-surface-hover transition-colors"
						{disabled}
					>
						<Plus size={12} />
						<span>Set path</span>
					</button>
				{/snippet}
				{#snippet content()}
					<div class="w-[400px] text-xs text-tertiary">
						<!-- File picker placeholder -->
					</div>
				{/snippet}
			</Popover>
		{:else}
			<!-- Path chip → picker popover -->
			<Popover
				placement="bottom-start"
				contentClasses="p-4"
				usePointerDownOutside
				excludeSelectors=".drawer"
				disableFocusTrap
				bind:isOpen={pickerOpen}
				{disabled}
			>
				{#snippet trigger()}
					<button
						type="button"
						title={path}
						class="inline-flex items-center gap-1 rounded min-w-0 max-w-full text-2xs text-tertiary font-mono hover:bg-surface-hover transition-colors {disabled
							? 'cursor-default'
							: 'cursor-pointer'}"
						{disabled}
					>
						<Folder size={12} class="shrink-0" />
						<span class="truncate">{path}</span>
					</button>
				{/snippet}
				{#snippet content()}
					<div class="w-[400px] text-xs text-tertiary">
						<!-- File picker placeholder -->
					</div>
				{/snippet}
			</Popover>

			<!-- Pen → path-edit popover (summary hidden) -->
			<Popover
				placement="bottom-start"
				contentClasses="p-4"
				usePointerDownOutside
				excludeSelectors=".drawer"
				disableFocusTrap
				bind:isOpen={pathPopoverOpen}
				{disabled}
			>
				{#snippet trigger()}
					<button
						type="button"
						aria-label="Edit path"
						class="inline-flex items-center justify-center p-0.5 rounded text-tertiary hover:bg-surface-hover hover:text-primary transition-opacity {penVisibility ===
						'hover'
							? 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'
							: ''}"
						{disabled}
					>
						<Pencil size={12} />
					</button>
				{/snippet}
				{#snippet content({ close })}
					<div class="flex flex-col gap-6 w-[480px]">
						<LabelsInput
							bind:labels
							onchange={() => {
								labelsDirty = true
							}}
						/>
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
		{/if}
	</div>
</div>
