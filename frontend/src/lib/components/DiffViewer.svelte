<script lang="ts">
	import { Button } from './common'
	import Modal from './common/modal/Modal.svelte'
	import { diffLines } from 'diff'
	import { FileText } from 'lucide-svelte'
	import type { WorkspaceItemDiff } from '$lib/gen'

	export let open = false
	export let item: WorkspaceItemDiff | undefined = undefined
	export let sourceData: any = undefined
	export let targetData: any = undefined

	$: sourceContent = getContent(sourceData, item?.kind)
	$: targetContent = getContent(targetData, item?.kind)
	$: diff = sourceContent && targetContent ? diffLines(sourceContent, targetContent) : []

	function getContent(data: any, kind: string | undefined): string {
		if (!data || !kind) return ''
		
		switch (kind) {
			case 'script':
				return data.content || ''
			case 'flow':
				return JSON.stringify(data.value, null, 2)
			case 'app':
				return JSON.stringify(data.value, null, 2)
			case 'resource':
				return JSON.stringify(data.value, null, 2)
			case 'variable':
				return data.is_secret ? '(secret value)' : data.value
			default:
				return JSON.stringify(data, null, 2)
		}
	}
</script>

<Modal bind:open title="Diff Viewer">
	<div class="flex flex-col h-full max-h-[80vh]">
		{#if item}
			<div class="flex items-center gap-2 px-4 py-2 border-b">
				<FileText class="w-4 h-4" />
				<span class="font-mono text-sm">{item.path}</span>
				<span class="text-xs text-gray-500">({item.kind})</span>
			</div>
			
			<div class="flex-1 overflow-auto p-4">
				{#if diff.length > 0}
					<pre class="text-xs font-mono">
						{#each diff as part}
							{#if part.added}
								<span class="bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200">+ {part.value}</span>
							{:else if part.removed}
								<span class="bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-200">- {part.value}</span>
							{:else}
								<span class="text-gray-600 dark:text-gray-400">  {part.value}</span>
							{/if}
						{/each}
					</pre>
				{:else if !sourceData && targetData}
					<div class="bg-green-100 dark:bg-green-900 p-4 rounded">
						<p class="text-green-800 dark:text-green-200">New item (only in target)</p>
						<pre class="mt-2 text-xs">{targetContent}</pre>
					</div>
				{:else if sourceData && !targetData}
					<div class="bg-red-100 dark:bg-red-900 p-4 rounded">
						<p class="text-red-800 dark:text-red-200">Deleted item (only in source)</p>
						<pre class="mt-2 text-xs">{sourceContent}</pre>
					</div>
				{:else}
					<div class="text-gray-500">No differences found</div>
				{/if}
			</div>
		{:else}
			<div class="p-4 text-gray-500">No item selected</div>
		{/if}
		
		<div class="p-4 border-t flex justify-end">
			<Button variant="default" on:click={() => open = false}>
				Close
			</Button>
		</div>
	</div>
</Modal>
