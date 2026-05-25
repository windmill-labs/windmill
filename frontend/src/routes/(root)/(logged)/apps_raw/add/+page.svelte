<script lang="ts">
	import { importStore } from '$lib/components/apps/store'

	import { AppService, type Policy } from '$lib/gen'
	import { page } from '$app/state'
	import { userStore, workspaceStore } from '$lib/stores'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'

	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import FileEditorIcon from '$lib/components/raw_apps/FileEditorIcon.svelte'
	import { UserDraft, localDraftDiffers } from '$lib/userDraft.svelte'
	import { readFieldsRecursively } from '$lib/utils'
	import { untrack } from 'svelte'
	import {
		react18Template,
		react19Template,
		svelte5Template
	} from '$lib/components/raw_apps/templates'
	import type { Runnable } from '$lib/components/raw_apps/rawAppPolicy'
	import { type RawAppData, DEFAULT_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
	import {
		createDatatablesResource,
		createSchemasResource,
		toDatatableItems,
		toSchemaItems
	} from '$lib/components/raw_apps/datatableUtils.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Sparkles, Plus, List, Ban, ExternalLinkIcon } from 'lucide-svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import RawAppDataTableList from '$lib/components/raw_apps/RawAppDataTableList.svelte'
	import RawAppDataTableDrawer from '$lib/components/raw_apps/RawAppDataTableDrawer.svelte'
	import { type DataTableRef, formatDataTableRef } from '$lib/components/raw_apps/dataTableRefUtils'
	import { copilotInfo } from '$lib/aiStore'
	import { aiChatManager, AIMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { Alert } from '$lib/components/common'
	import { AIBtnClasses } from '$lib/components/copilot/chat/AIButtonStyle'

	// `nodraft` is captured into a local because we strip it from the URL
	// below — downstream readers like `templatePicker` must see the original
	// signal.
	const nodraft = page.url.searchParams.get('nodraft')
	const templatePath = page.url.searchParams.get('template')
	const templateId = page.url.searchParams.get('template_id')
	const hubId = page.url.searchParams.get('hub')

	// "+ Raw App" / "+ App > Full code" buttons navigate with ?nodraft=true to
	// signal "start fresh". Wipe the persisted empty-path autosave and strip
	// the flag from the URL synchronously so a reload doesn't wipe the
	// freshly-started draft. A plain reload of /apps_raw/add (no nodraft)
	// instead restores the previous session.
	if (nodraft && typeof window !== 'undefined') {
		UserDraft.discard('raw_app', '', undefined)
		const url = new URL(window.location.href)
		url.searchParams.delete('nodraft')
		window.history.replaceState(window.history.state, '', url.toString())
	}

	// Check in-memory store first, then sessionStorage (used when full page reload occurs)
	let importRaw = $importStore
	if ($importStore) {
		$importStore = undefined
	}
	if (!importRaw) {
		const sessionData = sessionStorage.getItem('rawAppImport')
		if (sessionData) {
			sessionStorage.removeItem('rawAppImport')
			importRaw = JSON.parse(sessionData)
		}
	}

	const draftHandle = UserDraft.use<{
		files: Record<string, string>
		runnables: Record<string, Runnable>
		data: RawAppData
		summary: string
		policy?: Policy
		custom_path?: string
	}>('raw_app', '')
	// Restore the persisted autosave so a plain reload of /apps_raw/add
	// resumes the last session. Captured once; the $effect below mirrors
	// later edits back. Import/template/hub flows in loadApp() wipe the
	// entry first (`UserDraft.remove`) for "start fresh" semantics.
	const restoredDraft = untrack(() => draftHandle.draft)

	const defaultRunnables: Record<string, Runnable> = {
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
	}

	let summary = $state(restoredDraft?.summary ?? '')
	let files: Record<string, string> = $state(restoredDraft?.files ?? react19Template)
	let policy: Policy = $state(
		restoredDraft?.policy ?? {
			on_behalf_of: $userStore?.username.includes('@')
				? $userStore?.username
				: `u/${$userStore?.username}`,
			on_behalf_of_email: $userStore?.email,
			execution_mode: 'publisher'
		}
	)

	let runnables: Record<string, Runnable> = $state(restoredDraft?.runnables ?? defaultRunnables)
	/** Data configuration including tables and creation policy */
	let data: RawAppData = $state(restoredDraft?.data ?? { ...DEFAULT_DATA })

	// First mirror consumes the handle's first-write skip up-front (wipe
	// then restore) so the user's first real edit isn't the one dropped.
	let firstMirror = true
	$effect(() => {
		readFieldsRecursively(files)
		readFieldsRecursively(runnables)
		readFieldsRecursively(data)
		readFieldsRecursively(policy)
		void summary
		untrack(() => {
			if (firstMirror) {
				firstMirror = false
				draftHandle.setDraftAndMeta(undefined, {})
			}
			draftHandle.draft = { files, runnables, data, summary, policy }
		})
	})

	// Reflect an external UserDraft.save into the form. Idempotent + the
	// d == null guard keeps it from looping with the mirror above or
	// clobbering "start fresh" loads (which discard the in-memory draft).
	$effect(() => {
		const d = draftHandle.draft
		if (d == null) return
		untrack(() => {
			if (localDraftDiffers(d, { files, runnables, data, summary, policy })) {
				files = d.files
				runnables = d.runnables
				data = d.data
				summary = d.summary
				if (d.policy !== undefined) policy = d.policy
			}
		})
	})

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
			// Import/template/hub loads are an explicit "start fresh from this
			// content" — drop the restored empty-path autosave so it doesn't
			// linger as the next plain reload's baseline.
			UserDraft.discard('raw_app', '', undefined)
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
			UserDraft.discard('raw_app', '', undefined)
			const template = await AppService.getAppByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})
			extractValue(template.value)
			console.log('App loaded from template')
			sendUserToast('App loaded from template path')
			goto('?', { replaceState: true })
		} else if (templateId) {
			UserDraft.discard('raw_app', '', undefined)
			const template = await AppService.getAppByVersion({
				workspace: $workspaceStore!,
				id: parseInt(templateId)
			})
			extractValue(template.value)
			console.log('App loaded from template id')
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (hubId) {
			UserDraft.discard('raw_app', '', undefined)
			const hub = await AppService.getHubRawAppById({ id: Number(hubId) })
			if (hub.app?.value) {
				extractValue(hub.app.value)
			}
			if (hub.app?.summary) {
				summary = hub.app.summary
			}
			console.log('App loaded from Hub')
			sendUserToast('App loaded from Hub')
			goto('?', { replaceState: true })
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
	let templatePicker = $state(nodraft != null && !importRaw)
	let reloadCounter = $state(0)

	// Modal state
	let selectedTemplateIndex = $state(0)
	let tableCreationEnabled = $state(true)
	let selectedDatatable = $state<string | undefined>(undefined)
	let schemaMode = $state<'none' | 'new' | 'existing'>('new')
	let selectedSchema = $state<string | undefined>(undefined)
	let newSchemaName = $state('')
	let appSummary = $state('')
	let initialPrompt = $state('')
	let dataTableDrawer: RawAppDataTableDrawer | undefined = $state()

	// Pre-whitelisted tables for the app
	let preWhitelistedTables = $state<DataTableRef[]>([])

	// Load available datatables and schemas using shared utilities
	const datatables = createDatatablesResource(() => $workspaceStore)
	const schemas = createSchemasResource(() => selectedDatatable)

	// Derived value to force reactivity on datatables.current
	const availableDatatables = $derived(datatables.current)
	const availableSchemas = $derived(schemas.current)

	// Auto-select datatable: prefer "main" if available, otherwise first one
	// Only runs once when datatables first load (selectedDatatable is undefined)
	let hasAutoSelected = false
	$effect(() => {
		if (availableDatatables?.length > 0 && !hasAutoSelected) {
			hasAutoSelected = true
			if (availableDatatables.includes('main')) {
				selectedDatatable = 'main'
			} else {
				selectedDatatable = availableDatatables[0]
			}
		}
	})

	// Generate unique schema name (appX where X is first unused number)
	function generateUniqueSchemaName(existingSchemas: string[]): string {
		let num = 1
		while (existingSchemas.includes(`app${num}`)) {
			num++
		}
		return `app${num}`
	}

	// Check if new schema name already exists
	const newSchemaAlreadyExists = $derived(
		schemaMode === 'new' &&
			newSchemaName.trim() !== '' &&
			(availableSchemas ?? []).includes(newSchemaName.trim())
	)

	// Track if the user has manually edited the schema name
	let userEditedSchemaName = $state(false)

	// Set default new schema name when schemas load or when switching to new mode
	// Also auto-fix if the current name exists and was auto-generated (not user-edited)
	$effect(() => {
		const schemas = availableSchemas ?? []
		if (schemaMode === 'new') {
			if (!newSchemaName) {
				// Initial load: set default name
				newSchemaName = generateUniqueSchemaName(schemas)
				userEditedSchemaName = false
			} else if (!userEditedSchemaName && schemas.includes(newSchemaName)) {
				// Auto-generated name now exists (schemas reloaded), regenerate
				newSchemaName = generateUniqueSchemaName(schemas)
			}
		}
	})

	// Reset schema when datatable changes
	let previousDatatable = $state<string | undefined>(undefined)
	$effect(() => {
		if (previousDatatable !== undefined && selectedDatatable !== previousDatatable) {
			selectedSchema = undefined
			newSchemaName = ''
			userEditedSchemaName = false
		}
		previousDatatable = selectedDatatable
	})

	// Update AI prompt when summary changes
	$effect(() => {
		if (appSummary.trim() && isAiEnabled) {
			initialPrompt = `Build ${appSummary.trim()}`
		}
	})

	const datatableItems = $derived(toDatatableItems(availableDatatables))
	const schemaItems = $derived(toSchemaItems(availableSchemas))

	// The effective schema to use (either selected existing, new schema name, or undefined for none)
	const effectiveSchema = $derived(
		schemaMode === 'new' ? newSchemaName : schemaMode === 'existing' ? selectedSchema : undefined
	)

	const hasNoDatatables = $derived(availableDatatables?.length === 0)
	const isAiEnabled = $derived($copilotInfo.enabled)

	async function startApp(withPrompt: boolean) {
		const template = templates[selectedTemplateIndex]
		if (template.files) {
			files = template.files
			reloadCounter += 1
		}

		// Set summary
		summary = appSummary.trim()

		// Create new schema if needed
		if (schemaMode === 'new' && newSchemaName && selectedDatatable && $workspaceStore) {
			try {
				const { dbSchemaOpsWithPreviewScripts } = await import('$lib/components/dbOps')
				const dbOps = dbSchemaOpsWithPreviewScripts({
					workspace: $workspaceStore,
					input: {
						type: 'database',
						resourceType: 'postgresql',
						resourcePath: `datatable://${selectedDatatable}`
					}
				})
				await dbOps.onCreateSchema({ schema: newSchemaName })
			} catch (e) {
				console.error('Failed to create schema:', e)
				sendUserToast(`Failed to create schema: ${e}`, true)
			}
		}

		// Set the data configuration including pre-whitelisted tables
		const formattedTables = preWhitelistedTables.map(formatDataTableRef)
		if (tableCreationEnabled && selectedDatatable) {
			data = {
				tables: formattedTables,
				datatable: selectedDatatable,
				schema: effectiveSchema
			}
		} else {
			data = {
				tables: formattedTables,
				datatable: undefined,
				schema: undefined
			}
		}

		// Sync to aiChatManager
		aiChatManager.datatableCreationPolicy = {
			enabled: tableCreationEnabled && !!selectedDatatable,
			datatable: tableCreationEnabled ? selectedDatatable : undefined,
			schema: tableCreationEnabled ? effectiveSchema : undefined
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
		<div class="flex flex-col gap-6 min-w-sm">
			<!-- Summary -->
			<div>
				<h2 class="text-xs font-semibold text-emphasis mb-1">Summary</h2>
				<TextInput
					bind:value={appSummary}
					inputProps={{
						placeholder: "Brief description of the app (e.g., 'Todo list with authentication')"
					}}
				/>
			</div>

			<!-- Template Selection -->
			<div class="pt-6">
				<h2 class="text-xs font-semibold text-emphasis mb-1">Framework</h2>
				<div class="flex flex-wrap gap-3">
					{#each templates as t, i}
						<button
							onclick={() => (selectedTemplateIndex = i)}
							class="w-24 h-24 flex justify-between py-5 flex-col {selectedTemplateIndex === i
								? 'bg-surface-accent-selected border border-accent'
								: ''} hover:bg-surface-hover border rounded-lg transition-all"
						>
							<div class="w-full flex items-center justify-center">
								<FileEditorIcon file={'.' + t.icon} size={32} />
							</div>
							<div class="center-center w-full text-sm text-secondary">{t.name}</div>
						</button>
					{/each}
				</div>
			</div>

			<!-- Data Configuration -->
			<div class="pt-6">
				<h2 class="text-xs font-semibold text-emphasis mb-1">Data configuration</h2>

				{#if hasNoDatatables}
					<Alert type="warning" title="No datatables configured.">
						You can still create an app, but for data storage you won't be able to use data tables
						which are <b>highly recommended</b>.
						<br />

						{#if $userStore?.is_admin}
							Configure datatables in
							<a
								href="/workspace_settings?tab=windmill_data_tables"
								target="_blank"
								class="inline-flex items-center gap-1"
								>workspace settings <ExternalLinkIcon size={16} />
							</a> to enable this feature.
						{:else}
							Ask your workspace admin to configure datatables in workspace settings to enable this
							feature.
						{/if}
					</Alert>
				{:else}
					<div class="flex flex-col gap-4">
						<!-- Default Datatable & Schema -->
						<div class="flex flex-col gap-1">
							<span class="text-xs text-secondary mb-1 block">Default settings for new tables</span>
							<div class="flex flex-col gap-4 rounded-md p-4 border">
								<div class="flex flex-col gap-4">
									<div class="flex flex-col gap-1">
										<label class="text-xs text-emphasis font-semibold" for="datatable"
											>Datatable</label
										>
										<Select
											id="datatable"
											disablePortal
											items={datatableItems}
											bind:value={selectedDatatable}
											placeholder="Datatable"
											size="sm"
											class="w-40"
										/>
									</div>
									<div>
										<span class="text-xs text-emphasis font-semibold">Schema</span>

										<div class="flex flex-row gap-1 w-full items-center">
											<div>
												<ToggleButtonGroup bind:selected={schemaMode} noWFull>
													{#snippet children({ item })}
														<ToggleButton value="none" label="None" icon={Ban} {item} size="sm" />
														<ToggleButton value="new" label="New" icon={Plus} {item} size="sm" />
														<ToggleButton
															value="existing"
															label="Existing"
															icon={List}
															{item}
															size="sm"
														/>
													{/snippet}
												</ToggleButtonGroup>
											</div>
											{#if schemaMode === 'new'}
												<TextInput
													bind:value={newSchemaName}
													inputProps={{
														placeholder: 'Schema name',
														oninput: () => (userEditedSchemaName = true)
													}}
													class="flex-1"
													error={newSchemaAlreadyExists}
													size="sm"
												/>
											{:else if schemaMode === 'existing'}
												<div class="flex-1">
													<Select
														disablePortal
														items={schemaItems}
														bind:value={selectedSchema}
														placeholder="Schema"
														size="sm"
													/>
												</div>
											{/if}
										</div>
										{#if newSchemaAlreadyExists}
											<span class="text-xs text-red-500"
												>Schema "{newSchemaName}" already exists</span
											>
										{/if}
									</div>
								</div>
							</div>
						</div>

						<!-- Table Creation Toggle -->
						<div class="flex items-center">
							<Toggle
								size="sm"
								bind:checked={tableCreationEnabled}
								options={{ right: 'Allow AI to create new tables' }}
							/>
						</div>

						<!-- Pre-whitelisted Tables -->
						<div class="pt-6">
							<RawAppDataTableList
								dataTableRefs={preWhitelistedTables}
								defaultDatatable={selectedDatatable}
								defaultSchema={effectiveSchema}
								standalone
								hideDefaultSelector
								onAdd={() => dataTableDrawer?.openDrawer()}
								onRemove={(index) => {
									preWhitelistedTables = preWhitelistedTables.filter((_, i) => i !== index)
								}}
							/>
						</div>
					</div>
				{/if}
			</div>

			<!-- AI Prompt (Optional) -->
			<div class="pt-6">
				<h2 class="text-xs font-semibold text-emphasis mb-1 flex items-center gap-2">
					<Sparkles size={16} class="text-ai" />
					Start with AI
					<span class="text-xs font-normal text-tertiary">(optional)</span>
				</h2>

				{#if !isAiEnabled}
					<Alert type="info" title="AI is not configured for this workspace.">
						You can still create an app manually but using AI is highly recommended.
						<br />
						{#if $userStore?.is_admin}
							Configure AI in
							<a
								href="/workspace_settings?tab=ai"
								target="_blank"
								class="inline-flex items-center gap-1 font-semibold"
								>workspace settings <ExternalLinkIcon size={16} />
							</a>
							to enable this feature.
						{:else}
							Ask your workspace admin to configure AI in workspace settings to enable this feature.
						{/if}
					</Alert>
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
			<div class="pt-6 flex justify-end gap-3">
				<Button
					variant="default"
					size="sm"
					on:click={() => startApp(false)}
					disabled={!templates[selectedTemplateIndex] || newSchemaAlreadyExists}
				>
					Start without AI
				</Button>
				{#if isAiEnabled}
					<Button
						variant="accent"
						on:click={() => startApp(true)}
						disabled={!templates[selectedTemplateIndex] ||
							!initialPrompt.trim() ||
							newSchemaAlreadyExists}
						startIcon={{ icon: Sparkles }}
						btnClasses={AIBtnClasses('accent')}
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
		bind:files
		bind:runnables
		bind:data
		{policy}
		path={''}
		liveEditorDraftStoragePath=""
		bind:summary
		newApp
	/>
{/key}

<RawAppDataTableDrawer
	bind:this={dataTableDrawer}
	offset={10000}
	existingRefs={preWhitelistedTables}
	onAdd={(ref) => {
		preWhitelistedTables = [...preWhitelistedTables, ref]
	}}
/>
