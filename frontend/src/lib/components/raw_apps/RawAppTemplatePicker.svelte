<script lang="ts">
	import { untrack } from 'svelte'
	import {
		Sparkles,
		Plus,
		List,
		Ban,
		ExternalLinkIcon,
		Code,
		Database,
		Loader2
	} from 'lucide-svelte'
	import { WorkerService, type Policy } from '$lib/gen'
	import MissingWorkerTagAlert from '$lib/components/jobs/MissingWorkerTagAlert.svelte'
	import { hasWorkerForTag, queuedWithoutWorkerMessage } from '$lib/components/jobs/missingWorker'
	import { superadmin, userStore } from '$lib/stores'
	import { base } from '$lib/base'
	import { sendUserToast } from '$lib/toast'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Alert, Badge } from '$lib/components/common'
	import { copilotInfo, copilotWorkspace } from '$lib/aiStore'
	import { loadCopilot } from '$lib/components/copilot/loadCopilot'
	import { react18Template, react19Template, svelte5Template } from './templates'
	import type { Runnable } from './rawAppPolicy'
	import {
		type DataTableRef,
		type RawAppData,
		formatDataTableRef,
		withAppDatatableRole
	} from './dataTableRefUtils'
	import {
		createDatatableAccessResource,
		createDatatablesResource,
		createRolesResource,
		rolesWorthPicking,
		toDatatableItems,
		toSchemaItems
	} from './datatableUtils.svelte'
	import { datatableNameTakesRole, defaultMigrationRole } from '../dbTypes'
	import RawAppDataTableList from './RawAppDataTableList.svelte'
	import RawAppDataTableDrawer from './RawAppDataTableDrawer.svelte'
	import FileEditorIcon from './FileEditorIcon.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { useOperatorBuilderApps } from '$lib/operatorWriteRights'
	import { prefersSessionHandoff } from '$lib/components/copilot/chat/global/gate'

	export type RawAppTemplatePickerResult = {
		files: Record<string, string>
		runnables: Record<string, Runnable>
		data: RawAppData
		policy: Policy
	}

	/** Where the set-up app opens: `ai` hands it to the AI, `code` opens it in the editor. */
	export type RawAppBuildMode = 'ai' | 'code'

	let {
		open = $bindable(),
		onStart
	}: {
		open: boolean
		onStart: (result: RawAppTemplatePickerResult, mode: RawAppBuildMode) => void
	} = $props()

	const templates = [
		{ name: 'React 19', icon: 'tsx', files: react19Template, recommended: true },
		{ name: 'React 18', icon: 'tsx', files: react18Template },
		{ name: 'Svelte 5', icon: 'svelte', files: svelte5Template }
	]

	let selectedTemplateIndex = $state(0)
	let selectedDatatable = $state<string | undefined>(undefined)
	let schemaMode = $state<'none' | 'new' | 'existing'>('new')
	let selectedSchema = $state<string | undefined>(undefined)
	let newSchemaName = $state('')
	let preWhitelistedTables = $state<DataTableRef[]>([])
	/** The role each pre-whitelisted table's data table was browsed as. */
	let preWhitelistedRoles = $state<Record<string, string>>({})
	let dataTableDrawer: RawAppDataTableDrawer | undefined = $state()

	const operatingWorkspace = useOperatingWorkspace()
	const operatorBuilderApps = useOperatorBuilderApps()
	let opWs = $derived($operatingWorkspace)

	const datatables = createDatatablesResource(() => opWs)
	const roles = createRolesResource(
		() => selectedDatatable,
		() => opWs
	)
	let selectedRole = $state<string | undefined>(undefined)

	// Every reader waits for an answer stamped with the current selection: until then `current`
	// belongs to the previous workspace, data table or role.
	// A data table whose name cannot carry a role in a reference is used as its default one.
	const rolesAnswered = $derived(
		roles.current.workspace === opWs && roles.current.datatable === selectedDatatable
	)
	const loadedRoles = $derived(
		rolesAnswered && selectedDatatable !== undefined && datatableNameTakesRole(selectedDatatable)
			? roles.current.roles
			: []
	)
	const showRolePicker = $derived(rolesWorthPicking(loadedRoles))
	// Saved explicitly rather than left to resolve: "whatever the default is then" moves the app
	// the day an admin changes the default.
	const effectiveRole = $derived(
		selectedRole !== undefined && loadedRoles.includes(selectedRole) ? selectedRole : undefined
	)
	// An app uses one role per data table: the one picked above is what the table drawer browses
	// as and what the list shows, or tables would be added under a role the app is not saved with.
	const pickerRoles = $derived(
		selectedDatatable !== undefined && effectiveRole !== undefined
			? withAppDatatableRole(preWhitelistedRoles, selectedDatatable, effectiveRole)
			: preWhitelistedRoles
	)

	const availableDatatables = $derived(datatables.current)
	// `undefined` while the list loads, so this is false until it has answered.
	const hasNoDatatables = $derived(availableDatatables?.length === 0)

	const rolesSettled = $derived(
		hasNoDatatables || (selectedDatatable !== undefined && rolesAnswered)
	)

	// A role is picked on one data table: two data tables can both define an `analyst` that
	// means something different, so a name surviving the switch is not the role surviving it.
	let rolesPickedOn = $state<string | undefined>(undefined)
	$effect(() => {
		const loaded = roles.current
		if (!rolesAnswered) return
		const switched = untrack(() => rolesPickedOn) !== selectedDatatable
		const current = untrack(() => selectedRole)
		if (switched || current === undefined || !loaded.roles.includes(current)) {
			// Tables already picked on this data table were browsed as a role: keep that one.
			const browsed = selectedDatatable
				? untrack(() => preWhitelistedRoles)[selectedDatatable]
				: undefined
			pickRole(
				browsed !== undefined && loaded.roles.includes(browsed)
					? browsed
					: loaded.roles.includes(loaded.defaultRole)
						? loaded.defaultRole
						: loaded.roles[0]
			)
			rolesPickedOn = selectedDatatable
		}
	})

	/** Picks the app's role on the selected data table. Tables picked on it under another role are
	 * dropped: that role may reach them where this one does not. */
	function pickRole(role: string | undefined) {
		selectedRole = role
		const dt = selectedDatatable
		const browsed = dt ? preWhitelistedRoles[dt] : undefined
		if (dt === undefined || role === undefined || browsed === undefined || browsed === role) return
		preWhitelistedTables = preWhitelistedTables.filter((t) => t.datatable !== dt)
		const { [dt]: _, ...rest } = preWhitelistedRoles
		preWhitelistedRoles = rest
	}

	const access = createDatatableAccessResource(
		() => selectedDatatable,
		() => effectiveRole,
		() => opWs,
		() => rolesSettled && (loadedRoles.length === 0 || effectiveRole !== undefined)
	)
	// Under roles, and this caller may use none of them: the app would be saved with queries the
	// server refuses.
	const noUsableRole = $derived(
		rolesSettled &&
			selectedDatatable !== undefined &&
			roles.current.permissioned &&
			roles.current.roles.length === 0
	)
	const rolesUnknown = $derived(rolesSettled && roles.current.failed)
	const accessSettled = $derived(
		hasNoDatatables ||
			(rolesSettled &&
				selectedDatatable !== undefined &&
				access.current.workspace === opWs &&
				access.current.datatable === selectedDatatable &&
				access.current.role === effectiveRole)
	)
	const accessUnknown = $derived(accessSettled && access.current.failed)
	const availableSchemas = $derived(accessSettled ? access.current.schemas : [])
	const canCreateSchema = $derived(accessSettled && access.current.canCreateSchema)

	const blockedByRole = $derived(noUsableRole || rolesUnknown || accessUnknown)
	// A data table the caller cannot use is left out rather than blocking the app: it would be
	// saved with queries the server refuses, and the app may need no data table at all.
	const usableDatatable = $derived(blockedByRole ? undefined : selectedDatatable)

	// A role that cannot create schemas has nothing to name, so the mode goes back to the one
	// every role has, once that is an answer.
	$effect(() => {
		if (accessSettled && !accessUnknown && schemaMode === 'new' && !canCreateSchema) {
			schemaMode = 'none'
		}
	})
	// Likewise an existing schema the current role no longer reaches is unpicked, so the select
	// does not keep showing it.
	$effect(() => {
		if (
			accessSettled &&
			selectedSchema !== undefined &&
			!availableSchemas.includes(selectedSchema)
		) {
			selectedSchema = undefined
		}
	})

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

	// An existing schema counts only while the current role reaches it: one picked under another
	// role would save an app that creates its tables where it cannot.
	const effectiveSchema = $derived(
		schemaMode === 'new'
			? newSchemaName
			: schemaMode === 'existing' &&
				  selectedSchema !== undefined &&
				  availableSchemas.includes(selectedSchema)
				? selectedSchema
				: undefined
	)

	// copilotInfo is a global that stays empty until some ancestor's fetch lands, so
	// `enabled` alone cannot tell "no providers" from "not loaded yet" and the modal
	// would announce AI as unconfigured while it is merely unknown. Gate on the
	// config describing opWs, and load it here so the claim owns its own evidence.
	const aiConfigLoaded = $derived(!!opWs && $copilotWorkspace === opWs)
	const isAiEnabled = $derived(aiConfigLoaded && $copilotInfo.enabled)

	$effect(() => {
		if (open && opWs && !aiConfigLoaded) {
			loadCopilot(opWs)
		}
	})

	// With AI turned off for the workspace there is nothing to choose between. A builder is
	// offered AI only through a session: the docked app mode writes inline code they cannot save.
	const aiOffered = $derived(
		!$copilotInfo.workspaceDisabled &&
			(!$operatorBuilderApps || prefersSessionHandoff($userStore?.operator))
	)

	const canStart = $derived(
		!!templates[selectedTemplateIndex] && !newSchemaAlreadyExists && rolesSettled && accessSettled
	)

	function appPolicy(): Policy {
		return {
			on_behalf_of: $userStore?.username.includes('@')
				? $userStore?.username
				: `u/${$userStore?.username}`,
			on_behalf_of_email: $userStore?.email,
			execution_mode: 'publisher'
		}
	}

	const startCardClasses =
		'relative h-full flex-col items-start justify-start gap-1.5 rounded-lg p-3 text-left whitespace-normal'

	async function createSchema(input: {
		workspace: string
		datatable: string
		schema: string
		role: string | undefined
		migrationRole: string | undefined
	}) {
		try {
			const { dbSchemaOpsWithPreviewScripts } = await import('$lib/components/dbOps')
			const dbOps = dbSchemaOpsWithPreviewScripts({
				workspace: input.workspace,
				input: {
					type: 'database',
					resourceType: 'postgresql',
					resourcePath: `datatable://${input.datatable}`,
					role: input.role,
					migrationRole: input.migrationRole
				}
			})
			await dbOps.onCreateSchema({ schema: input.schema })
		} catch (e) {
			console.error('Failed to create schema:', e)
			sendUserToast(`Failed to create schema ${input.schema}: ${e}`, true)
		}
	}

	// The start that is in flight, and the schema it is waiting on: creating one is a job,
	// which can sit in the queue long enough for a silent modal to look dead.
	let starting = $state<{ mode: RawAppBuildMode; schema?: string } | undefined>(undefined)

	const SCHEMA_JOB_TAG = 'postgresql'

	/** Whether a worker can pick up the job that creates a schema. Only a definite "no"
	 * counts: with per-workspace default tags the job runs on a variant of the tag, and a
	 * failed lookup establishes nothing. */
	async function schemaJobsServed(workspace: string): Promise<boolean> {
		try {
			if (await WorkerService.isDefaultTagsPerWorkspace()) return true
			return await hasWorkerForTag(workspace, SCHEMA_JOB_TAG)
		} catch {
			return true
		}
	}

	// Existing tables go with an existing schema: under the other modes the list is not shown,
	// and what is not shown is not saved.
	const tablesToUse = $derived(schemaMode === 'existing' ? preWhitelistedTables : [])

	function exploreDatatable() {
		dataTableDrawer?.openDrawer(
			selectedDatatable
				? {
						datatable: selectedDatatable,
						schema: schemaMode === 'existing' ? selectedSchema : undefined
					}
				: undefined
		)
	}

	async function start(mode: RawAppBuildMode) {
		if (starting) return
		const template = templates[selectedTemplateIndex]

		if (schemaMode === 'new' && newSchemaName && usableDatatable && opWs) {
			starting = { mode, schema: newSchemaName }
			const creation = createSchema({
				workspace: opWs,
				datatable: usableDatatable,
				schema: newSchemaName,
				role: effectiveRole,
				migrationRole: defaultMigrationRole(
					usableDatatable,
					roles.current.permissioned,
					roles.current.defaultRole
				)
			})
			if (await schemaJobsServed(opWs)) {
				await creation
			} else {
				// The job stays queued for as long as no worker serves its tag: waiting on it
				// would hold the app back for nothing, so say why the schema is missing instead.
				sendUserToast(
					`Schema "${newSchemaName}" is not created yet. ${queuedWithoutWorkerMessage(SCHEMA_JOB_TAG)}`,
					true
				)
			}
		}

		starting = undefined

		const formattedTables = tablesToUse.map(formatDataTableRef)
		const keepsDatatable = usableDatatable !== undefined
		// The roles shown, for the data tables the app ends up using.
		const usedDatatables = new Set(tablesToUse.map((t) => t.datatable))
		if (keepsDatatable) usedDatatables.add(usableDatatable!)
		const shownRoles = Object.entries(pickerRoles ?? {}).filter(([dt]) => usedDatatables.has(dt))
		const appRoles = shownRoles.length > 0 ? Object.fromEntries(shownRoles) : undefined
		const data: RawAppData = keepsDatatable
			? {
					tables: formattedTables,
					datatable: usableDatatable,
					schema: effectiveSchema,
					roles: appRoles
				}
			: { tables: formattedTables, datatable: undefined, schema: undefined, roles: appRoles }

		open = false
		onStart(
			{
				files: template.files,
				runnables: {},
				data,
				policy: appPolicy()
			},
			mode
		)
	}
</script>

{#if open}
	<!-- `bind:open` (not `open`) so the inner Modal's X / Esc / click-
	     outside dismissal propagates back to the parent. Without it the
	     Modal closes its own UI but the picker's `open` prop stays true,
	     so the route's `templatePicker → false` watcher never fires and
	     autosave stays suspended after the dismissal. -->
	<Modal kind="X" bind:open title="New app">
		<div class="flex flex-col gap-6 min-w-sm">
			<div>
				<h2 class="text-xs font-semibold text-emphasis mb-3">Framework</h2>
				<div class="flex flex-wrap gap-3">
					{#each templates as t, i}
						<button
							onclick={() => (selectedTemplateIndex = i)}
							class="relative w-20 h-[4.5rem] flex justify-between py-3 flex-col {selectedTemplateIndex ===
							i
								? 'bg-surface-accent-selected border border-accent'
								: ''} hover:bg-surface-hover border rounded-lg transition-all"
						>
							<div class="w-full flex items-center justify-center">
								<FileEditorIcon file={'.' + t.icon} size={22} />
							</div>
							<div class="center-center w-full text-xs text-secondary">{t.name}</div>
							{#if t.recommended}
								<div class="absolute -top-3 left-1/2 -translate-x-1/2">
									<Badge color="blue" small>Recommended</Badge>
								</div>
							{/if}
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
						<MissingWorkerTagAlert
							tag={SCHEMA_JOB_TAG}
							subject="Data table queries and schema changes"
						/>
						<div class="flex flex-col gap-1">
							<span class="text-xs text-secondary mb-1 block">Default settings for new tables</span>
							<div class="flex flex-col gap-4 rounded-md p-4 border">
								<div class="flex flex-col gap-4">
									<div class="flex flex-col gap-1">
										<label class="text-xs text-emphasis font-semibold" for="datatable"
											>Datatable</label
										>
										<div class="flex flex-row items-center gap-2">
											<Select
												id="datatable"
												disablePortal
												items={datatableItems}
												bind:value={selectedDatatable}
												placeholder="Datatable"
												size="sm"
												class="w-40"
											/>
											{#if showRolePicker}
												<!-- Reads as one phrase, "main as analyst", so the role needs no label. -->
												<span class="text-xs text-secondary">as</span>
												<Select
													id="datatable-role"
													disablePortal
													items={loadedRoles.map((r) => ({ value: r, label: r }))}
													bind:value={() => selectedRole, pickRole}
													clearable={false}
													placeholder="Role"
													size="sm"
													class="w-40"
												/>
											{/if}
											{#if noUsableRole || rolesUnknown || accessUnknown}
												<span
													class="text-xs text-red-600 dark:text-red-400"
													title={access.current.error}
												>
													{rolesUnknown
														? 'could not read its roles'
														: noUsableRole
															? 'no role you can use'
															: 'could not reach it'}; the app is created without a default data
													table
												</span>
											{/if}
										</div>
									</div>
									<div>
										<span class="text-xs text-emphasis font-semibold">Schema</span>
										<div class="flex flex-row gap-1 w-full items-center">
											<div>
												<ToggleButtonGroup bind:selected={schemaMode} noWFull>
													{#snippet children({ item })}
														<ToggleButton value="none" label="None" icon={Ban} {item} size="sm" />
														<ToggleButton
															value="new"
															label="New"
															icon={Plus}
															disabled={!canCreateSchema}
															tooltip={canCreateSchema
																? undefined
																: noUsableRole
																	? `You can use no role of ${selectedDatatable}`
																	: accessUnknown
																		? `Could not read what may be created in ${selectedDatatable}`
																		: `${effectiveRole ?? 'This connection'} cannot create schemas in ${selectedDatatable}`}
															{item}
															size="sm"
														/>
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
											<Button
												variant="default"
												unifiedSize="sm"
												iconOnly
												startIcon={{ icon: Database }}
												title="Explore {selectedDatatable ?? 'the data table'}"
												onclick={exploreDatatable}
											/>
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

						{#if schemaMode === 'existing'}
							<div class="pt-6">
								<RawAppDataTableList
									dataTableRefs={preWhitelistedTables}
									defaultDatatable={selectedDatatable}
									defaultSchema={effectiveSchema}
									roles={pickerRoles}
									standalone
									hideDefaultSelector
									onAdd={exploreDatatable}
									onRemove={(index) => {
										preWhitelistedTables = preWhitelistedTables.filter((_, i) => i !== index)
									}}
								/>
							</div>
						{/if}
					</div>
				{/if}
			</div>

			{#if aiOffered && aiConfigLoaded && !isAiEnabled}
				<Alert type="info" title="AI is not configured.">
					You can still create an app manually but using AI is highly recommended.
					<br />
					{#if $userStore?.is_admin}
						Configure AI in
						<a
							href="{base}/workspace_settings?tab=ai"
							target="_blank"
							class="inline-flex items-center gap-1 font-semibold"
							>workspace settings <ExternalLinkIcon size={16} />
						</a>
						{#if $superadmin}
							or
							<a
								href="{base}/?workspace=admins#superadmin-settings"
								target="_blank"
								class="inline-flex items-center gap-1 font-semibold"
								>instance settings <ExternalLinkIcon size={16} />
							</a>
						{/if} to enable this feature.
					{:else if $superadmin}
						Configure AI in
						<a
							href="{base}/?workspace=admins#superadmin-settings"
							target="_blank"
							class="inline-flex items-center gap-1 font-semibold"
							>instance settings <ExternalLinkIcon size={16} />
						</a> to enable this feature.
					{:else}
						Ask your workspace admin to configure AI in workspace settings to enable this feature.
					{/if}
				</Alert>
			{/if}

			{#if aiOffered}
				<div class="pt-6 flex flex-col gap-2">
					<div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
						<Button
							variant="default"
							onClick={() => start('ai')}
							disabled={!canStart || !isAiEnabled || starting !== undefined}
							btnClasses="{startCardClasses} border-ai/40 bg-ai/5 hover:bg-ai/10 disabled:opacity-50 disabled:cursor-not-allowed"
						>
							<span class="flex w-full items-center gap-2">
								{#if !aiConfigLoaded || starting?.mode === 'ai'}
									<Loader2 size={18} class="text-ai animate-spin" />
								{:else}
									<Sparkles size={18} class="text-ai" />
								{/if}
								<span class="text-sm font-semibold text-emphasis whitespace-nowrap"
									>Start with AI</span
								>
							</span>
							<span class="text-xs font-normal text-secondary">
								Describe your app and let the AI build it.
							</span>
							<span class="absolute -top-3 right-3">
								<Badge color="blue" small>Recommended</Badge>
							</span>
						</Button>
						<Button
							variant="default"
							onClick={() => start('code')}
							disabled={!canStart || starting !== undefined}
							btnClasses="{startCardClasses} disabled:opacity-50 disabled:cursor-not-allowed"
						>
							<span class="flex w-full items-center gap-2">
								{#if starting?.mode === 'code'}
									<Loader2 size={18} class="text-primary animate-spin" />
								{:else}
									<Code size={18} class="text-primary" />
								{/if}
								<span class="text-sm font-semibold text-emphasis whitespace-nowrap">
									Start with code editor
								</span>
							</span>
							<span class="text-xs font-normal text-secondary">
								Write the app yourself in the editor.
							</span>
						</Button>
					</div>
					<p class="text-xs text-center {starting?.schema ? 'text-secondary' : 'text-hint'}">
						{starting?.schema
							? `Creating schema ${starting.schema}…`
							: 'You can always switch later.'}
					</p>
				</div>
			{:else}
				<div class="pt-6 flex items-center justify-end gap-3">
					{#if starting?.schema}
						<p class="mr-auto text-xs text-secondary">Creating schema {starting.schema}…</p>
					{/if}
					<Button
						variant="accent"
						unifiedSize="md"
						onclick={() => start('code')}
						loading={starting?.mode === 'code'}
						disabled={!canStart || starting !== undefined}
					>
						Create app
					</Button>
				</div>
			{/if}
		</div>
	</Modal>
{/if}

<RawAppDataTableDrawer
	bind:this={dataTableDrawer}
	offset={10000}
	existingRefs={preWhitelistedTables}
	roles={pickerRoles}
	onAdd={(refs, browsedRoles, roleChanged) => {
		preWhitelistedTables = [
			...preWhitelistedTables.filter((t) => !roleChanged.has(t.datatable)),
			...refs
		]
		preWhitelistedRoles = { ...preWhitelistedRoles, ...browsedRoles }
		// Tables picked while exploring are existing ones to use: show them.
		if (refs.length > 0) schemaMode = 'existing'
		if (selectedDatatable !== undefined && browsedRoles[selectedDatatable] !== undefined) {
			selectedRole = browsedRoles[selectedDatatable]
		}
	}}
/>
