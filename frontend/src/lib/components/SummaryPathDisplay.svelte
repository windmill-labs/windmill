<script lang="ts">
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
	import PathEditPen from './PathEditPen.svelte'
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
		/** Header variant: no path line (the host shows it) and a lighter summary. */
		compact?: boolean
	}

	let {
		summary = $bindable(''),
		path = $bindable(''),
		labels = $bindable(),
		inheritedLabels = undefined,
		editable = false,
		onSaved,
		kind = 'flow',
		compact = false
	}: Props = $props()

	// An agent lives at a resource path, so that is what the picker validates against; the
	// placeholder still says "agent", which is what the reader is naming.
	const pathKind = $derived(kind === 'agent' ? 'resource' : kind)

	let editSummary = $state('')
	let editPath = $state('')
	let dirtyPath = $state(false)
	let popoverOpen = $state(false)
	let ownPath = $state<string | undefined>(undefined)
	// Derived: ownership answers about the operating workspace, whose user resolves asynchronously.
	const own = $derived(
		ownPath === undefined ? false : isOwner(ownPath, actingUser, $operatingWorkspace)
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
			ownPath = path ?? ''
			onBehalfOfEmail = undefined
			if (kind === 'flow' && $operatingWorkspace && path) {
				checkFlowOnBehalfOf($operatingWorkspace, path).then((email) => {
					onBehalfOfEmail = email
				})
			}
		}
	})

	async function save(close: () => void) {
		const initialPath = path ?? ''
		const newPath = own ? editPath : initialPath

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

{#if editable || onSaved}
	<!-- The name reads as text and the pen beside it opens the popover, the same way the editors
	     carry theirs. The whole block used to be the trigger, so a name you only wanted to read
	     answered a click — and the pen has to sit outside the popover to be the only thing that
	     does. -->
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
		<div class="group flex items-center gap-3 max-w-full min-w-0">
			<span
				class="{compact ? 'text-xs font-medium' : 'text-sm font-semibold'} truncate {emptyString(
					summary
				)
					? 'text-tertiary italic font-normal'
					: 'text-emphasis'}"
			>
				{emptyString(summary) ? 'Add a summary...' : summary}
			</span>
			{#if labels?.length}
				<div class="flex items-center gap-0.5">
					{#each labels as label}
						<Badge color="blue" verySmall class="px-1" title="Label: {label}">{label}</Badge>
					{/each}
				</div>
			{/if}
			<InheritedLabels labels={inheritedLabels} />
			<Popover
				class="shrink-0"
				placement="bottom-start"
				contentClasses="p-4"
				usePointerDownOutside
				excludeSelectors=".drawer"
				disableFocusTrap
				openFocus={() => {
					summaryInput?.focus()
					return null
				}}
				bind:isOpen={popoverOpen}
			>
				{#snippet trigger()}
					<!-- Both of the popover's arms show the summary and the path, and ownership — which
					     is only resolved once it opens — decides whether the path can be changed, not
					     whether it is there. -->
					<PathEditPen
						label="Edit summary and path"
						visibility={emptyString(summary) ? 'always' : 'hover'}
						open={popoverOpen}
					/>
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
						{/if}
					</div>
				{/snippet}
			</Popover>
		</div>
	</div>
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
