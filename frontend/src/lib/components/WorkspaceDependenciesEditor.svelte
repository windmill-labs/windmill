<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { WorkspaceDependenciesService, WorkspaceService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Alert from './common/alert/Alert.svelte'
	import DependencyWarning from './DependencyWarning.svelte'
	import type SimpleEditor from './SimpleEditor.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import Section from './Section.svelte'
	import { Loader2, Rocket, Code2, FolderOpen, AlertTriangle } from 'lucide-svelte'
		import Select from './select/Select.svelte'
	import RadioButton from './RadioButton.svelte'

	const dispatch = createEventDispatcher()

	// Helper function to get full filename
	function getFullFilename(language: string, name: string | null): string | null {
		const extension = getFileExtension(language)
		if (extension == null) return null;
		return name ? `${name}.${extension}` : extension
	}

	function getWorkspaceDependenciesPath(name: string | null, language: string): string | null {
		const extension = getFileExtension(language)
		if (extension == null) return null;
		return name ? `workspace_dependencies/${name}.${extension}` : `workspace_dependencies/${extension}`
	}

	function getFileExtension(language: string): string | null {
		switch (language) {
			case 'python3':
				return 'requirements.in'
			case 'typescript':
				return 'package.json'
			case 'go':
				return 'go.mod'
			case 'php':
				return 'composer.json'
			default:
				return null
		}
	}

	// Check if workspace default exists for a language
	function hasWorkspaceDefault(language: string): boolean {
		return existingWorkspaceDefaults[language] || false
	}

	function goToWorkspaceDefault(): void {
		// TODO: Navigate to existing workspace default
		console.log('Navigate to workspace default for', workspaceDependencies.language)
		drawer?.closeDrawer()
	}

	let workspaceDependenciesName: string = $state('')
	let workspaceDependenciesType: 'workspace' | 'named' = $state('workspace')
	
	// Language options for workspace dependencies - only supported languages
	const LANGUAGE_OPTIONS = [
		{ value: 'python3', label: 'Python' },
		{ value: 'typescript', label: 'TypeScript' },
		{ value: 'go', label: 'Go' },
		{ value: 'php', label: 'PHP' }
	]

	// Default templates for each language
	const LANGUAGE_TEMPLATES: Record<string, string> = {
		python3: `# Python Requirements (requirements.in format)
## py: 3.11
#^ Uncomment to pin to python version.
# Core dependencies
requests>=2.31.0
pandas>=2.0.0
numpy>=1.24.0

# Database connectivity (uncomment as needed)
# psycopg2-binary>=2.9.0  # PostgreSQL
# pymongo>=4.0.0          # MongoDB
# redis>=4.5.0            # Redis

# Web frameworks (uncomment as needed)  
# fastapi>=0.100.0        # FastAPI
# flask>=2.3.0            # Flask

# Data processing (uncomment as needed)
# pydantic>=2.0.0         # Data validation
# python-dotenv>=1.0.0    # Environment management
# structlog>=23.0.0       # Structured logging

# Development dependencies (use requirements-dev.in for these)
# pytest>=7.4.0
# black>=23.0.0
# mypy>=1.5.0`,

		typescript: `{
  "name": "windmill-typescript-script",
  "version": "1.0.0",
  "description": "TypeScript script dependencies for Windmill",
  "main": "index.ts",
  "scripts": {
    "build": "tsc",
    "dev": "ts-node index.ts",
    "test": "jest"
  },
  "dependencies": {
    "axios": "^1.6.0",
    "lodash": "^4.17.21",
    "date-fns": "^2.30.0",
    "uuid": "^9.0.0"
  },
  "devDependencies": {
    "@types/node": "^20.0.0",
    "@types/lodash": "^4.14.0",
    "@types/uuid": "^9.0.0",
    "typescript": "^5.0.0",
    "ts-node": "^10.9.0",
    "jest": "^29.0.0",
    "@types/jest": "^29.0.0",
    "ts-jest": "^29.0.0"
  },
  "engines": {
    "node": ">=18.0.0"
  }
}`,

		go: `module windmill-go-script

go 1.21

require (
	github.com/gorilla/mux v1.8.1
	github.com/lib/pq v1.10.9
	github.com/joho/godotenv v1.5.1
	github.com/sirupsen/logrus v1.9.3
	github.com/stretchr/testify v1.8.4
)

require (
	github.com/davecgh/go-spew v1.1.1 // indirect
	github.com/pmezard/go-difflib v1.0.0 // indirect
	golang.org/x/sys v0.0.0-20220715151400-c0bba94af5f8 // indirect
	gopkg.in/yaml.v3 v3.0.1 // indirect
)`,

		php: `{
  "name": "windmill-php-script",
  "description": "PHP script dependencies for Windmill",
  "type": "project",
  "require": {
    "php": ">=8.1",
    "guzzlehttp/guzzle": "^7.8",
    "monolog/monolog": "^3.5",
    "vlucas/phpdotenv": "^5.6",
    "symfony/console": "^6.4"
  },
  "require-dev": {
    "phpunit/phpunit": "^10.5",
    "phpstan/phpstan": "^1.10",
    "squizlabs/php_codesniffer": "^3.8"
  },
  "autoload": {
    "psr-4": {
      "App\\\\": "src/"
    }
  },
  "config": {
    "optimize-autoloader": true,
    "sort-packages": true
  },
  "scripts": {
    "test": "phpunit",
    "analyse": "phpstan analyse",
    "format": "phpcbf"
  }
}`
	}

	let workspaceDependencies: {
		content: string
		language: string
		description: string
	} = $state({
		content: '',
		language: 'python',
		description: ''
	})

	let valid = $state(true)
	let drawer: Drawer | undefined = $state()
	let edit = $state(false)
	let initialId: number | undefined = $state(undefined)
	let initialName: string | null = $state(null)
	let initialLanguage: string = $state('python')
	let existingWorkspaceDefaults: Record<string, boolean> = $state({})
	let can_write = $state(true)
	let editor: SimpleEditor | undefined = $state(undefined)
	let dependents: any[] = $state([])
	let showWarning = $state(false)
	let currentImportedPath: string | null = $state(null)

	const MAX_WORKSPACE_DEPENDENCIES_LENGTH = 50000

	$effect(() => {
		valid = workspaceDependencies.content.length <= MAX_WORKSPACE_DEPENDENCIES_LENGTH
	})

	// Track when editor is ready
	let editorReady = $state(false)
	
	// Calculate when deploy button should be disabled
	let isDisabled = $derived(!can_write || !valid || (workspaceDependenciesType === 'named' && workspaceDependenciesName.trim() === ''))
	
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
	$effect(() => {
		if (workspaceDependencies.language && LANGUAGE_TEMPLATES[workspaceDependencies.language] && !edit) {
			// Only update template for new workspace dependencies, not when editing existing ones
			workspaceDependencies.content = LANGUAGE_TEMPLATES[workspaceDependencies.language]
			if (editor && editorReady) {
				editor.setCode(workspaceDependencies.content)
			}
		}
	})

	export function initNew(): void {
		workspaceDependencies = {
			content: LANGUAGE_TEMPLATES.python3,
			language: 'python3',
			description: "",
		}
		edit = false
		initialId = undefined
		initialName = null
		initialLanguage = 'python3'
		workspaceDependenciesName = ''
		workspaceDependenciesType = 'named' // Start with named by default
		can_write = true
		editorReady = false
		// Check for existing workspace defaults
		existingWorkspaceDefaults = {
			python3: true, // Mock: workspace default exists for Python
			typescript: false,
			go: false,
			php: false
		}
		drawer?.openDrawer()
	}

	export async function editWorkspaceDependencies(id: number, name: string | null, language: string): Promise<void> {
		edit = true
		
		try {
			// TODO: Implement getWorkspaceDependencies endpoint and use actual API call
			// For now, fall back to template content since we don't have a get endpoint
			const mockContent = LANGUAGE_TEMPLATES[language] || LANGUAGE_TEMPLATES.python
			
			can_write = true // TODO: Implement proper permissions

			workspaceDependencies = {
				content: mockContent,
				language: language,
				description: `${name || 'Default'} requirements for ${language}`
			}
		} catch (error) {
			console.error('Error loading workspace dependencies:', error)
			// Fall back to template on error
			workspaceDependencies = {
				content: LANGUAGE_TEMPLATES[language] || LANGUAGE_TEMPLATES.python3,
				language: language,
				description: `${name || 'Default'} requirements for ${language}`
			}
			sendUserToast('Could not load workspace dependencies file content, using template', true)
		}
		
		initialId = id
		initialName = name
		initialLanguage = language
		workspaceDependenciesName = name || ''
		workspaceDependenciesType = name === null ? 'workspace' : 'named'
		editorReady = false
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
					workspace_id: $workspaceStore!
				}
			})

			const displayName = workspaceDependenciesType === 'workspace' ? `workspace default for ${workspaceDependencies.language}` : workspaceDependenciesName
			sendUserToast(`Deployed workspace dependencies: ${displayName}`)
			dispatch('create')
			drawer?.closeDrawer()
		} catch (error) {
			console.error('Error updating workspace dependencies:', error)
			sendUserToast(`Failed to update workspace dependencies: ${error.message}`, true)
		} finally {
			showWarning = false
		}
	}

	async function handleDeployClick(): Promise<void> {
		// For updates, always show warning to confirm deployment
		if (edit) {
			const existingPath = getWorkspaceDependenciesPath(initialName, initialLanguage)
			
			if (existingPath === null) {
				sendUserToast('Unsupported language for workspace dependencies path generation', true)
				return
			}

			// Always show warning for updates
			currentImportedPath = existingPath
			showWarning = true
		} else {
			// New workspace dependencies, no need to check for dependents
			await updateWorkspaceDependencies()
		}
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
		title={edit ? `Deploy ${getFullFilename(workspaceDependencies.language, workspaceDependenciesType === 'workspace' ? null : workspaceDependenciesName)}` : 'Add workspace dependencies'}
		on:close={drawer?.closeDrawer}
	>
		<div class="flex flex-col gap-8">
			{#if !can_write}
				<Alert type="warning" title="Only read access">
					You only have read access to this resource and cannot edit it
				</Alert>
			{/if}

			{#if showWarning && currentImportedPath}
				<DependencyWarning
					importedPath={currentImportedPath}
					title="Deployment Warning"
					confirmText="Deploy Anyway"
					cancelText="Cancel"
					onConfirm={confirmDeploy}
					onCancel={cancelDeploy}
				/>
			{/if}
			
			<Section label="Workspace Dependencies Type">
				<div class="flex flex-col gap-4">
					{#if hasWorkspaceDefault(workspaceDependencies.language) && !edit}
						<RadioButton
							bind:value={workspaceDependenciesType}
							options={[
								['Named Dependencies', 'named']
							]}
							disabled={!can_write}
						/>
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
						<RadioButton
							bind:value={workspaceDependenciesType}
							options={[
								['Named Dependencies', 'named'],
								['Workspace Default', 'workspace']
							]}
							disabled={!can_write || edit}
						/>
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
						Workspace default dependencies are used when no specific dependencies are specified in scripts.
						Named dependencies can be referenced by scripts using
						<code class="bg-surface-secondary px-1 rounded">#raw_reqs requirement_name</code> annotations.
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
						{edit ? 'Language cannot be changed after creation.' : 'Select the programming language these dependencies are for. This will load a default template.'}
					</div>
				</div>
			</Section>

			<Section label="Description">
				<input
					type="text"
					bind:value={workspaceDependencies.description}
					placeholder="Brief description of these workspace dependencies (optional)"
					disabled={!can_write}
					class="input"
				/>
				<div class="text-sm text-tertiary mt-2">
					Provide a brief description to help others understand the purpose of these workspace dependencies.
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
