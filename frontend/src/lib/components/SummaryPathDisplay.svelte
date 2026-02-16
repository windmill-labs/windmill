<script lang="ts">
	import { emptyString } from '$lib/utils'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		summary?: string
		path?: string
		onEdit?: (summary: string, path: string) => void
		kind?: 'flow' | 'script'
	}

	let { summary, path, onEdit, kind = 'flow' }: Props = $props()

	let editSummary = $state('')
	let editPath = $state('')
	let dirtyPath = $state(false)
	let popoverOpen = $state(false)
	let hasChanges = $derived(editSummary !== (summary ?? '') || dirtyPath)

	$effect(() => {
		if (popoverOpen) {
			editSummary = summary ?? ''
			editPath = path ?? ''
		}
	})
</script>

{#if onEdit}
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
				class={twMerge(
					'min-w-24 truncate flex items-baseline gap-3 px-4 py-1 rounded-md border transition-colors cursor-pointer',
					popoverOpen
						? '!border-border-selected bg-surface-input'
						: '!border-transparent hover:!border-border-selected/50 hover:bg-surface-input'
				)}
			>
				<span
					class="text-base font-semibold truncate {emptyString(summary)
						? 'text-tertiary italic'
						: 'text-emphasis'}"
				>
					{emptyString(summary) ? 'Add a summary...' : summary}
				</span>
				<span class="text-2xs text-secondary font-mono font-normal truncate shrink-0">{path}</span>
			</div>
		{/snippet}
		{#snippet content({ close })}
			<div class="flex flex-col gap-3 w-[480px]">
				<label class="block text-primary">
					<div class="pb-1 text-xs font-semibold text-emphasis">Summary</div>
					<TextInput
						inputProps={{
							type: 'text',
							placeholder: 'Short summary',
							onkeydown: (e) => {
								if (e.key === 'Enter') {
									onEdit?.(editSummary, editPath)
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
			</div>
		{/snippet}
	</Popover>
{:else}
	<div class="min-w-24 text-emphasis truncate flex items-baseline gap-2">
		<span class="text-sm font-semibold text-emphasis truncate">
			{emptyString(summary) ? (path ?? '') : summary}
		</span>
		{#if !emptyString(summary)}
			<span class="text-2xs text-secondary font-mono truncate shrink-0">{path}</span>
		{/if}
	</div>
{/if}
