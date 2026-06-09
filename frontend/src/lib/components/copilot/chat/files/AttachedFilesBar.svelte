<script lang="ts">
	import { getAiChatManager } from '../aiChatManagerContext'
	import AttachedFileChip from './AttachedFileChip.svelte'

	const aiChatManager = getAiChatManager()
	const files = $derived(aiChatManager.attachedFiles.list())
</script>

{#if files.length > 0}
	<div class="flex flex-row flex-wrap items-center gap-1 mb-1" role="list">
		{#each files as file (file.name)}
			<AttachedFileChip {file} onRemove={() => aiChatManager.attachedFiles.removeFile(file.name)} />
		{/each}
	</div>
{/if}
