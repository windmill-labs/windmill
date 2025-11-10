<script lang="ts">
	// TODO: Replace with actual API service when available
	// import { RequirementService } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Alert from './common/alert/Alert.svelte'
	import type SimpleEditor from './SimpleEditor.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import Section from './Section.svelte'
	import { Loader2, Save, Code2, FolderOpen } from 'lucide-svelte'
	import autosize from '$lib/autosize'
	import Select from './select/Select.svelte'

	const dispatch = createEventDispatcher()

	let directoryPath: string = $state('')
	
	// Language options for requirements - only supported languages
	const LANGUAGE_OPTIONS = [
		{ value: 'python', label: 'Python' },
		{ value: 'typescript', label: 'TypeScript' },
		{ value: 'go', label: 'Go' },
		{ value: 'php', label: 'PHP' }
	]

	// Default templates for each language
	const LANGUAGE_TEMPLATES: Record<string, string> = {
		python: `# Python Requirements (requirements.in format)
# Use pip-tools for dependency management: pip-compile requirements.in

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
		documentation?: string
	} = $state({
		content: '',
		language: 'python',
		description: '',
		documentation: ''
	})

	let valid = $state(true)
	let drawer: Drawer | undefined = $state()
	let edit = $state(false)
	let initialPath: string = $state('')
	let can_write = $state(true)
	let editor: SimpleEditor | undefined = $state(undefined)

	const MAX_REQUIREMENT_LENGTH = 50000

	$effect(() => {
		valid = requirement.content.length <= MAX_REQUIREMENT_LENGTH
	})

	// Track when editor is ready
	let editorReady = $state(false)
	
	$effect(() => {
		if (editor && !editorReady) {
			editor.setCode(requirement.content)
			editorReady = true
		}
	})

	// Handle editor content changes
	function handleEditorChange(newCode: string) {
		requirement.content = newCode
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
			content: LANGUAGE_TEMPLATES.python,
			language: 'python',
			description: '',
			documentation: ''
		}
		edit = false
		initialPath = ''
		directoryPath = ''
		can_write = true
		editorReady = false
		drawer?.openDrawer()
	}

	export async function editRequirement(edit_path: string): Promise<void> {
		edit = true
		// TODO: Replace with actual API call
		// const getReq = await RequirementService.getRequirement({
		//     workspace: $workspaceStore ?? '',
		//     path: edit_path
		// })
		
		// Mock data for now
		const getReq = {
			content: LANGUAGE_TEMPLATES.python,
			language: 'python',
			description: 'Mock requirement description',
			documentation: 'Mock documentation',
			workspace_id: $workspaceStore,
			extra_perms: {}
		}

		can_write = getReq.workspace_id == $workspaceStore && canWrite(edit_path, getReq.extra_perms ?? {}, $userStore)

		requirement = {
			content: getReq.content ?? '',
			language: getReq.language ?? 'python',
			description: getReq.description ?? '',
			documentation: getReq.documentation ?? ''
		}
		initialPath = edit_path
		directoryPath = edit_path
		editorReady = false
		drawer?.openDrawer()
	}

	async function createRequirement(): Promise<void> {
		// TODO: Replace with actual API call
		// await RequirementService.createRequirement({
		//     workspace: $workspaceStore!,
		//     requestBody: {
		//         path: directoryPath,
		//         content: requirement.content,
		//         language: requirement.language,
		//         description: requirement.description,
		//         documentation: requirement.documentation
		//     }
		// })
		
		console.log('Creating requirement:', {
			path: directoryPath,
			content: requirement.content,
			language: requirement.language,
			description: requirement.description,
			documentation: requirement.documentation
		})

		sendUserToast(`Created requirement for ${directoryPath}`)
		dispatch('create')
		drawer?.closeDrawer()
	}

	async function updateRequirement(): Promise<void> {
		try {
			// TODO: Replace with actual API call
			// await RequirementService.updateRequirement({
			//     workspace: $workspaceStore!,
			//     path: initialPath,
			//     requestBody: {
			//         path: initialPath != directoryPath ? directoryPath : undefined,
			//         content: requirement.content,
			//         language: requirement.language,
			//         description: requirement.description,
			//         documentation: requirement.documentation
			//     }
			// })

			console.log('Updating requirement:', {
				initialPath,
				newPath: directoryPath,
				content: requirement.content,
				language: requirement.language,
				description: requirement.description,
				documentation: requirement.documentation
			})

			sendUserToast(`Updated requirement ${initialPath}`)
			dispatch('create')
			drawer?.closeDrawer()
		} catch (err) {
			sendUserToast(`Could not update requirement: ${err.body}`, true)
		}
	}

</script>

<Drawer bind:this={drawer} size="1200px">
	<DrawerContent
		title={edit ? `Update requirement at ${initialPath}` : 'Add a requirement'}
		on:close={drawer?.closeDrawer}
	>
		<div class="flex flex-col gap-8">
			{#if !can_write}
				<Alert type="warning" title="Only read access">
					You only have read access to this resource and cannot edit it
				</Alert>
			{/if}
			
			<Section label="Directory Path">
				<div class="flex flex-col gap-4">
					<input
						type="text"
						bind:value={directoryPath}
						placeholder="Enter directory path (e.g., /u/admin/scripts, /my_project)"
						disabled={!can_write}
						class="input"
					/>
					<div class="text-sm text-tertiary">
						<FolderOpen size={16} class="inline mr-2" />
						Requirements apply to all files in the specified directory recursively. Use paths like:
						<code class="bg-surface-secondary px-1 rounded">/u/admin/scripts</code>,
						<code class="bg-surface-secondary px-1 rounded">/my_project</code>, or
						<code class="bg-surface-secondary px-1 rounded">/flows/data_processing</code>
					</div>
				</div>
			</Section>

			<Section label="Language">
				<div class="flex flex-col gap-4">
					<Select
						bind:value={requirement.language}
						items={LANGUAGE_OPTIONS}
					/>
					<div class="text-sm text-tertiary">
						<Code2 size={16} class="inline mr-2" />
						Select the programming language this requirement is for. This will load a default template.
					</div>
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

			<Section label="Description">
				<textarea 
					rows="2" 
					use:autosize 
					bind:value={requirement.description} 
					placeholder="Brief description of this requirement..."
					disabled={!can_write}
				></textarea>
			</Section>

			<Section label="Documentation (Optional)">
				<textarea 
					rows="4" 
					use:autosize 
					bind:value={requirement.documentation} 
					placeholder="Additional usage information, examples, or implementation notes..."
					disabled={!can_write}
				></textarea>
				<div class="text-sm text-tertiary mt-2">
					Use this field to provide extra context, usage examples, or implementation details that help developers understand how to meet this requirement.
				</div>
			</Section>
		</div>
		
		{#snippet actions()}
			<Button
				on:click={() => (edit ? updateRequirement() : createRequirement())}
				disabled={!can_write || !valid || directoryPath.trim() === ''}
				startIcon={{ icon: Save }}
				color="dark"
				size="sm"
			>
				{edit ? 'Update' : 'Save'}
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>