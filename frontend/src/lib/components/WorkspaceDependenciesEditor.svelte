<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import {
		WorkspaceDependenciesService,
		type ScriptLang,
		type WorkspaceDependencies
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Alert from './common/alert/Alert.svelte'
	import DependenciesDeploymentWarning from './DependenciesDeploymentWarning.svelte'
	import { sendUserToast } from '$lib/toast'
	import Section from './Section.svelte'
	import { Loader2, Rocket, Code2, FolderOpen } from 'lucide-svelte'
	import Select from './select/Select.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'

	const dispatch = createEventDispatcher()

	// Helper function to get full filename
	export function getFullFilename(language: ScriptLang, name: string | null): string | null {
		const extension = getFileExtension(language)
		if (extension == null) return null
		return name ? `${name}.${extension}` : extension
	}

	export function getWorkspaceDependenciesPath(
		name: string | null,
		language: ScriptLang
	): string | null {
		const extension = getFileExtension(language)
		if (extension == null) return null
		return name ? `dependencies/${name}.${extension}` : `dependencies/${extension}`
	}

	export function getDisplayName(deps: WorkspaceDependencies): string {
		return deps.name || `Default (${deps.language})`
	}

	export function getFileExtension(language: ScriptLang): string | null {
		switch (language) {
			case 'python3':
				return 'requirements.in'
			case 'bun':
				return 'package.json'
			// case 'go':
			// 	return 'go.mod'
			case 'php':
				return 'composer.json'
			default:
				return null
		}
	}

	// Load existing workspace defaults from API
	async function loadExistingWorkspaceDefaults(): Promise<void> {
		if (!$workspaceStore) return

		try {
			const workspaceDeps = await WorkspaceDependenciesService.listWorkspaceDependencies({
				workspace: $workspaceStore
			})

			// Reset defaults
			existingWorkspaceDefaults = {}
			workspaceDefaultIds = {}

			// Check for workspace defaults (where name is null) for each language
			workspaceDeps.forEach((dep) => {
				if (dep.name === null) {
					existingWorkspaceDefaults[dep.language] = true
					workspaceDefaultIds[dep.language] = dep.id
				}
			})
		} catch (error) {
			console.error('Error loading existing workspace defaults:', error)
			// Fallback to empty defaults on error
			existingWorkspaceDefaults = {}
			workspaceDefaultIds = {}
		}
	}

	// Check if workspace default exists for a language
	function hasWorkspaceDefault(language: string): boolean {
		return existingWorkspaceDefaults[language] || false
	}

	function goToWorkspaceDefault(): void {
		const language = workspaceDependencies.language
		const defaultId = workspaceDefaultIds[language]

		if (defaultId) {
			// Close current drawer and edit the workspace default
			drawer?.closeDrawer()
			// Call editWorkspaceDependencies with the workspace default info (name = null for workspace default)
			setTimeout(() => {
				editWorkspaceDependencies(defaultId, undefined, language)
			}, 100) // Small delay to allow drawer to close before opening new one
		} else {
			console.error('No workspace default found for language:', language)
			sendUserToast(`No workspace default found for ${language}`, true)
		}
	}

	let workspaceDependenciesName: string = $state('')
	let workspaceDependenciesType = $state<'workspace' | 'named'>('workspace')

	// Language options for workspace dependencies - only supported languages
	const LANGUAGE_OPTIONS = [
		{ value: 'python3', label: 'Python' },
		{ value: 'bun', label: 'TypeScript (Bun/Bunnative)' },
		// { value: 'go', label: 'Go' },
		{ value: 'php', label: 'PHP' }
	]

	// Default templates for each language
	const LANGUAGE_TEMPLATES: Record<string, string> = {
		python3: `# Python Requirements (requirements.in format)
# # py: 3.11
# ^ Uncomment to pin to python version.
# Core dependencies
requests>=2.31.0
pandas>=2.0.0
numpy>=1.24.0
`,

		bun: `{
	"dependencies": {
	    "number-to-words": "1",
	    "windmill-client": "*",
	    "date-fns": "^2.30.0",
	    "uuid": "^9.0.0"
	}
}`,

		// 		go: `module mymod

		// go 1.26

		// require (
		// 	rsc.io/quote v1.5.2
		// 	github.com/gorilla/mux v1.8.1
		// 	github.com/lib/pq v1.10.9
		// 	github.com/joho/godotenv v1.5.1
		// 	github.com/sirupsen/logrus v1.9.3
		// 	github.com/stretchr/testify v1.8.4
		// )
		// `,

		php: `{
  "require": {
    "guzzlehttp/guzzle": "^7.8",
    "monolog/monolog": "^3.5",
    "vlucas/phpdotenv": "^5.6",
    "symfony/console": "^6.4"
  }
}`
	}

	let workspaceDependencies: {
		content: string
		language: ScriptLang
		description: string
	} = $state({
		content: '',
		language: 'python3',
		description: ''
	})

	let valid = $state(true)
	let drawer: Drawer | undefined = $state()
	let edit = $state(false)
	let initialName: string | undefined = $state(undefined)
	let initialLanguage: ScriptLang = $state('python3')
	let existingWorkspaceDefaults: Record<string, boolean> = $state({})
	let workspaceDefaultIds: Record<string, number> = $state({})
	let can_write = $state(true)
	let editor: any | undefined = $state(undefined)
	let showWarning = $state(false)
	let currentImportedPath: string | null = $state(null)

	const MAX_WORKSPACE_DEPENDENCIES_LENGTH = 50000

	$effect(() => {
		valid = workspaceDependencies.content.length <= MAX_WORKSPACE_DEPENDENCIES_LENGTH
	})

	// Track when editor is ready
	let editorReady = $state(false)

	// Calculate when deploy button should be disabled
	let isDisabled = $derived(
		!can_write ||
			!valid ||
			(workspaceDependenciesType === 'named' && workspaceDependenciesName.trim() === '')
	)

	$effect(() => {
		if (editor && !editorReady) {
			editor.setCode(workspaceDependencies.content)
			editorReady = true
		}
	})

	// Handle editor content changes
	function handleEditorChange(event: { code: string }) {
		// SimpleEditor dispatches change event with { code: string }
		workspaceDependencies.content = event.code
	}

	// Watch for language changes and update template
	let initialLanguageSet = $state(false)
	let previousLanguage = $state<ScriptLang | null>(null)

	$effect(() => {
		const lang = workspaceDependencies.language

		// Only update template when language actually changes, not on every content edit
		if (lang && LANGUAGE_TEMPLATES[lang] && !edit && previousLanguage !== lang) {
			workspaceDependencies.content = LANGUAGE_TEMPLATES[lang]
			previousLanguage = lang

			if (!initialLanguageSet) {
				initialLanguageSet = true
			}

			if (editor && editorReady) {
				editor.setCode(workspaceDependencies.content)
			}
		}
	})

	export async function initNew(): Promise<void> {
		workspaceDependencies = {
			content: LANGUAGE_TEMPLATES.python3,
			language: 'python3',
			description: ''
		}
		edit = false
		initialName = undefined
		initialLanguage = 'python3'
		workspaceDependenciesName = ''
		workspaceDependenciesType = 'named' // Start with named by default
		can_write = true
		editorReady = false
		initialLanguageSet = false
		previousLanguage = null

		// Load existing workspace defaults from API
		await loadExistingWorkspaceDefaults()

		drawer?.openDrawer()
	}

	export async function editWorkspaceDependencies(
		id: number,
		name: string | undefined,
		language: ScriptLang
	): Promise<void> {
		edit = true

		try {
			// Call the get-latest endpoint to get actual content
			const workspaceDeps = await WorkspaceDependenciesService.getLatestWorkspaceDependencies({
				workspace: $workspaceStore!,
				language,
				name: name || undefined
			})

			can_write = true // TODO: Implement proper permissions

			if (workspaceDeps) {
				workspaceDependencies = {
					content: workspaceDeps.content,
					language: workspaceDeps.language,
					description:
						workspaceDeps.description || `${name || 'Default'} requirements for ${language}`
				}
			} else {
				sendUserToast('Enforced dependencies not found', true)
				return
			}
		} catch (error) {
			console.error('Error loading workspace dependencies:', error)
			sendUserToast(`Failed to load enforced dependencies: ${error.message}`, true)
			return
		}

		initialName = name
		initialLanguage = language
		workspaceDependenciesName = name || ''
		workspaceDependenciesType = !name ? 'workspace' : 'named'
		editorReady = false
		initialLanguageSet = true // Don't override content when editing
		previousLanguage = language // Set to current language to prevent template reset

		drawer?.openDrawer()
	}

	async function updateWorkspaceDependencies(): Promise<void> {
		try {
			await WorkspaceDependenciesService.createWorkspaceDependencies({
				workspace: $workspaceStore!,
				requestBody: {
					name: workspaceDependenciesType === 'workspace' ? undefined : workspaceDependenciesName,
					content: workspaceDependencies.content,
					language: workspaceDependencies.language as any,
					workspace_id: $workspaceStore!,
					description: workspaceDependencies.description
				}
			})

			const displayName =
				workspaceDependenciesType === 'workspace'
					? `workspace default for ${workspaceDependencies.language}`
					: workspaceDependenciesName
			sendUserToast(`Deployed enforced dependencies: ${displayName}`)
			dispatch('create')
			drawer?.closeDrawer()
		} catch (error) {
			console.error('Error updating workspace dependencies:', error)
			sendUserToast(`Failed to update enforced dependencies: ${error.message}`, true)
		} finally {
			showWarning = false
		}
	}

	async function handleDeployClick(): Promise<void> {
		const isUnnamed = workspaceDependenciesType === 'workspace'
		const path = getWorkspaceDependenciesPath(
			isUnnamed ? null : workspaceDependenciesName || initialName || null,
			edit ? initialLanguage : workspaceDependencies.language
		)

		if (path === null) {
			sendUserToast('Unsupported language for enforced dependencies path generation', true)
			return
		}

		// Always show warning - for all types and operations
		currentImportedPath = path
		showWarning = true
	}

	function confirmDeploy(): void {
		showWarning = false
		updateWorkspaceDependencies()
	}

	function cancelDeploy(): void {
		showWarning = false
	}
</script>

<Drawer bind:this={drawer} size="1200px">
	<DrawerContent
		title={edit
			? `Deploy ${getFullFilename(workspaceDependencies.language, workspaceDependenciesType === 'workspace' ? null : workspaceDependenciesName)}`
			: 'Add enforced dependencies'}
		on:close={drawer?.closeDrawer}
	>
		<div class="flex flex-col gap-8">
			{#if !can_write}
				<Alert type="warning" title="Only read access">
					You only have read access to this resource and cannot edit it
				</Alert>
			{/if}

			{#if showWarning && currentImportedPath}
				<DependenciesDeploymentWarning
					importedPath={currentImportedPath}
					title={workspaceDependenciesType === 'workspace'
						? 'Redeploy impacted runnables?'
						: 'Deployment Warning'}
					confirmText={workspaceDependenciesType === 'workspace'
						? "I'm Sure - Deploy Default"
						: 'Deploy Anyway'}
					onConfirm={confirmDeploy}
					onCancel={cancelDeploy}
					isUnnamedDefault={workspaceDependenciesType === 'workspace'}
					language={LANGUAGE_OPTIONS.find((opt) => opt.value === workspaceDependencies.language)
						?.label || workspaceDependencies.language}
				/>
			{/if}

			<Section label="Enforced Dependencies Type">
				<div class="flex flex-col gap-4">
					{#if hasWorkspaceDefault(workspaceDependencies.language) && !edit}
						<ToggleButtonGroup bind:selected={workspaceDependenciesType} disabled={!can_write}>
							{#snippet children({ item })}
								<ToggleButton
									disabled={!can_write}
									value="named"
									label="Named Dependencies"
									{item}
								/>
							{/snippet}
						</ToggleButtonGroup>
						<div class="flex items-center gap-2 p-3 bg-blue-50 border border-blue-200 rounded">
							<FolderOpen size={16} class="text-blue-600" />
							<span class="text-sm text-blue-800 flex-1">
								Workspace default already exists for {workspaceDependencies.language}
							</span>
							<Button size="xs" color="blue" variant="border" on:click={goToWorkspaceDefault}>
								Go to default
							</Button>
						</div>
					{:else}
						<ToggleButtonGroup
							bind:selected={workspaceDependenciesType}
							disabled={!can_write || edit}
						>
							{#snippet children({ item })}
								<ToggleButton
									disabled={!can_write || edit}
									value="named"
									label="Named Dependencies"
									{item}
								/>
								<ToggleButton
									disabled={!can_write || edit}
									value="workspace"
									label="Workspace Default"
									{item}
								/>
							{/snippet}
						</ToggleButtonGroup>
					{/if}
					{#if workspaceDependenciesType === 'named'}
						<input
							type="text"
							bind:value={workspaceDependenciesName}
							placeholder="Enter dependencies name (e.g., 'data-science', 'web-api')"
							disabled={!can_write}
							class="input"
						/>
					{/if}
					<div class="text-sm text-tertiary">
						<FolderOpen size={16} class="inline mr-2" />
						Default Enforced Dependencies are used when no specific Dependencies are referenced from
						runnables. Named dependencies can be referenced by scripts using
						<a
							href="https://www.windmill.dev/docs/core_concepts/workspace_dependencies"
							target="_blank"
							rel="noopener noreferrer"
							class="text-accent"
						>
							annotations
						</a>.
					</div>
				</div>
			</Section>

			<Section label="Language">
				<div class="flex flex-col gap-4">
					<Select
						bind:value={workspaceDependencies.language}
						items={LANGUAGE_OPTIONS}
						disabled={edit}
					/>
					<div class="text-sm text-tertiary">
						<Code2 size={16} class="inline mr-2" />
						{edit
							? 'Language cannot be changed after creation.'
							: 'Select the programming language these dependencies are for. This will load a default template.'}
					</div>
				</div>
			</Section>

			<Section label="Description">
				<input
					type="text"
					bind:value={workspaceDependencies.description}
					placeholder="Brief description of these enforced dependencies (optional)"
					disabled={!can_write}
					class="input"
				/>
				<div class="text-sm text-tertiary mt-2">
					Provide a brief description to help others understand the purpose of these enforced
					dependencies.
				</div>
			</Section>

			<Section label="Dependencies File Content">
				{#snippet header()}
					<span class="text-sm text-tertiary mr-4 font-normal">
						({workspaceDependencies.content.length}/{MAX_WORKSPACE_DEPENDENCIES_LENGTH} characters)
					</span>
				{/snippet}
				<div class="border rounded mb-4 w-full">
					{#await import('$lib/components/SimpleEditor.svelte')}
						<Loader2 class="animate-spin" />
					{:then Module}
						<Module.default
							bind:this={editor}
							autoHeight
							lang="markdown"
							code={workspaceDependencies.content}
							on:change={(e) => handleEditorChange(e.detail)}
							fixedOverflowWidgets={false}
							disabled={!can_write}
						/>
					{/await}
				</div>
			</Section>
		</div>

		{#snippet actions()}
			<Button
				on:click={handleDeployClick}
				disabled={isDisabled}
				startIcon={{ icon: Rocket }}
				color="dark"
				size="sm"
			>
				Deploy
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
