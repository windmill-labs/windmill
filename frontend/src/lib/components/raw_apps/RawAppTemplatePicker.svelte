<script lang="ts">
	import { Sparkles, Plus, List, Ban, ExternalLinkIcon } from 'lucide-svelte'
	import type { Policy } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { getRawAppOperatingWorkspace } from './rawAppWorkspace'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Alert } from '$lib/components/common'
	import { AIBtnClasses } from '$lib/components/copilot/chat/AIButtonStyle'
	import { copilotInfo } from '$lib/aiStore'
	import { react18Template, react19Template, svelte5Template } from './templates'
	import type { Runnable } from './rawAppPolicy'
	import { type DataTableRef, type RawAppData, formatDataTableRef } from './dataTableRefUtils'
	import {
		createDatatablesResource,
		createSchemasResource,
		toDatatableItems,
		toSchemaItems
	} from './datatableUtils.svelte'
	import RawAppDataTableList from './RawAppDataTableList.svelte'
	import RawAppDataTableDrawer from './RawAppDataTableDrawer.svelte'
	import FileEditorIcon from './FileEditorIcon.svelte'

	export type RawAppTemplatePickerResult = {
		files: Record<string, string>
		runnables: Record<string, Runnable>
		data: RawAppData
		summary: string
		policy: Policy
		prompt?: string
	}

	let {
		open = $bindable(),
		onStart
	}: {
		open: boolean
		onStart: (result: RawAppTemplatePickerResult, withPrompt: boolean) => void
	} = $props()

	const templates = [
		{ name: 'React 19', icon: 'tsx', files: react19Template },
		{ name: 'React 18', icon: 'tsx', files: react18Template },
		{ name: 'Svelte 5', icon: 'svelte', files: svelte5Template }
	]

	let selectedTemplateIndex = $state(0)
	let tableCreationEnabled = $state(true)
	let selectedDatatable = $state<string | undefined>(undefined)
	let schemaMode = $state<'none' | 'new' | 'existing'>('new')
	let selectedSchema = $state<string | undefined>(undefined)
	let newSchemaName = $state('')
	let appSummary = $state('')
	let initialPrompt = $state('')
	let preWhitelistedTables = $state<DataTableRef[]>([])
	let dataTableDrawer: RawAppDataTableDrawer | undefined = $state()

	const getOpWs = getRawAppOperatingWorkspace()
	let opWs = $derived(getOpWs?.() ?? $workspaceStore)

	const datatables = createDatatablesResource(() => opWs)
	const schemas = createSchemasResource(
		() => selectedDatatable,
		() => opWs
	)

	const availableDatatables = $derived(datatables.current)
	const availableSchemas = $derived(schemas.current)

	let hasAutoSelected = false
	$effect(() => {
		if (availableDatatables?.length > 0 && !hasAutoSelected) {
			hasAutoSelected = true
			selectedDatatable = availableDatatables.includes('main') ? 'main' : availableDatatables[0]
		}
	})

	function generateUniqueSchemaName(existingSchemas: string[]): string {
		let num = 1
		while (existingSchemas.includes(`app${num}`)) {
			num++
		}
		return `app${num}`
	}

	const newSchemaAlreadyExists = $derived(
		schemaMode === 'new' &&
			newSchemaName.trim() !== '' &&
			(availableSchemas ?? []).includes(newSchemaName.trim())
	)

	let userEditedSchemaName = $state(false)

	$effect(() => {
		const schemas = availableSchemas ?? []
		if (schemaMode === 'new') {
			if (!newSchemaName) {
				newSchemaName = generateUniqueSchemaName(schemas)
				userEditedSchemaName = false
			} else if (!userEditedSchemaName && schemas.includes(newSchemaName)) {
				newSchemaName = generateUniqueSchemaName(schemas)
			}
		}
	})

	const datatableItems = $derived(toDatatableItems(availableDatatables))
	const schemaItems = $derived(toSchemaItems(availableSchemas))

	const effectiveSchema = $derived(
		schemaMode === 'new' ? newSchemaName : schemaMode === 'existing' ? selectedSchema : undefined
	)

	const hasNoDatatables = $derived(availableDatatables?.length === 0)
	const isAiEnabled = $derived($copilotInfo.enabled)

	async function start(withPrompt: boolean) {
		const template = templates[selectedTemplateIndex]

		if (schemaMode === 'new' && newSchemaName && selectedDatatable && opWs) {
			try {
				const { dbSchemaOpsWithPreviewScripts } = await import('$lib/components/dbOps')
				const dbOps = dbSchemaOpsWithPreviewScripts({
					workspace: opWs,
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

		const formattedTables = preWhitelistedTables.map(formatDataTableRef)
		const data: RawAppData =
			tableCreationEnabled && selectedDatatable
				? {
						tables: formattedTables,
						datatable: selectedDatatable,
						schema: effectiveSchema
					}
				: { tables: formattedTables, datatable: undefined, schema: undefined }

		const policy: Policy = {
			on_behalf_of: $userStore?.username.includes('@')
				? $userStore?.username
				: `u/${$userStore?.username}`,
			on_behalf_of_email: $userStore?.email,
			execution_mode: 'publisher'
		}

		open = false
		onStart(
			{
				files: template.files,
				runnables: {},
				data,
				summary: appSummary.trim(),
				policy,
				prompt: withPrompt ? initialPrompt.trim() : undefined
			},
			withPrompt
		)
	}
</script>

{#if open}
	<!-- `bind:open` (not `open`) so the inner Modal's X / Esc / click-
	     outside dismissal propagates back to the parent. Without it the
	     Modal closes its own UI but the picker's `open` prop stays true,
	     so the route's `templatePicker → false` watcher never fires and
	     autosave stays suspended after the dismissal. -->
	<Modal kind="X" bind:open title="New App setup">
		<div class="flex flex-col gap-6 min-w-sm">
			<div>
				<h2 class="text-xs font-semibold text-emphasis mb-1">Summary</h2>
				<TextInput
					bind:value={appSummary}
					inputProps={{
						placeholder: "Brief description of the app (e.g., 'Todo list with authentication')"
					}}
				/>
			</div>

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

						<div class="flex items-center">
							<Toggle
								size="sm"
								bind:checked={tableCreationEnabled}
								options={{ right: 'Allow AI to create new tables' }}
							/>
						</div>

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
							</a> to enable this feature.
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

			<div class="pt-6 flex justify-end gap-3">
				<Button
					variant="default"
					size="sm"
					on:click={() => start(false)}
					disabled={!templates[selectedTemplateIndex] || newSchemaAlreadyExists}
				>
					Start without AI
				</Button>
				{#if isAiEnabled}
					<Button
						variant="accent"
						on:click={() => start(true)}
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

<RawAppDataTableDrawer
	bind:this={dataTableDrawer}
	offset={10000}
	existingRefs={preWhitelistedTables}
	onAdd={(ref) => {
		preWhitelistedTables = [...preWhitelistedTables, ref]
	}}
/>
