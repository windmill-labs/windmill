<script lang="ts" module>
	/**
	 * The OAuth leg is a full-page redirect, so the wizard cannot stay mounted across it.
	 * What the user had chosen is parked here and picked back up by the settings page when
	 * Supabase redirects them home.
	 */
	const RESUME_KEY = 'datatable_wizard_resume'

	/** True while a wizard run is waiting on the Supabase redirect to come back. */
	export function hasParkedWizard(): boolean {
		return sessionStorage.getItem(RESUME_KEY) != null
	}

	export type WizardResume = { name: string; region: string; projectName: string }

	export function parkWizard(state: WizardResume) {
		sessionStorage.setItem(RESUME_KEY, JSON.stringify(state))
	}

	export function takeParkedWizard(): WizardResume | undefined {
		const raw = sessionStorage.getItem(RESUME_KEY)
		sessionStorage.removeItem(RESUME_KEY)
		if (!raw) return undefined
		try {
			return JSON.parse(raw)
		} catch {
			return undefined
		}
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte'
	import { Database, ArrowRight } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Stepper from '../common/stepper/Stepper.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import SupabaseIcon from '../icons/SupabaseIcon.svelte'
	import {
		OauthService,
		ResourceService,
		SettingService,
		VariableService,
		WorkspaceService
	} from '$lib/gen'
	import type { ListCustomInstanceDbsResponse, TestDataTableConnectionResponse } from '$lib/gen'
	import type { ResourceReturn } from 'runed'
	import type { ConfirmationModalHandle } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import SetupChecklist from '../wizards/SetupChecklist.svelte'
	import { instanceSetupSteps } from './instanceDbSteps'
	import SupabaseProjectStep, { type SupabasePick } from './SupabaseProjectStep.svelte'
	import { DEFAULT_SUPABASE_REGION, type SupabaseProject } from './supabaseProvisioning'
	import { useSupabaseOauth } from './supabaseOauth.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import { supabaseResourceValue } from './supabaseProvisioning'

	type Props = {
		opened: boolean
		existingNames: string[]
		/** Set when Supabase redirected the user back here mid-flow. */
		resume?: WizardResume | undefined
		onDone: () => void
		/** The instance database pool and its confirmation host are owned by the settings page,
		 * which already loads them for the rows; sharing them keeps one source of truth. */
		customInstanceDbs: ResourceReturn<ListCustomInstanceDbsResponse>
		confirmationModal: ConfirmationModalHandle
		defaultInstanceDbName: () => string
	}

	let {
		opened = $bindable(),
		existingNames,
		resume,
		onDone,
		customInstanceDbs,
		confirmationModal,
		defaultInstanceDbName
	}: Props = $props()

	type Provider = 'supabase' | 'existing' | 'instance'

	let step = $state(1)
	let provider: Provider | undefined = $state(undefined)
	let supaStep: ReturnType<typeof SupabaseProjectStep> | undefined = $state(undefined)
	let supaResult: SupabasePick | undefined = $state(undefined)

	let resourcePath: string | undefined = $state(undefined)
	let dataTableName = $state('main')

	let instanceDbName: string | undefined = $state(undefined)
	let instanceMode: 'existing' | 'create' = $state('create')
	let instanceSetupRunning = $state(false)
	// Selecting a database that is already healthy should not open with seven green ticks; the
	// checks are worth the room once they have something to report.
	let instanceSetupAttempted = $state(false)
	let preventClose = false
	let instanceStatus = $derived(
		instanceDbName ? customInstanceDbs.current?.[instanceDbName] : undefined
	)
	// Instance databases are pooled across features; DuckLake catalogs are tagged separately
	// and must not show up as somewhere to put a data table.
	let instanceDbs = $derived(
		Object.entries(customInstanceDbs.current ?? {})
			.filter(([_, db]) => db.tag === 'datatable')
			.map(([name, db]) => ({ name, db }))
	)

	function enterInstanceStep() {
		instanceMode = 'create'
		instanceDbName ??= defaultInstanceDbName()
	}

	function setInstanceMode(mode: 'existing' | 'create') {
		if (mode === instanceMode) return
		instanceMode = mode
		selectInstanceDb(mode === 'create' ? defaultInstanceDbName() : undefined)
	}

	function selectInstanceDb(name: string | undefined) {
		if (name === instanceDbName) return
		instanceDbName = name
		instanceSetupAttempted = false
	}

	function otherWorkspaces(name: string): string[] {
		return (customInstanceDbs.current?.[name]?.used_by_workspaces ?? []).filter(
			(w) => w !== $workspaceStore
		)
	}

	let checking = $state(false)
	let checkReport: TestDataTableConnectionResponse | undefined = $state(undefined)
	let checkError = $state('')
	let finishing = $state(false)
	// Which password the saved resource holds, so a retry only re-creates it when it changed.
	let savedPassword = $state('')

	function defaultProjectName(): string {
		return `windmill-${$workspaceStore ?? 'workspace'}`
	}

	function defaultTableName(): string {
		return existingNames.includes('main') ? `${$workspaceStore ?? 'data'}_datatable` : 'main'
	}

	let nameTaken = $derived(existingNames.includes(dataTableName.trim()))

	// Reopening after the Supabase redirect: drop the user back on the setup step with what
	// they had already chosen, so authorizing does not feel like starting over.
	let primed = $state(false)
	$effect(() => {
		if (!opened) {
			primed = false
			return
		}
		if (!primed) {
			primed = true
			// The name is derived on open, not at construction: the wizard is mounted for the
			// lifetime of the page, so the existing data tables are not known until then.
			dataTableName = resume?.name || defaultTableName()
		}
		if (resume) {
			provider = 'supabase'
			step = 2
			dataTableName = resume.name
		}
	})

	// The Supabase branch goes through the instance's `supabase_wizard` OAuth client; where a
	// superadmin has not configured one, the connect endpoint dead-ends, so do not offer it.
	let supabaseAvailable = $state(false)
	$effect(() => {
		if (!opened) return
		OauthService.listOauthConnects()
			.then((cs) => (supabaseAvailable = (cs ?? []).some((c) => c.name === 'supabase_wizard')))
			.catch(() => {})
	})

	const SUPABASE_SIGNUP_URL = 'https://supabase.com/dashboard/sign-up'

	// The step component drives its own authorization once the user is on it, but signing in is
	// the whole of that step when it has not happened yet -- so Continue does it directly rather
	// than spending a screen telling the user what the next button will do.
	const supaOauth = useSupabaseOauth({
		onPopupBlocked: () =>
			parkWizard({
				name: dataTableName,
				region: DEFAULT_SUPABASE_REGION,
				projectName: defaultProjectName()
			})
	})

	/**
	 * The variable and the resource share a path, so both have to be free. Reusing a taken one
	 * fails *after* the Supabase project exists, stranding a database whose generated password
	 * is then unrecoverable -- so the path is resolved before anything is created.
	 */
	async function freePath(name: string): Promise<string> {
		const base = `u/${$userStore?.username ?? 'admin'}/${name.replace(/[^\w]/g, '_')}`
		for (let i = 0; i < 50; i++) {
			const candidate = i === 0 ? base : `${base}_${i + 1}`
			const [varTaken, resTaken] = await Promise.all([
				VariableService.existsVariable({ workspace: $workspaceStore!, path: candidate }),
				ResourceService.existsResource({ workspace: $workspaceStore!, path: candidate })
			])
			if (!varTaken && !resTaken) return candidate
		}
		throw new Error(`Could not find a free path for ${base}`)
	}

	/** Creates the secret variable + postgres resource for a Supabase project. */
	async function saveSupabaseResource(project: SupabaseProject, password: string) {
		const path = await freePath(project.name)
		await VariableService.createVariable({
			workspace: $workspaceStore!,
			requestBody: {
				path,
				value: password,
				is_secret: true,
				description: `Password for the ${project.name} Supabase database`,
				is_oauth: false
			}
		})
		await ResourceService.createResource({
			workspace: $workspaceStore!,
			requestBody: {
				resource_type: 'postgresql',
				path,
				value: supabaseResourceValue(project, path),
				description: `Supabase project ${project.name}`
			}
		})
		return path
	}

	/**
	 * The step component hands back a project and the password Windmill knows for it; from
	 * here it is the same work either way. Credentials are saved before the check because
	 * Supabase never hands a generated password back, and only re-saved when it changed, so a
	 * retry after fixing GRANTs does not leave another orphan variable behind.
	 */
	async function adoptSupabaseResult(pick: SupabasePick) {
		checking = true
		checkError = ''
		try {
			if (!resourcePath || savedPassword !== pick.password) {
				resourcePath = await saveSupabaseResource(pick.project, pick.password)
				savedPassword = pick.password
			}
			await runCheck()
		} catch (err) {
			checkError = String(err)
			sendUserToast(String(err), true)
		} finally {
			checking = false
		}
	}

	$effect(() => {
		if (supaResult) adoptSupabaseResult(supaResult)
	})

	async function runCheck() {
		if (!resourcePath) return
		checking = true
		checkError = ''
		checkReport = undefined
		try {
			checkReport = await WorkspaceService.testDataTableResourceConnection({
				workspace: $workspaceStore!,
				resourcePath
			})
		} catch (err: any) {
			checkError = err?.body ?? err?.message ?? String(err)
		} finally {
			checking = false
		}
	}

	async function checkAndContinue() {
		await runCheck()
		if (canAdvanceFromSetup()) step = 3
	}

	// A check result describes one database, so it must not outlive the choice that produced
	// it: switching provider or Supabase mode has to leave the new tab with a blank slate.
	function clearCheck() {
		checkReport = undefined
		checkError = ''
	}

	function selectProvider(key: Provider) {
		if (key === provider) return
		clearCheck()
		provider = key
	}

	/**
	 * setup_custom_instance_db both creates the database and re-runs every check, so one call
	 * serves the first attempt and every retry. Creation is destructive enough to confirm, but
	 * only the first time: once the database exists the call is a pure re-check.
	 */
	async function setupInstanceDb() {
		if (!instanceDbName) return
		const exists =
			instanceStatus?.logs.created_database === 'OK' ||
			instanceStatus?.logs.created_database === 'SKIP'
		if (!exists) {
			// The confirmation dialog takes focus from this modal, which Modal2 reads as a
			// dismissal -- without the guard the wizard closes the moment setup is confirmed.
			preventClose = true
			const confirmed = await confirmationModal.ask({
				title: 'Confirm setup',
				children: `This will create a new database ${instanceDbName} in the Windmill PostgreSQL instance`,
				confirmationText: 'Setup database'
			})
			preventClose = false
			if (!confirmed) return
		}
		instanceSetupRunning = true
		instanceSetupAttempted = true
		try {
			const result = await SettingService.setupCustomInstanceDb({
				name: instanceDbName,
				requestBody: { tag: 'datatable' }
			})
			await customInstanceDbs.refetch()
			// Stay on the step even when everything passed: the checks are the point of this
			// screen, and skipping past them hides what was just done to the database.
			if (!result.success) sendUserToast(result.error ?? 'Setup failed', true)
		} catch (err) {
			sendUserToast(`Could not set up ${instanceDbName}: ${err}`, true)
		} finally {
			instanceSetupRunning = false
		}
	}

	function checkPassed(): boolean {
		return !!checkReport && checkReport.can_create_table && !checkError
	}

	function canAdvanceFromSetup(): boolean {
		return checkPassed()
	}

	async function finish() {
		if (provider === 'instance' ? !instanceDbName : !resourcePath) return
		finishing = true
		try {
			// editDataTableConfig replaces the whole map, so the existing entries have to be
			// read back and merged or they are silently dropped.
			const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
			const datatables: Record<string, any> = { ...(settings.datatable?.datatables ?? {}) }
			datatables[dataTableName] =
				provider === 'instance'
					? { database: { resource_type: 'instance', resource_path: instanceDbName } }
					: { database: { resource_type: 'postgresql', resource_path: resourcePath } }
			await WorkspaceService.editDataTableConfig({
				workspace: $workspaceStore!,
				requestBody: { settings: { datatables }, renames: [], deleted_datatables: [] }
			})
			sendUserToast(`${dataTableName} is ready`)
			opened = false
			reset()
			onDone()
		} catch (err) {
			sendUserToast(`Could not save the data table: ${err}`, true)
		} finally {
			finishing = false
		}
	}

	function reset() {
		step = 1
		provider = undefined
		// The Supabase step keeps its own state; it is remounted with the modal, so closing is
		// all it takes to clear it.
		supaResult = undefined
		resourcePath = undefined
		instanceDbName = undefined
		instanceMode = 'create'
		instanceSetupAttempted = false
		checkReport = undefined
		checkError = ''
		savedPassword = ''
		dataTableName = defaultTableName()
	}

	// The single primary action. Its label says what it is about to do, and doing it is what
	// moves the wizard on -- there is no separate "run the check" or "create" button.
	let primary = $derived.by(() => {
		if (step === 1) {
			if (provider === 'supabase' && !supaOauth.authed)
				return {
					label: 'Connect to Supabase',
					disabled: false,
					act: () => {
						supaOauth.connect()
						step = 2
					}
				}
			return {
				label: 'Continue',
				disabled: !provider,
				act: () => {
					if (provider === 'instance') enterInstanceStep()
					step = 2
				}
			}
		}
		if (step === 2) {
			if (provider === 'supabase') {
				// Picking or creating the project is the step component's business; this only
				// takes over once it has handed one back and the credentials have been checked.
				if (!supaResult) return supaStep?.getAction() ?? { label: 'Continue', disabled: true }
				if (checking) return { label: 'Checking', disabled: true, busy: true }
				if (!checkPassed())
					return {
						label: 'Try again',
						disabled: false,
						act: () => adoptSupabaseResult(supaResult!)
					}
				return { label: 'Continue', disabled: false, act: () => (step = 3) }
			}
			if (provider === 'instance') {
				if (instanceSetupRunning) return { label: 'Setting it up', disabled: true, busy: true }
				if (instanceStatus?.success)
					return { label: 'Continue', disabled: false, act: () => (step = 3) }
				return {
					label: instanceStatus ? 'Try again' : 'Set up database',
					disabled: !instanceDbName,
					act: setupInstanceDb
				}
			}
			if (checking) return { label: 'Checking', disabled: true, busy: true }
			return {
				label: checkReport && !checkPassed() ? 'Try again' : 'Continue',
				disabled: !resourcePath,
				act: checkAndContinue
			}
		}
		return {
			label: 'Finish',
			disabled: !dataTableName.trim() || nameTaken,
			busy: finishing,
			act: finish
		}
	})

	const STEPS = ['Choose a database', 'Set it up', 'Name it']
</script>

<Modal2
	bind:isOpen={
		() => opened,
		(v) => {
			if (!v && preventClose) return
			opened = v
			if (!v) reset()
		}
	}
	target="#content"
	title="Add a database"
	contentClasses="flex flex-col"
	fixedWidth="md"
	fixedHeight="md"
>
	<div class="flex h-full flex-col gap-4">
		<Stepper tabs={STEPS} selectedIndex={step - 1} maxReachedIndex={step - 1} small />

		<div class="flex-1 flex flex-col min-h-0">
			<div class="flex-1 overflow-y-auto flex flex-col gap-3">
				{#if step === 1}
					<Alert type="info" size="xs" bgClass="border-0" title="">
						A data table runs on a database of your own. It stays yours &mdash; you can take it with
						you at any time.
					</Alert>
					<div class="flex flex-col gap-2">
						{#if $isCustomInstanceDbEnabled}
							{#snippet instanceIcon()}
								<Database size={18} class="text-secondary" />
							{/snippet}
							{@render providerCard(
								'instance',
								instanceIcon,
								'Windmill database',
								'Managed for you. No setup.'
							)}
						{/if}
						{#if supabaseAvailable}
							{#snippet supabaseIcon()}
								<SupabaseIcon height="18px" width="18px" />
							{/snippet}
							{@render providerCard(
								'supabase',
								supabaseIcon,
								'Supabase',
								'Connect a project you already have, or let Windmill create one.'
							)}
						{/if}
						{#snippet ownIcon()}
							<Database size={18} class="text-secondary" />
						{/snippet}
						{@render providerCard(
							'existing',
							ownIcon,
							'Your own database',
							'Use a database resource, or add one with its connection string.'
						)}
					</div>
				{:else if step === 2}
					{#if provider === 'supabase'}
						<SupabaseProjectStep
							bind:this={supaStep}
							bind:result={supaResult}
							defaultProjectName={defaultProjectName()}
							resume={resume
								? { region: resume.region, projectName: resume.projectName }
								: undefined}
							onPopupBlocked={(s) => parkWizard({ name: dataTableName, ...s })}
							hostBusy={checking}
							extraSteps={supaResult
								? [
										{
											title: 'Checking Windmill can store data',
											status: checkPassed()
												? 'done'
												: checking
													? 'running'
													: checkError || checkReport
														? 'failed'
														: 'pending'
										}
									]
								: undefined}
						/>
					{:else if provider === 'instance'}
						{#if instanceDbs.length}
							<ToggleButtonGroup bind:selected={() => instanceMode, (v) => setInstanceMode(v)}>
								{#snippet children({ item })}
									<ToggleButton value="existing" label="Use an existing one" {item} small />
									<ToggleButton value="create" label="Create a new one" {item} small />
								{/snippet}
							</ToggleButtonGroup>
						{/if}
						{#if instanceMode === 'existing'}
							<!-- Above the list, not under it: the list scrolls, and a warning about sharing
							another workspace's data is worthless if the user has to scroll to reach it. -->
							{#if instanceDbName && otherWorkspaces(instanceDbName).length}
								{@const shared = otherWorkspaces(instanceDbName)}
								<Alert type="warning" size="xs" bgClass="border-0" title="">
									This database is also used by workspace{shared.length > 1 ? 's' : ''}
									<span class="font-semibold">{shared.join(', ')}</span>. Any data written here will
									be shared with {shared.length > 1 ? 'them' : 'it'}.
								</Alert>
							{/if}
							<!-- The list takes the leftover height and is the only thing that scrolls, so the
							toggle and the sharing warning stay put and the step itself never needs a second
							scrollbar. A pooled instance can hold dozens of databases. -->
							<div class="flex flex-col gap-2 overflow-y-auto flex-1 min-h-24 pr-1">
								{#each instanceDbs as { name, db } (name)}
									{@const selected = instanceDbName === name}
									{@const shared = otherWorkspaces(name)}
									<button
										class="text-left border rounded-md p-3 flex gap-3 items-start transition-colors {selected
											? 'border-border-selected/50 bg-surface-accent-selected'
											: 'border-border-light hover:bg-surface-hover'}"
										onclick={() => selectInstanceDb(name)}
									>
										<span class="mt-0.5 shrink-0"
											><Database size={18} class="text-secondary" /></span
										>
										<span class="flex flex-col gap-0.5 min-w-0">
											<span class="text-xs font-medium {selected ? 'text-accent' : 'text-emphasis'}"
												>{name}</span
											>
											<span class="text-xs text-secondary font-normal">
												{db.success ? 'Ready' : 'Needs setup'}{shared.length
													? ` · shared with ${shared.length} other workspace${shared.length > 1 ? 's' : ''}`
													: ''}
											</span>
										</span>
									</button>
								{/each}
							</div>
						{:else}
							<div>
								<span class="text-xs font-semibold text-emphasis">Database name</span>
								<TextInput
									bind:value={() => instanceDbName ?? '', (v) => (instanceDbName = v)}
									inputProps={{ placeholder: defaultInstanceDbName() }}
								/>
								<p class="text-2xs text-secondary mt-1">
									Created in the Windmill PostgreSQL instance. Windmill manages its credentials.
								</p>
							</div>
						{/if}
						{#if instanceSetupRunning || (instanceStatus && (instanceSetupAttempted || instanceStatus.error))}
							<SetupChecklist
								steps={instanceSetupSteps(
									instanceDbName ?? '',
									instanceStatus,
									instanceSetupRunning
								)}
							/>
						{/if}
					{:else}
						<div>
							<span class="text-xs font-semibold text-emphasis">Database</span>
							<ResourcePicker bind:value={resourcePath} resourceType="postgresql" />
							<p class="text-2xs text-secondary mt-1">
								Pick one, or add a new one with its connection string.
							</p>
						</div>
					{/if}

					{@render checkResult()}
				{:else}
					<div>
						<span class="text-xs font-semibold text-emphasis">Name this data table</span>
						<TextInput bind:value={dataTableName} inputProps={{ placeholder: 'main' }} />
						<p class="text-2xs text-secondary mt-1">
							{#if nameTaken}
								<span class="text-red-500"
									>A data table called {dataTableName.trim()} already exists in this workspace.</span
								>
							{:else}
								This is how your scripts will refer to it. <span class="font-mono">main</span> is used
								by default when a script does not name one.
							{/if}
						</p>
					</div>
					<Alert type="info" size="xs" bgClass="border-0" title="">
						Once you finish, <span class="font-mono">{dataTableName}</span> is ready to use from any
						script in this workspace.
					</Alert>
				{/if}
			</div>

			<div class="flex flex-col gap-1 pt-3">
				<div class="flex justify-between items-center gap-2">
					<div>
						{#if step > 1 && !primary.busy}
							<Button size="xs" variant="default" onClick={() => (step = step - 1)}>Back</Button>
						{/if}
					</div>
					<Button
						size="sm"
						variant="accent"
						disabled={primary.disabled}
						loading={primary.busy}
						endIcon={primary.busy ? undefined : { icon: ArrowRight }}
						onClick={() => primary.act?.()}
					>
						{primary.label}
					</Button>
				</div>
				{#if provider === 'supabase' && !supaOauth.authed}
					<p class="text-2xs text-secondary text-right">
						If you do not have a Supabase account you can <a
							href={SUPABASE_SIGNUP_URL}
							target="_blank"
							rel="noreferrer"
							class="text-blue-500 hover:underline">create one for free</a
						>.
					</p>
				{/if}
			</div>
		</div>
	</div>
</Modal2>

{#snippet providerCard(key: Provider, icon: Snippet, title: string, subtitle: string)}
	{@const selected = provider === key}
	<button
		class="text-left border rounded-md p-3 flex gap-3 items-start transition-colors {selected
			? 'border-border-selected/50 bg-surface-accent-selected'
			: 'border-border-light hover:bg-surface-hover'}"
		onclick={() => selectProvider(key)}
	>
		<span class="mt-0.5 shrink-0">{@render icon()}</span>
		<span class="flex flex-col gap-0.5 min-w-0">
			<span class="text-xs font-medium {selected ? 'text-accent' : 'text-emphasis'}">{title}</span>
			<span class="text-xs text-secondary font-normal">{subtitle}</span>
		</span>
	</button>
{/snippet}

{#snippet checkResult()}
	{#if checkError}
		<Alert type="error" size="xs" bgClass="border-0" title="Could not connect">{checkError}</Alert>
	{:else if checkReport && !checkReport.can_create_table}
		<Alert type="warning" size="xs" bgClass="border-0" title="Windmill cannot store data here">
			<div class="flex flex-col gap-2">
				<div>Your database user is not allowed to create tables. Run this, then try again:</div>
				<pre class="whitespace-pre-wrap select-all text-2xs"
					>{checkReport.suggested_grants.map((g) => `${g};`).join('\n')}</pre
				>
			</div>
		</Alert>
	{/if}
{/snippet}
