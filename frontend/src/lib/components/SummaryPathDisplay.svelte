<script lang="ts">
	import { emptyString } from '$lib/utils'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Path from '$lib/components/Path.svelte'

	interface Props {
		summary?: string
		path?: string
		editable?: boolean
		onEdit?: (summary: string, path: string) => void
		kind?: 'flow' | 'script'
	}

	let {
		summary = $bindable(''),
		path = $bindable(''),
		editable = false,
		onEdit,
		kind = 'flow'
	}: Props = $props()

	let editSummary = $state('')
	let editPath = $state('')
	let dirtyPath = $state(false)
	let popoverOpen = $state(false)
	let hasChanges = $derived(editSummary !== (summary ?? '') || dirtyPath)

	$effect(() => {
		if (popoverOpen && onEdit) {
			editSummary = summary ?? ''
			editPath = path ?? ''
		}
	})
</script>

{#if editable || onEdit}
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
			<div class="flex flex-col gap-3 w-[480px]">
				{#if onEdit}
					<label class="block text-primary">
						<div class="pb-1 text-xs font-semibold text-emphasis">Summary</div>
						<TextInput
							inputProps={{
								type: 'text',
								placeholder: 'Short summary',
								onkeydown: (e) => {
									if (e.key === 'Enter') {
										onEdit(editSummary, editPath)
										close()
									}
								}
							}}
							bind:value={editSummary}
						/>
					</label>
					<div class="block text-primary">
						<div class="pb-1 text-xs font-semibold text-emphasis">Path</div>
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
					</div>
					<Button
						size="xs"
						variant="accent"
						disabled={!hasChanges}
						title="Save summary and path"
						onclick={() => {
							onEdit?.(editSummary, editPath)
							close()
						}}
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
	<div class="min-w-24 truncate flex flex-col">
		{#if !emptyString(summary)}
			<span class="text-[10px] leading-tight text-tertiary font-mono truncate">{path}</span>
		{/if}
		<span class="text-sm font-semibold text-emphasis truncate">
			{emptyString(summary) ? (path ?? '') : summary}
		</span>
	</div>
{/if}
