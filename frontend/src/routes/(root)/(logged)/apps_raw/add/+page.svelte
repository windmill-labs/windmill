<script lang="ts">
	import { importStore } from '$lib/components/apps/store'

	import { AppService, type Policy, WorkspaceService } from '$lib/gen'
	import { page } from '$app/stores'
	import { decodeState } from '$lib/utils'
	import { userStore, workspaceStore, dbSchemas } from '$lib/stores'
	import { afterNavigate, replaceState } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'

	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import FileEditorIcon from '$lib/components/raw_apps/FileEditorIcon.svelte'
	import { react18Template, react19Template, svelte5Template } from './templates'
	import type { Runnable } from '$lib/components/raw_apps/rawAppPolicy'
	import { type RawAppData, DEFAULT_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
	import { resource } from 'runed'
	import { getDbSchemas } from '$lib/components/apps/components/display/dbtable/metadata'
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { AlertTriangle, Sparkles, ArrowRight } from 'lucide-svelte'
	import { copilotInfo } from '$lib/aiStore'
	import { aiChatManager, AIMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	let nodraft = $page.url.searchParams.get('nodraft')
	const templatePath = $page.url.searchParams.get('template')
	const templateId = $page.url.searchParams.get('template_id')

	const importRaw = $importStore
	if ($importStore) {
		$importStore = undefined
	}

	const appState = nodraft ? undefined : localStorage.getItem('rawapp')

	let summary = $state('')
	let files: Record<string, string> = $state(react19Template)
	afterNavigate(() => {
		if (nodraft) {
			let url = new URL($page.url.href)
			url.search = ''
			replaceState(url.toString(), $page.state)
		}
	})
	let policy: Policy = $state({
		on_behalf_of: $userStore?.username.includes('@')
			? $userStore?.username
			: `u/${$userStore?.username}`,
		on_behalf_of_email: $userStore?.email,
		execution_mode: 'publisher'
	})

	let runnables: Record<string, Runnable> = $state({
		a: {
			name: 'a',
			fields: {},
			type: 'inline',
			inlineScript: {
				content:
					'// import * as wmill from "windmill-client"\n\nexport async function main(x: string) {\n  return x\n}\n',
				language: 'bun',
				schema: {
					$schema: 'https://json-schema.org/draft/2020-12/schema',
					properties: {
						x: {
							default: null,
							description: '',
							originalType: 'string',
							type: 'string'
						}
					},
					required: ['x'],
					type: 'object'
				}
			}
		}
	})
	/** Data configuration including tables and creation policy */
	let data: RawAppData = $state({ ...DEFAULT_DATA })
	loadApp()

	function extractValue(value: any) {
		files = value.files
		runnables = value.runnables
		// Support old formats and new format
		if (value.data) {
			const d = value.data
			// Handle old nested creation format
			if (d.creation) {
				data = {
					tables: d.tables ?? [],
					datatable: d.creation.datatable,
					schema: d.creation.schema
				}
			} else {
				data = d
			}
		} else if (value.dataTableRefs) {
			data = { ...DEFAULT_DATA, tables: value.dataTableRefs }
		}
	}
	async function loadApp() {
		if (importRaw) {
			sendUserToast('Loaded from YAML/JSON')
			if ('value' in importRaw) {
				summary = importRaw.summary
				extractValue(importRaw.value)

				policy = importRaw.policy
			} else {
				extractValue(importRaw)
			}
			console.log('importRaw', importRaw)
		} else if (templatePath) {
			const template = await AppService.getAppByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})
			extractValue(template.value)
			console.log('App loaded from template')
			sendUserToast('App loaded from template path')
			goto('?', { replaceState: true })
		} else if (templateId) {
			const template = await AppService.getAppByVersion({
				workspace: $workspaceStore!,
				id: parseInt(templateId)
			})
			extractValue(template.value)
			console.log('App loaded from template id')
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (!templatePath && appState) {
			console.log('App loaded from browser stored autosave')
			sendUserToast('App restored from browser stored autosave', false, [
				{
					label: 'Start from blank',
					callback: () => {
						files = {}
						runnables = {}
					}
				}
			])
			let decoded = decodeState(appState)
			extractValue(decoded)
		}
	}

	const templates = [
		{
			name: 'React 19',
			icon: 'tsx',
			files: undefined,
			selected: true
		},
		{
			name: 'React 18',
			icon: 'tsx',
			files: react18Template
		},
		{
			name: 'Svelte 5',
			icon: 'svelte',
			files: svelte5Template
		}
	]
	let templatePicker = $state(nodraft != null)
	let reloadCounter = $state(0)

	// Modal state
	let selectedTemplateIndex = $state(0)
	let tableCreationEnabled = $state(true)
	let selectedDatatable = $state<string | undefined>(undefined)
	let selectedSchema = $state<string | undefined>(undefined)
	let initialPrompt = $state('')

	// Load available datatables from workspace
	const datatables = resource<string[]>([], async () => {
		if (!$workspaceStore) return []
		try {
			return await WorkspaceService.listDataTables({ workspace: $workspaceStore })
		} catch (e) {
			console.error('Failed to load datatables:', e)
			return []
		}
	})

	// Auto-select first datatable when datatables load
	$effect(() => {
		if (datatables.current.length > 0 && !selectedDatatable) {
			selectedDatatable = datatables.current[0]
		}
	})

	// Load schemas for the selected datatable
	const schemas = resource<string[]>([], async () => {
		if (!selectedDatatable || !$workspaceStore) return []

		const resourcePath = `datatable://${selectedDatatable}`
		let dbSchema = $dbSchemas[resourcePath]

		if (!dbSchema) {
			try {
				await getDbSchemas('postgresql', resourcePath, $workspaceStore, $dbSchemas, (msg) =>
					console.error('Schema error:', msg)
				)
				dbSchema = $dbSchemas[resourcePath]
			} catch (e) {
				console.error(`Failed to load schema for ${selectedDatatable}:`, e)
				return []
			}
		}

		if (!dbSchema?.schema) return []
		return Object.keys(dbSchema.schema)
	})

	// Reset schema when datatable changes
	let previousDatatable = $state<string | undefined>(undefined)
	$effect(() => {
		if (previousDatatable !== undefined && selectedDatatable !== previousDatatable) {
			selectedSchema = undefined
		}
		previousDatatable = selectedDatatable
	})

	const datatableItems = $derived(
		datatables.current.map((dt) => ({
			value: dt,
			label: dt
		}))
	)

	const schemaItems = $derived(
		schemas.current.map((s) => ({
			value: s,
			label: s
		}))
	)

	const hasNoDatatables = $derived((datatables.current?.length ?? 0) === 0)
	const isAiEnabled = $derived($copilotInfo.enabled)

	function startApp(withPrompt: boolean) {
		const template = templates[selectedTemplateIndex]
		if (template.files) {
			files = template.files
			reloadCounter += 1
		}

		// Set the data configuration
		if (tableCreationEnabled && selectedDatatable) {
			data = {
				...data,
				datatable: selectedDatatable,
				schema: selectedSchema
			}
		} else {
			data = {
				...data,
				datatable: undefined,
				schema: undefined
			}
		}

		// Sync to aiChatManager
		aiChatManager.datatableCreationPolicy = {
			enabled: tableCreationEnabled && !!selectedDatatable,
			datatable: tableCreationEnabled ? selectedDatatable : undefined,
			schema: tableCreationEnabled ? selectedSchema : undefined
		}

		templatePicker = false

		// Remove nodraft from URL
		const url = new URL(window.location.href)
		if (url.searchParams.has('nodraft')) {
			url.searchParams.delete('nodraft')
			window.history.replaceState({}, '', url.toString())
		}

		// If starting with a prompt, trigger AI after a short delay for the editor to initialize
		if (withPrompt && initialPrompt.trim() && isAiEnabled) {
			setTimeout(() => {
				aiChatManager.changeMode(AIMode.APP)
				if (!aiChatManager.open) {
					aiChatManager.toggleOpen()
				}
				aiChatManager.instructions = initialPrompt.trim()
				aiChatManager.sendRequest()
			}, 500)
		}
	}
</script>

{#if templatePicker}
	<Modal kind="X" open title="New App setup">
		<div class="flex flex-col gap-6 min-w-[500px]">
			<!-- Template Selection -->
			<div>
				<h2 class="text-sm font-medium text-primary mb-2">Template</h2>
				<div class="flex flex-wrap gap-3">
					{#each templates as t, i}
						<button
							onclick={() => (selectedTemplateIndex = i)}
							class="w-24 h-24 flex justify-between py-5 flex-col {selectedTemplateIndex === i
								? 'bg-surface-selected ring-2 ring-blue-500'
								: ''} hover:bg-surface-hover border rounded-lg transition-all"
						>
							<div class="w-full flex items-center justify-center">
								<FileEditorIcon file={'.' + t.icon} />
							</div>
							<div class="center-center w-full text-sm">{t.name}</div>
						</button>
					{/each}
				</div>
			</div>

			<!-- Data Configuration -->
			<div class="border-t pt-4">
				<h2 class="text-sm font-medium text-primary mb-3">Data Configuration</h2>

				{#if hasNoDatatables}
					<div
						class="flex items-center gap-2 p-3 rounded-lg bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800"
					>
						<AlertTriangle size={16} class="text-yellow-600 dark:text-yellow-400 shrink-0" />
						<div class="text-sm text-yellow-800 dark:text-yellow-200">
							<span class="font-medium">No datatables configured.</span>
							You can still create an app, but AI won't be able to create database tables. Configure
							datatables in workspace settings to enable this feature.
						</div>
					</div>
				{:else}
					<div class="flex flex-col gap-4">
						<!-- Table Creation Toggle -->
						<div class="flex items-center justify-between">
							<div>
								<span class="text-sm text-secondary">Allow AI to create tables</span>
								<p class="text-xs text-tertiary mt-0.5">
									When enabled, AI can create new database tables as needed
								</p>
							</div>
							<Toggle size="sm" bind:checked={tableCreationEnabled} />
						</div>

						<div class="flex gap-4">
							<!-- Datatable Selector -->
							<div class="flex-1">
								<span class="text-xs text-tertiary mb-1 block">Default Database</span>
								<Select
									items={datatableItems}
									bind:value={selectedDatatable}
									placeholder="Select database"
									size="sm"
								/>
							</div>

							<!-- Schema Selector -->
							<div class="flex-1">
								<span class="text-xs text-tertiary mb-1 block">Default Schema</span>
								<Select
									items={schemaItems}
									bind:value={selectedSchema}
									placeholder="public"
									size="sm"
								/>
							</div>
						</div>
					</div>
				{/if}
			</div>

			<!-- AI Prompt (Optional) -->
			<div class="border-t pt-4">
				<h2 class="text-sm font-medium text-primary mb-2 flex items-center gap-2">
					<Sparkles size={16} class="text-blue-500" />
					Start with AI
					<span class="text-xs font-normal text-tertiary">(optional)</span>
				</h2>

				{#if !isAiEnabled}
					<div
						class="flex items-center gap-2 p-3 rounded-lg bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700"
					>
						<AlertTriangle size={16} class="text-gray-500 shrink-0" />
						<div class="text-sm text-tertiary">
							AI is not configured for this workspace. You can still create an app manually.
						</div>
					</div>
				{:else}
					<div class="flex flex-col gap-2">
						<TextInput
							underlyingInputEl="textarea"
							bind:value={initialPrompt}
							inputProps={{
								rows: 3,
								placeholder:
									"Describe what you want to build... (e.g., 'Create a todo list app with user authentication')"
							}}
						/>
						<p class="text-xs text-tertiary">
							Leave empty to start with a blank template, or describe your app to get AI assistance
							right away.
						</p>
					</div>
				{/if}
			</div>

			<!-- Actions -->
			<div class="border-t pt-4 flex justify-end gap-3">
				<Button
					color="light"
					size="sm"
					on:click={() => startApp(false)}
					disabled={!templates[selectedTemplateIndex]}
				>
					Start without AI
				</Button>
				{#if isAiEnabled}
					<Button
						color="blue"
						size="sm"
						on:click={() => startApp(true)}
						disabled={!templates[selectedTemplateIndex] || !initialPrompt.trim()}
						startIcon={{ icon: Sparkles }}
						endIcon={{ icon: ArrowRight }}
					>
						Start with AI
					</Button>
				{/if}
			</div>
		</div>
	</Modal>
{/if}
{#key reloadCounter}
	<RawAppEditor
		on:savedNewAppPath={(event) => {
			goto(`/apps_raw/edit/${event.detail}`)
		}}
		initFiles={files}
		initRunnables={runnables}
		initData={data}
		{policy}
		path={''}
		{summary}
		newApp
	/>
{/key}
