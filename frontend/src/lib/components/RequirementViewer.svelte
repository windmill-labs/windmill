<script lang="ts">
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import HighlightCode from './HighlightCode.svelte'
	import { Button } from './common'
	import RequirementEditor from './RequirementEditor.svelte'
	import { Code2, Edit, FileText } from 'lucide-svelte'
	import { canWrite } from '$lib/utils'
	import { userStore } from '$lib/stores'

	// Component state
	let drawer: Drawer | undefined = $state()
	let requirementEditor: RequirementEditor | undefined = $state()
	
	// Content state
	let viewContent: string = $state('')
	let viewLanguage: string = $state('python')
	let viewPath: string = $state('')
	let viewDescription: string = $state('')
	let canWriteReq: boolean = $state(false)

	// Export methods for external control
	export function openViewer(path: string, content?: string, language?: string, description?: string) {
		viewPath = path
		viewContent = content || ''
		viewLanguage = language || 'python'
		viewDescription = description || ''
		
		// TODO: Replace with actual API call to check permissions
		// For now, mock the permission check
		canWriteReq = canWrite(path, {}, $userStore)
		
		drawer?.openDrawer()
	}

	export function closeViewer() {
		drawer?.closeDrawer()
	}

	function editRequirement() {
		requirementEditor?.editRequirement(viewPath)
		closeViewer()
	}

	function onRequirementUpdated() {
		// TODO: Reload requirement data
		console.log('Requirement updated, should reload data')
	}

	function getLanguageForHighlighting(lang: string): 'python3' | 'nativets' | 'go' | 'php' | undefined {
		// Map our requirement languages to syntax highlighting languages
		switch (lang) {
			case 'python':
				return 'python3'
			case 'typescript':
				return 'nativets'
			case 'go':
				return 'go'
			case 'php':
				return 'php'
			default:
				return undefined
		}
	}
</script>

<RequirementEditor bind:this={requirementEditor} on:create={onRequirementUpdated} />

<Drawer bind:this={drawer} size="900px">
	<DrawerContent title="Requirements from {viewPath}" on:close={closeViewer}>
		{#snippet actions()}
			<div class="flex items-center gap-4">
				<div class="flex items-center gap-2">
					<Code2 size={16} class="text-secondary" />
					<span class="text-sm font-mono text-secondary">{viewLanguage}</span>
				</div>
				{#if canWriteReq}
					<Button
						size="xs"
						variant="border"
						color="light"
						startIcon={{ icon: Edit }}
						on:click={editRequirement}
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
				<HighlightCode language={getLanguageForHighlighting(viewLanguage)} code={viewContent} />
			{:else}
				<div class="text-center text-secondary py-8">
					<FileText size={48} class="mx-auto mb-4 opacity-50" />
					<p>No requirements found for this path</p>
					<p class="text-xs mt-2">Create a requirement to define dependencies for scripts in this directory</p>
				</div>
			{/if}
		</div>
	</DrawerContent>
</Drawer>