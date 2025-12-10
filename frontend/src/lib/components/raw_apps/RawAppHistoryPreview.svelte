<script lang="ts">
	import type { HistoryEntry } from './RawAppHistoryManager.svelte'
	import { displayDate } from '$lib/utils'
	import { Eye, AlertTriangle, RotateCcw } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import FileTreeNode from './FileTreeNode.svelte'
	import { buildFileTree } from './fileTreeUtils'
	import RawAppInlineScriptPanelList from './RawAppInlineScriptPanelList.svelte'

	interface Props {
		entry: HistoryEntry | undefined
		onRestore: () => void
	}

	let { entry, onRestore }: Props = $props()

	const fileTree = $derived(entry ? buildFileTree(Object.keys(entry.files)) : [])
	const runnableCount = $derived(entry ? Object.keys(entry.runnables).length : 0)
	const fileCount = $derived(entry ? Object.keys(entry.files).length : 0)
</script>

{#if entry}
	<!-- Preview Mode Banner -->
	<div
		class="bg-amber-50 dark:bg-amber-900/20 border-b border-amber-200 dark:border-amber-700 px-4 py-3"
	>
		<div class="flex items-center justify-between gap-4">
			<div class="flex items-center gap-2 text-amber-800 dark:text-amber-300">
				<Eye size={16} class="flex-shrink-0" />
				<div class="flex flex-col gap-0.5">
					<span class="font-medium text-sm">Preview Mode - Read Only</span>
					<span class="text-xs opacity-80">
						Viewing snapshot from {displayDate(entry.timestamp.toISOString())}
					</span>
				</div>
			</div>
			<Button size="xs" color="dark" startIcon={{ icon: RotateCcw }} on:click={onRestore}>
				Restore This Version
			</Button>
		</div>
	</div>

	<!-- Warning Message -->
	<div class="p-4 bg-blue-50 dark:bg-blue-900/20 border-b border-blue-200 dark:border-blue-700">
		<div class="flex gap-2 text-sm text-blue-800 dark:text-blue-300">
			<AlertTriangle size={16} class="flex-shrink-0 mt-0.5" />
			<div>
				<div class="font-medium">Restoring will save your current state first</div>
				<div class="text-xs opacity-80 mt-1">
					Your current work will be automatically saved as a snapshot before restoring this
					version.
				</div>
			</div>
		</div>
	</div>

	<!-- Summary Info -->
	<div class="p-4 border-b border-gray-200 dark:border-gray-700">
		<div class="text-sm">
			<div class="font-medium text-primary mb-2">Snapshot Summary</div>
			<div class="text-secondary space-y-1">
				<div class="flex justify-between">
					<span>Summary:</span>
					<span class="font-medium text-primary">{entry.summary || 'Untitled App'}</span>
				</div>
				<div class="flex justify-between">
					<span>Files:</span>
					<span class="font-medium text-primary">{fileCount}</span>
				</div>
				<div class="flex justify-between">
					<span>Runnables:</span>
					<span class="font-medium text-primary">{runnableCount}</span>
				</div>
			</div>
		</div>
	</div>

	<!-- Content Preview -->
	<div class="flex-1 overflow-y-auto">
		<!-- Files Section -->
		<div class="border-b border-gray-200 dark:border-gray-700">
			<div class="px-4 py-2 bg-surface-secondary">
				<div class="text-xs font-semibold text-secondary uppercase tracking-wide">Files</div>
			</div>
			<div class="p-2">
				{#if fileTree.length > 0}
					{#each fileTree as node}
						<FileTreeNode
							{node}
							noEdit={true}
							level={0}
							selectedPath={undefined}
							onSelectFile={() => {}}
							onAddFile={() => {}}
							onAddFolder={() => {}}
							onDelete={() => {}}
							onRename={() => {}}
							pathToRename={undefined}
							pathToExpand={undefined}
						/>
					{/each}
				{:else}
					<div class="text-secondary text-sm p-2">No files</div>
				{/if}
			</div>
		</div>

		<!-- Runnables Section -->
		<div>
			<div class="px-4 py-2 bg-surface-secondary">
				<div class="text-xs font-semibold text-secondary uppercase tracking-wide">Runnables</div>
			</div>
			<div class="p-2">
				{#if runnableCount > 0}
					<RawAppInlineScriptPanelList
						runnables={entry.runnables}
						selectedRunnable={undefined}
						readonly={true}
					/>
				{:else}
					<div class="text-secondary text-sm p-2">No runnables</div>
				{/if}
			</div>
		</div>
	</div>
{:else}
	<div class="flex items-center justify-center h-full text-secondary p-8 text-center">
		<div>
			<Eye size={48} class="mx-auto mb-4 opacity-50" />
			<div class="text-sm">Select a snapshot from the list to preview its contents</div>
		</div>
	</div>
{/if}
