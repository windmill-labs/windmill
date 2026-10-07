<script lang="ts">
	import type { Snippet } from 'svelte'
	import { emptyString, isOwner } from '$lib/utils'
	import { Alert, Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import { userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { updateItemPathAndSummary, checkFlowOnBehalfOf } from './moveRenameManager'
	import Label from './Label.svelte'
	import LabelsInput from './LabelsInput.svelte'
	import InheritedLabels from './InheritedLabels.svelte'
	import Badge from './common/badge/Badge.svelte'
	import {
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()
	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)

	interface Props {
		summary?: string
		path?: string
		labels?: string[] | undefined
		inheritedLabels?: string[] | undefined
		editable?: boolean
		onSaved?: (newPath: string) => void
		/** What is being renamed. An agent is a resource, so the path field is scoped to one and
		 *  the write goes through the agent branch of the rename manager. */
		kind?: 'flow' | 'script' | 'agent'
		/** Saves instead of the API, for an item that exists only locally (a draft).
		 * Returns an error message to keep the popover open. */
		saveOverride?: (next: {
			path: string
			summary: string
			labels: string[] | undefined
		}) => string | undefined
		/** Header variant: no path line (the host shows it) and a lighter summary. */
		compact?: boolean
		/** Rendered inside the trigger in place of the summary, for a host that hangs a second copy
		 *  of this editor off something else it draws — the band's path segment. */
		label?: Snippet
		/** The trigger's own classes, for a host whose row has a look of its own. */
		triggerClass?: string
		/** Which field takes the cursor on open: the click says which half of the name the user came
		 *  to change. `Path` is only reachable for an owner, so 'path' falls back to the summary. */
		focusField?: 'summary' | 'path'
	}

	let {
		summary = $bindable(''),
		path = $bindable(''),
		labels = $bindable(),
		inheritedLabels = undefined,
		editable = false,
		onSaved,
		kind = 'flow',
		saveOverride = undefined,
		compact = false,
		label,
		triggerClass = 'block min-w-0 max-w-full px-1 py-0.5 rounded text-left cursor-pointer hover:bg-surface-hover transition-colors',
		focusField = 'summary'
	}: Props = $props()

	// An agent lives at a resource path, so that is what the picker validates against; the
	// placeholder still says "agent", which is what the reader is naming.
	const pathKind = $derived(kind === 'agent' ? 'resource' : kind)

	let editSummary = $state('')
	let editPath = $state('')
	let dirtyPath = $state(false)
	let popoverOpen = $state(false)
	// Resolved before the popover opens, not on opening it: the path field only exists for an owner,
	// and `openFocus` looks for it in the same flush the popover renders in.
	const own = $derived(
		saveOverride !== undefined || isOwner(path ?? '', actingUser, $operatingWorkspace)
	)
	let onBehalfOfEmail = $state<string | undefined>(undefined)
	let summaryInput: ReturnType<typeof TextInput> | undefined = $state()
	let labelsDirty = $state(false)
	let hasChanges = $derived(editSummary !== (summary ?? '') || (own && dirtyPath) || labelsDirty)

	$effect(() => {
		if (popoverOpen && onSaved) {
			editSummary = summary ?? ''
			editPath = path ?? ''
			labelsDirty = false
			onBehalfOfEmail = undefined
			if (kind === 'flow' && $operatingWorkspace && path) {
				checkFlowOnBehalfOf($operatingWorkspace, path).then((email) => {
					onBehalfOfEmail = email
				})
			}
		}
	})

	/** The path's name field inside this popover. `Path` hardcodes `id="path"`, and melt resolves a
	 *  string `openFocus` with a document-wide `querySelector`, so a popover portalled to the body
	 *  would hand the keystrokes to whichever other `Path` the page happens to have mounted. */
	const pathField = () =>
		document.querySelector<HTMLInputElement>('[data-path-edit-path] #path') ?? null

	async function save(close: () => void) {
		const initialPath = path ?? ''
		const newPath = own ? editPath : initialPath
		if (saveOverride) {
			const error = saveOverride({ path: newPath, summary: editSummary, labels })
			if (error) {
				sendUserToast(error, true)
				return
			}
			labelsDirty = false
			close()
			onSaved?.(newPath)
			return
		}

		try {
			await updateItemPathAndSummary({
				workspace: $operatingWorkspace!,
				kind,
				initialPath,
				newPath,
				newSummary: editSummary,
				labels
			})
			sendUserToast(`${kind.charAt(0).toUpperCase()}${kind.slice(1)} updated`)
			labelsDirty = false
			close()
			onSaved?.(newPath)
		} catch (e: any) {
			sendUserToast(`Could not update ${kind}: ${e.body ?? e.message}`, true)
		}
	}
</script>

{#snippet editor()}
	<!-- Without naming the path's own field melt lands on the row's first button, so a popover
	     opened from the path would not be typing into the path. -->
	<Popover
		class={triggerClass}
		placement="bottom-start"
		contentClasses="p-4"
		triggerAttrs={{ title: 'Edit summary and path', 'aria-label': 'Edit summary and path' }}
		usePointerDownOutside
		excludeSelectors=".drawer"
		disableFocusTrap
		openFocus={focusField === 'path' && own
			? pathField
			: () => {
					summaryInput?.focus()
					return null
				}}
		bind:isOpen={popoverOpen}
	>
		{#snippet trigger()}
			<!-- Both of the popover's arms show the summary and the path, and ownership decides
					     whether the path can be changed, not whether it is there. -->
			{#if label}
				{@render label()}
			{:else}
				<span
					class="{compact
						? 'text-xs font-medium'
						: 'text-sm font-semibold'} block truncate {emptyString(summary)
						? 'text-tertiary italic font-normal'
						: 'text-emphasis'}"
				>
					{emptyString(summary) ? 'Add a summary...' : summary}
				</span>
			{/if}
		{/snippet}
		{#snippet content({ close })}
			<div class="flex flex-col gap-6 w-[480px]">
				{#if onSaved}
					<Label label="Summary">
						<TextInput
							bind:this={summaryInput}
							inputProps={{
								type: 'text',
								placeholder: 'Short summary',
								onkeydown: (e) => {
									if (e.key === 'Enter') {
										save(close)
									}
								}
							}}
							bind:value={editSummary}
						/>
					</Label>
					<div class="-mt-4 flex items-center gap-2">
						<LabelsInput
							bind:labels
							onchange={() => {
								labelsDirty = true
							}}
						/>
						{#if inheritedLabels?.length}
							<InheritedLabels labels={inheritedLabels} />
						{/if}
					</div>
					{#if inheritedLabels?.length}
						<p class="-mt-5 text-2xs text-tertiary">
							Gray labels are inherited from the folder and can only be edited there.
						</p>
					{/if}
					<Label label="Path">
						{#if own}
							<div data-path-edit-path>
								<Path
									autofocus={false}
									bind:path={editPath}
									bind:dirty={dirtyPath}
									initialPath={path ?? ''}
									namePlaceholder={kind}
									kind={pathKind}
									size="sm"
									drawerOffset={4000}
								/>
							</div>
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
						disabled={!hasChanges}
						title="Save summary and path"
						onclick={() => save(close)}
					>
						Save
					</Button>
				{:else}
					<label class="block text-primary">
						<div class="pb-1 text-xs font-semibold text-emphasis">Summary</div>
						<TextInput
							bind:this={summaryInput}
							inputProps={{
								type: 'text',
								placeholder: 'Short summary',
								onkeydown: (e) => {
									if (e.key === 'Enter') {
										close()
									}
								}
							}}
							bind:value={summary}
						/>
					</label>
					<div class="block text-primary">
						<div class="pb-1 text-xs font-semibold text-emphasis">Path</div>
						<div data-path-edit-path>
							<Path
								autofocus={false}
								bind:path
								bind:dirty={dirtyPath}
								initialPath={path ?? ''}
								namePlaceholder={kind}
								kind={pathKind}
								size="sm"
								drawerOffset={4000}
							/>
						</div>
					</div>
				{/if}
			</div>
		{/snippet}
	</Popover>
{/snippet}

{#if label && (editable || onSaved)}
	<!-- The host draws the row this hangs in — the band's path segment — so only the trigger comes
	     from here: no path line above it, and no second copy of the labels. -->
	{@render editor()}
{:else if editable || onSaved}
	<!-- The name itself opens the popover, the same way the editors do it. Only the name: the whole
	     block was the trigger once, which meant a path or a label you only wanted to read answered
	     a click. -->
	<div
		class="min-w-0 truncate flex {compact
			? 'items-center px-1 py-0.5'
			: 'flex-col items-start px-2 py-1'}"
	>
		{#if !compact}
			<span class="text-2xs leading-tight text-tertiary font-mono font-normal truncate max-w-full"
				>{path}</span
			>
		{/if}
		<div class="flex items-center gap-3 max-w-full min-w-0">
			{@render editor()}
			<!-- Outside the trigger: a label is read, not clicked through to a rename. -->
			{#if labels?.length}
				<div class="flex items-center gap-0.5">
					{#each labels as labelText}
						<Badge color="blue" verySmall class="px-1" title="Label: {labelText}">{labelText}</Badge
						>
					{/each}
				</div>
			{/if}
			<InheritedLabels labels={inheritedLabels} />
		</div>
	</div>
{:else if label}
	<!-- Nothing to open, but the host still drew this into a slot of its own: give back what it
	     handed over rather than this component's own block, which belongs in a row it owns. -->
	{@render label()}
{:else}
	<div class="min-w-0 truncate flex items-center {compact ? '' : 'flex-col px-2'}">
		{#if !emptyString(summary) && !compact}
			<span class="text-[10px] leading-tight text-tertiary font-mono truncate">{path}</span>
		{/if}
		<span
			class="{compact ? 'text-xs font-medium' : 'text-sm font-semibold'} text-emphasis truncate"
		>
			{emptyString(summary) ? (path ?? '') : summary}
		</span>
	</div>
{/if}
