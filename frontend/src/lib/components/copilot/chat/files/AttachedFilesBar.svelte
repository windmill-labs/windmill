<script lang="ts">
	import { Lock } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { getAiChatManager } from '../aiChatManagerContext'
	import AttachedFileChip from './AttachedFileChip.svelte'
	import AttachedFolderChip from './AttachedFolderChip.svelte'
	import type { AttachedFile, AttachedFolder } from './attachedFiles.svelte'

	const aiChatManager = getAiChatManager()
	const lockedCount = $derived(aiChatManager.attachedFiles.lockedCount)

	const MAX_VISIBLE = 3

	type Card =
		| { kind: 'folder'; key: string; folder: AttachedFolder }
		| { kind: 'file'; key: string; file: AttachedFile }

	// One card per linked folder, one per standalone file.
	const cards = $derived.by<Card[]>(() => [
		...aiChatManager.attachedFiles.folders.map(
			(folder): Card => ({ kind: 'folder', key: 'd:' + folder.name, folder })
		),
		...aiChatManager.attachedFiles.standalone.map(
			(file): Card => ({ kind: 'file', key: 'f:' + file.name, file })
		)
	])

	const visible = $derived(cards.slice(0, MAX_VISIBLE))
	const overflow = $derived(cards.slice(MAX_VISIBLE))

	function removeCard(card: Card) {
		if (card.kind === 'folder') aiChatManager.attachedFiles.removeFolder(card.folder.name)
		else aiChatManager.attachedFiles.removeFile(card.file.name)
	}
</script>

{#snippet chip(card: Card)}
	{#if card.kind === 'folder'}
		<AttachedFolderChip folder={card.folder} onRemove={() => removeCard(card)} />
	{:else}
		<AttachedFileChip file={card.file} onRemove={() => removeCard(card)} />
	{/if}
{/snippet}

{#if cards.length > 0}
	<div class="flex flex-row flex-nowrap items-center gap-1 mt-1 mb-1 min-w-0" role="list">
		{#each visible as card (card.key)}
			{@render chip(card)}
		{/each}

		{#if overflow.length > 0}
			<Popover>
				{#snippet trigger()}
					<div
						class="shrink-0 border rounded-md px-1.5 py-0.5 flex flex-row items-center text-primary text-xs cursor-pointer hover:bg-surface-hover bg-surface"
						title={`${overflow.length} more`}
					>
						+{overflow.length}
					</div>
				{/snippet}
				{#snippet content()}
					<div
						class="flex flex-col gap-1 p-1 max-h-64 overflow-y-auto min-w-40"
						role="list"
						aria-label="More attached files"
					>
						{#each overflow as card (card.key)}
							{@render chip(card)}
						{/each}
					</div>
				{/snippet}
			</Popover>
		{/if}

		{#if lockedCount > 0}
			<button
				class="shrink-0 border rounded-md px-1.5 py-0.5 flex flex-row items-center gap-1 text-xs text-amber-600 dark:text-amber-400 border-amber-300 dark:border-amber-700 hover:bg-surface-hover"
				title="Re-grant access to linked files after a reload"
				onclick={() => aiChatManager.attachedFiles.regrantLocked()}
			>
				<Lock size={12} class="shrink-0" />
				Restore access ({lockedCount})
			</button>
		{/if}
	</div>
{/if}
