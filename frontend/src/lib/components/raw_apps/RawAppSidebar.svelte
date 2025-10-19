<script lang="ts">
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import type { Runnable } from '../apps/inputType'
	import RawAppInlineScriptPanelList from './RawAppInlineScriptPanelList.svelte'
	import FileTreeNode from './FileTreeNode.svelte'
	import { buildFileTree } from './fileTreeUtils'
	import { Plus, File, Folder } from 'lucide-svelte'

	interface Props {
		runnables: Record<string, Runnable>
		selectedRunnable: string | undefined
		files: Record<string, string> | undefined
		modules?: Record<string, boolean>
	}

	let { runnables, selectedRunnable = $bindable(), files = $bindable(), modules }: Props = $props()

	const fileTree = $derived(buildFileTree(Object.keys(files ?? {})))

	let selectedPath = $state<string | undefined>(undefined)

	function handleFileClick(path: string) {
		console.log('File clicked:', path)
		selectedPath = path
		// TODO: Implement file selection logic
	}

	function handleAddFile(folderPath: string) {
		console.log('Add file to:', folderPath)
		// TODO: Implement add file logic
	}

	function handleAddFolder(folderPath: string) {
		console.log('Add folder to:', folderPath)
		// TODO: Implement add folder logic
	}

	function handleAddRootFile() {
		console.log('Add file to root')
		// TODO: Implement add file to root logic
	}

	function handleAddRootFolder() {
		console.log('Add folder to root')
		// TODO: Implement add folder to root logic
	}
</script>

<PanelSection size="lg" fullHeight={false} title="Frontend" id="app-editor-frontend-panel">
	{#snippet action()}
		<div class="flex gap-0.5">
			<button
				onclick={handleAddRootFile}
				class="p-0.5 hover:bg-surface-hover rounded transition-colors flex items-center gap-0.5"
				title="Add file to root"
			>
				<Plus size={12} class="text-secondary" />
				<File size={12} class="text-tertiary" />
			</button>
			<button
				onclick={handleAddRootFolder}
				class="p-0.5 hover:bg-surface-hover rounded transition-colors flex items-center gap-0.5"
				title="Add folder to root"
			>
				<Plus size={12} class="text-secondary" />
				<Folder size={12} class="text-tertiary" />
			</button>
		</div>
	{/snippet}

	<div class="flex flex-col gap-0.5 w-full">
		{#each fileTree as node (node.path)}
			<FileTreeNode
				{node}
				onFileClick={handleFileClick}
				onAddFile={handleAddFile}
				onAddFolder={handleAddFolder}
				{selectedPath}
			/>
		{/each}
		<FileTreeNode
			node={{
				name: 'wmill.ts',
				path: '/wmill.ts',
				isFolder: false
			}}
			onFileClick={handleFileClick}
			onAddFile={handleAddFile}
			onAddFolder={handleAddFolder}
			{selectedPath}
		/>
	</div>
</PanelSection>

<PanelSection
	size="md"
	fullHeight={false}
	title="Installed modules"
	id="app-editor-frontend-panel-modules"
>
	<div class="mt-2 flex flex-col gap-1">
		{#each Object.keys(modules ?? {}) as mod}
			<div class="text-xs px-2 text-secondary font-mono">{mod}</div>
		{/each}
	</div>
</PanelSection>

<RawAppInlineScriptPanelList bind:selectedRunnable {runnables} />
