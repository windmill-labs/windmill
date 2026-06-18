<script lang="ts">
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import HighlightCode from './HighlightCode.svelte'
	import { Button } from './common'
	import WorkspaceDependenciesEditor from './WorkspaceDependenciesEditor.svelte'
	import type { ScriptLang } from '$lib/gen'
	import { Code2, Edit, FileText } from 'lucide-svelte'
	import { canWrite } from '$lib/utils'
	import { userStore } from '$lib/stores'

	// Component state
	let drawer: Drawer | undefined = $state()
	let workspaceDependenciesEditor: WorkspaceDependenciesEditor | undefined = $state()

	// Content state
	let viewContent: string = $state('')
	let viewLanguage: ScriptLang = $state('python3')
	let viewPath: string = $state('')
	let viewDescription: string = $state('')
	let viewDepsId: number | undefined = $state(undefined)
	let viewDepsName: string | undefined = $state(undefined)
	let canWriteDeps: boolean = $state(false)

	// Export methods for external control
	export function openViewer(path: string, content?: string, language?: ScriptLang, description?: string, depsId?: number, depsName?: string) {
		viewPath = path
		viewContent = content || ''
		viewLanguage = language || 'python3'
		viewDescription = description || ''
		viewDepsId = depsId
		viewDepsName = depsName

		// TODO: Replace with actual API call to check permissions
		// For now, mock the permission check
		canWriteDeps = canWrite(path, {}, $userStore)

		drawer?.openDrawer()
	}

	export function closeViewer() {
		drawer?.closeDrawer()
	}

	function editWorkspaceDependencies() {
		if (viewDepsId) {
			workspaceDependenciesEditor?.editWorkspaceDependencies(viewDepsId, viewDepsName, viewLanguage)
		}
		closeViewer()
	}

	function onWorkspaceDependenciesUpdated() {
		// TODO: Reload workspace dependencies data
		console.log('Workspace dependencies updated, should reload data')
	}
</script>

<WorkspaceDependenciesEditor bind:this={workspaceDependenciesEditor} on:create={onWorkspaceDependenciesUpdated} />

<Drawer bind:this={drawer} size="900px">
	<DrawerContent title="Workspace Dependencies from {viewPath}" on:close={closeViewer}>
		{#snippet actions()}
			<div class="flex items-center gap-4">
				<div class="flex items-center gap-2">
					<Code2 size={16} class="text-secondary" />
					<span class="text-sm font-mono text-secondary">{viewLanguage}</span>
				</div>
				{#if canWriteDeps}
					<Button
						size="xs"
						variant="border"
						color="light"
						startIcon={{ icon: Edit }}
						on:click={editWorkspaceDependencies}
					>
						Edit
					</Button>
				{/if}
			</div>
		{/snippet}
		
		<div class="space-y-4">
			{#if viewDescription}
				<div class="bg-surface-secondary rounded-md p-3">
					<p class="text-sm text-secondary">{viewDescription}</p>
				</div>
			{/if}
			
			{#if viewContent}
				<HighlightCode language={viewLanguage} code={viewContent} />
			{:else}
				<div class="text-center text-secondary py-8">
					<FileText size={48} class="mx-auto mb-4 opacity-50" />
					<p>No workspace dependencies found for this path</p>
					<p class="text-xs mt-2">Create workspace dependencies to define dependencies for scripts in this directory</p>
				</div>
			{/if}
		</div>
	</DrawerContent>
</Drawer>