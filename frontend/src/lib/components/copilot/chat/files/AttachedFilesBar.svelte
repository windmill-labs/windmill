<script lang="ts">
	import { getAiChatManager } from '../aiChatManagerContext'
	import AttachedFileChip from './AttachedFileChip.svelte'
	import AttachedFolderChip from './AttachedFolderChip.svelte'
	import type { AttachedFile } from './attachedFiles.svelte'

	const aiChatManager = getAiChatManager()
	const files = $derived(aiChatManager.attachedFiles.list())

	// Files attached on their own get individual chips; files that came from a folder
	// collapse into a single chip per folder (preserving first-seen order).
	const standalone = $derived(files.filter((f) => !f.folder))
	const folders = $derived.by(() => {
		const map = new Map<string, AttachedFile[]>()
		for (const f of files) {
			if (!f.folder) continue
			const group = map.get(f.folder)
			if (group) group.push(f)
			else map.set(f.folder, [f])
		}
		return [...map.entries()].map(([name, groupFiles]) => ({ name, files: groupFiles }))
	})
</script>

{#if files.length > 0}
	<div class="flex flex-row flex-wrap items-center gap-1 mb-1" role="list">
		{#each folders as group (group.name)}
			<AttachedFolderChip
				folder={group.name}
				files={group.files}
				onRemove={() => aiChatManager.attachedFiles.removeFolder(group.name)}
			/>
		{/each}
		{#each standalone as file (file.name)}
			<AttachedFileChip {file} onRemove={() => aiChatManager.attachedFiles.removeFile(file.name)} />
		{/each}
	</div>
{/if}
