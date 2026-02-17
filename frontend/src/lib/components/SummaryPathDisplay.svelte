<script lang="ts">
	import { emptyString, isOwner } from '$lib/utils'
	import { Alert, Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { updateItemPathAndSummary, checkFlowOnBehalfOf } from './moveRenameManager'
	import Label from './Label.svelte'

	interface Props {
		summary?: string
		path?: string
		editable?: boolean
		onSaved?: (newPath: string) => void
		kind?: 'flow' | 'script'
	}

	let {
		summary = $bindable(''),
		path = $bindable(''),
		editable = false,
		onSaved,
		kind = 'flow'
	}: Props = $props()

	let editSummary = $state('')
	let editPath = $state('')
	let dirtyPath = $state(false)
	let popoverOpen = $state(false)
	let own = $state(false)
	let onBehalfOfEmail = $state<string | undefined>(undefined)
	let hasChanges = $derived(editSummary !== (summary ?? '') || (own && dirtyPath))

	$effect(() => {
		if (popoverOpen && onSaved) {
			editSummary = summary ?? ''
			editPath = path ?? ''
			own = isOwner(path ?? '', $userStore, $workspaceStore)
			onBehalfOfEmail = undefined
			if (kind === 'flow' && $workspaceStore && path) {
				checkFlowOnBehalfOf($workspaceStore, path).then((email) => {
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
				workspace: $workspaceStore!,
				kind,
				initialPath,
				newPath,
				newSummary: editSummary
			})
			sendUserToast(`${kind === 'flow' ? 'Flow' : 'Script'} updated`)
			close()
			onSaved?.(newPath)
		} catch (e: any) {
			sendUserToast(`Could not update ${kind}: ${e.body ?? e.message}`, true)
		}
	}
</script>

{#if editable || onSaved}
	<Popover
		placement="bottom-start"
		contentClasses="p-4"
		usePointerDownOutside
		excludeSelectors=".drawer"
		disableFocusTrap
		bind:isOpen={popoverOpen}
	>
		{#snippet trigger()}
			<div
				class={'min-w-24 truncate flex flex-col items-start px-2 py-1 rounded-md  transition-colors cursor-pointer hover:bg-surface-hover'}
			>
				<span class="text-2xs leading-tight text-tertiary font-mono font-normal truncate max-w-full"
					>{path}</span
				>
				<span
					class="text-sm font-semibold truncate max-w-full {emptyString(summary)
						? 'text-tertiary italic font-normal'
						: 'text-emphasis'}"
				>
					{emptyString(summary) ? 'Add a summary...' : summary}
				</span>
			</div>
		{/snippet}
		{#snippet content({ close })}
			<div class="flex flex-col gap-6 w-[480px]">
				{#if onSaved}
					<Label label="Summary">
						<TextInput
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
					<Label label="Path">
						{#if own}
							<Path
								autofocus={false}
								bind:path={editPath}
								bind:dirty={dirtyPath}
								initialPath={path ?? ''}
								namePlaceholder={kind}
								{kind}
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
							{kind}
							hideFullPath
							size="sm"
							drawerOffset={4000}
						/>
					</div>
				{/if}
			</div>
		{/snippet}
	</Popover>
{:else}
	<div class="min-w-24 truncate flex flex-col px-2">
		{#if !emptyString(summary)}
			<span class="text-[10px] leading-tight text-tertiary font-mono truncate">{path}</span>
		{/if}
		<span class="text-sm font-semibold text-emphasis truncate">
			{emptyString(summary) ? (path ?? '') : summary}
		</span>
	</div>
{/if}
