<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { RawRequirementsService, WorkspaceService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Alert from './common/alert/Alert.svelte'
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

	function getRequirementPath(name: string | null, language: string): string | null {
		const extension = getFileExtension(language)
		if (extension == null) return null;
		return name ? `raw_requirements/${name}.${extension}` : `raw_requirements/${extension}`
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
		console.log('Navigate to workspace default for', requirement.language)
		drawer?.closeDrawer()
	}

	let requirementName: string = $state('')
	let requirementType: 'workspace' | 'named' = $state('workspace')
	
	// Language options for requirements - only supported languages
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

	let requirement: {
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

	const MAX_REQUIREMENT_LENGTH = 50000

	$effect(() => {
		valid = requirement.content.length <= MAX_REQUIREMENT_LENGTH
	})

	// Track when editor is ready
	let editorReady = $state(false)
	
	// Calculate when deploy button should be disabled
	let isDisabled = $derived(!can_write || !valid || (requirementType === 'named' && requirementName.trim() === ''))
	
	$effect(() => {
		if (editor && !editorReady) {
			editor.setCode(requirement.content)
			editorReady = true
		}
	})

	// Handle editor content changes
	function handleEditorChange(event: { code: string }) {
		// SimpleEditor dispatches change event with { code: string }
		requirement.content = event.code
	}

	// Watch for language changes and update template
	$effect(() => {
		if (requirement.language && LANGUAGE_TEMPLATES[requirement.language] && !edit) {
			// Only update template for new requirements, not when editing existing ones
			requirement.content = LANGUAGE_TEMPLATES[requirement.language]
			if (editor && editorReady) {
				editor.setCode(requirement.content)
			}
		}
	})

	export function initNew(): void {
		requirement = {
			content: LANGUAGE_TEMPLATES.python3,
			language: 'python3',
			description: "",
		}
		edit = false
		initialId = undefined
		initialName = null
		initialLanguage = 'python3'
		requirementName = ''
		requirementType = 'named' // Start with named by default
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

	export async function editRequirement(id: number, name: string | null, language: string): Promise<void> {
		edit = true
		
		try {
			// TODO: Implement getRawRequirement endpoint and use actual API call
			// For now, fall back to template content since we don't have a get endpoint
			const mockContent = LANGUAGE_TEMPLATES[language] || LANGUAGE_TEMPLATES.python
			
			can_write = true // TODO: Implement proper permissions

			requirement = {
				content: mockContent,
				language: language,
				description: `${name || 'Default'} requirements for ${language}`
			}
		} catch (error) {
			console.error('Error loading requirement:', error)
			// Fall back to template on error
			requirement = {
				content: LANGUAGE_TEMPLATES[language] || LANGUAGE_TEMPLATES.python3,
				language: language,
				description: `${name || 'Default'} requirements for ${language}`
			}
			sendUserToast('Could not load requirement content, using template', true)
		}
		
		initialId = id
		initialName = name
		initialLanguage = language
		requirementName = name || ''
		requirementType = name === null ? 'workspace' : 'named'
		editorReady = false
		drawer?.openDrawer()
	}

	async function updateRequirement(): Promise<void> {
		try {
			await RawRequirementsService.createRawRequirements({
				workspace: $workspaceStore!,
				requestBody: {
					name: requirementType === 'workspace' ? undefined : requirementName,
					content: requirement.content,
					language: requirement.language as any,
					workspace_id: $workspaceStore!
				}
			})

			const displayName = requirementType === 'workspace' ? `workspace default for ${requirement.language}` : requirementName
			sendUserToast(`Deployed raw requirement: ${displayName}`)
			dispatch('create')
			drawer?.closeDrawer()
		} catch (error) {
			console.error('Error updating requirement:', error)
			sendUserToast(`Failed to update requirement: ${error.message}`, true)
		} finally {
			showWarning = false
		}
	}

	async function handleDeployClick(): Promise<void> {
		// For updates, check for dependents first
		try {
			const existingPath = getRequirementPath(initialName, initialLanguage)
			console.log(existingPath);
			// TODO(claude): handle existingPath null, it should toast that it is not expected.
			dependents = await WorkspaceService.getDependents({
				workspace: $workspaceStore!,
				importedPath: existingPath
			})

			if (dependents.length > 0) {
				showWarning = true
			} else {
				// No dependents, proceed directly
				await updateRequirement()
			}
		} catch (error) {
			console.error('Error fetching dependents:', error)
			// On error, proceed without warning
			await updateRequirement()
		}
	}

	function confirmDeploy(): void {
		showWarning = false
		updateRequirement()
	}

	function cancelDeploy(): void {
		showWarning = false
	}

</script>

<Drawer bind:this={drawer} size="1200px">
	<DrawerContent
		title={edit ? `Deploy ${getFullFilename(requirement.language, requirementType === 'workspace' ? null : requirementName)}` : 'Add a raw requirement'}
		on:close={drawer?.closeDrawer}
	>
		<div class="flex flex-col gap-8">
			{#if !can_write}
				<Alert type="warning" title="Only read access">
					You only have read access to this resource and cannot edit it
				</Alert>
			{/if}

			{#if showWarning}
				<Alert type="warning" title="Redeploy Warning" collapsible={true} isCollapsed={false}>
					{#snippet children()}
						<div class="space-y-3">
							<p>This deployment will trigger redeployment of the following dependent scripts, flows, and apps:</p>
							<div class="space-y-2 max-h-40 overflow-y-auto">
								{#each dependents as dependent}
									<div class="flex items-center gap-2 p-2 bg-yellow-50 dark:bg-yellow-900/20 rounded border text-sm">
										<AlertTriangle size={14} class="text-yellow-600 dark:text-yellow-400" />
										<span class="font-medium">{dependent.importer_kind}:</span>
										<span class="font-mono">{dependent.importer_path}</span>
										{#if dependent.importer_node_ids && dependent.importer_node_ids.length > 0}
											<span class="text-xs text-tertiary">
												({dependent.importer_node_ids.length} node{dependent.importer_node_ids.length !== 1 ? 's' : ''})
											</span>
										{/if}
									</div>
								{/each}
							</div>
							<div class="flex gap-2 pt-2">
								<Button size="sm" color="warning" on:click={confirmDeploy}>
									Deploy Anyway
								</Button>
								<Button size="sm" variant="border" on:click={cancelDeploy}>
									Cancel
								</Button>
							</div>
						</div>
					{/snippet}
				</Alert>
			{/if}
			
			<Section label="Requirement Type">
				<div class="flex flex-col gap-4">
					{#if hasWorkspaceDefault(requirement.language) && !edit}
						<RadioButton
							bind:value={requirementType}
							options={[
								['Named Requirement', 'named']
							]}
							disabled={!can_write}
						/>
						<div class="flex items-center gap-2 p-3 bg-blue-50 border border-blue-200 rounded">
							<FolderOpen size={16} class="text-blue-600" />
							<span class="text-sm text-blue-800 flex-1">
								Workspace default already exists for {requirement.language}
							</span>
							<Button size="xs" color="blue" variant="border" on:click={goToWorkspaceDefault}>
								Go to default
							</Button>
						</div>
					{:else}
						<RadioButton
							bind:value={requirementType}
							options={[
								['Named Requirement', 'named'],
								['Workspace Default', 'workspace']
							]}
							disabled={!can_write || edit}
						/>
					{/if}
					{#if requirementType === 'named'}
						<input
							type="text"
							bind:value={requirementName}
							placeholder="Enter requirement name (e.g., 'data-science', 'web-api')"
							disabled={!can_write}
							class="input"
						/>
					{/if}
					<div class="text-sm text-tertiary">
						<FolderOpen size={16} class="inline mr-2" />
						Workspace default requirements are used when no specific requirement is specified in scripts.
						Named requirements can be referenced by scripts using
						<code class="bg-surface-secondary px-1 rounded">#raw_reqs requirement_name</code> annotations.
					</div>
				</div>
			</Section>

			<Section label="Language">
				<div class="flex flex-col gap-4">
					<Select
						bind:value={requirement.language}
						items={LANGUAGE_OPTIONS}
						disabled={edit}
					/>
					<div class="text-sm text-tertiary">
						<Code2 size={16} class="inline mr-2" />
						{edit ? 'Language cannot be changed after creation.' : 'Select the programming language this requirement is for. This will load a default template.'}
					</div>
				</div>
			</Section>

			<Section label="Description">
				<input
					type="text"
					bind:value={requirement.description}
					placeholder="Brief description of this requirement (optional)"
					disabled={!can_write}
					class="input"
				/>
				<div class="text-sm text-tertiary mt-2">
					Provide a brief description to help others understand the purpose of this requirement.
				</div>
			</Section>

			<Section label="Requirement Content">
				{#snippet header()}
					<span class="text-sm text-tertiary mr-4 font-normal">
						({requirement.content.length}/{MAX_REQUIREMENT_LENGTH} characters)
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
							code={requirement.content}
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
				disabled={false}
				startIcon={{ icon: Rocket }}
				color="dark"
				size="sm"
			>
				Deploy
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
