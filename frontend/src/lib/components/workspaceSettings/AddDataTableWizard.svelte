<script lang="ts" module>
	/**
	 * Where popups are blocked the Supabase leg falls back to a full-page redirect, which
	 * unmounts the wizard. What the user had chosen is parked here and picked back up by the
	 * settings page when Supabase sends them home.
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
	import Select from '../select/Select.svelte'
	import SupabaseIcon from '../icons/SupabaseIcon.svelte'
	import { FolderService, OauthService, WorkspaceService } from '$lib/gen'
	import type { ListCustomInstanceDbsResponse } from '$lib/gen'
	import type { ResourceReturn } from 'runed'
	import type { ConfirmationModalHandle } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import SetupChecklist, { type SetupStep } from '../wizards/SetupChecklist.svelte'
	import SupabaseProjectStep from './SupabaseProjectStep.svelte'
	import DataTableConnectionReport from './DataTableConnectionReport.svelte'
	import { useSupabaseOauth } from './supabaseOauth.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import { parsePostgresConnectionString } from '$lib/utils/postgresConnectionString'
	import {
		clearProbe,
		intentComplete,
		newWizardState,
		originOf,
		planSteps,
		probeValue,
		resourcePathOf,
		runSetup,
		type Provider,
		type WizardState
	} from './addDataTableModel'
	import {
		getSupabasePooler,
		projectRef,
		supabaseResourceValue,
		DEFAULT_SUPABASE_REGION
	} from './supabaseProvisioning'

	type Props = {
		opened: boolean
		existingNames: string[]
		/** Every data table already configured, so the review step can warn about sharing one. */
		existingDataTables: {
			name: string
			resourcePath: string | undefined
			projectRef: string | undefined
		}[]
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
		existingDataTables,
		resume,
		onDone,
		customInstanceDbs,
		confirmationModal,
		defaultInstanceDbName
	}: Props = $props()

	const STEPS = ['Choose a database', 'Set it up', 'Review']

	let wiz: WizardState = $state(
		newWizardState({ name: 'main', projectName: 'windmill-data', folder: '' })
	)
	let preventClose = false

	/** Set once Finish has been pressed; the modal shows the checklist from then on. */
	let run: { steps: SetupStep[]; running: boolean; result?: Awaited<ReturnType<typeof runSetup>> } =
		$state({ steps: [], running: false })
	let rowCreated = $state(false)

	let folders: string[] = $state([])

	function defaultProjectName(): string {
		return `windmill-${$workspaceStore ?? 'workspace'}`
	}

	function defaultTableName(): string {
		return existingNames.includes('main') ? `${$workspaceStore ?? 'data'}_datatable` : 'main'
	}

	function defaultFolder(): string {
		// The first folder this admin can write to, so the resource lands somewhere the team
		// can find and repair. A workspace with no folders falls back to the personal space.
		return folders.length ? `f/${folders[0]}` : `u/${$userStore?.username ?? 'admin'}`
	}

	let nameTaken = $derived(existingNames.includes(wiz.review.name.trim()))
	let resourcePath = $derived(resourcePathOf(wiz))

	/**
	 * Another data table on the same database. They would share one `_wm_migrations` table,
	 * so each would see the other's migrations as already applied.
	 */
	let sharesDatabaseWith = $derived.by(() => {
		const ref = wiz.supabase.project ? projectRef(wiz.supabase.project) : undefined
		return existingDataTables.find(
			(d) =>
				(wiz.provider === 'resource' &&
					wiz.own.mode === 'pick' &&
					!!wiz.own.resourcePath &&
					d.resourcePath === wiz.own.resourcePath) ||
				(wiz.provider === 'supabase' && !!ref && d.projectRef === ref)
		)
	})

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
			reset()
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
		FolderService.listFolderNames({ workspace: $workspaceStore! })
			.then(
				(f) => (folders = f.filter((x) => !['app_groups', 'app_custom', 'app_themes'].includes(x)))
			)
			.catch(() => {})
	})

	const SUPABASE_SIGNUP_URL = 'https://supabase.com/dashboard/sign-up'

	const supaOauth = useSupabaseOauth({
		onPopupBlocked: () =>
			parkWizard({
				name: wiz.review.name,
				region: wiz.supabase.region,
				projectName: wiz.supabase.projectName
			})
	})

	function reset() {
		wiz = newWizardState({
			name: resume?.name || defaultTableName(),
			projectName: resume?.projectName || defaultProjectName(),
			folder: defaultFolder()
		})
		wiz.supabase.region = resume?.region ?? DEFAULT_SUPABASE_REGION
		run = { steps: [], running: false }
		rowCreated = false
		if (resume) {
			wiz.provider = 'supabase'
			wiz.step = 2
		}
	}

	function selectProvider(key: Provider) {
		if (key === wiz.provider) return
		wiz.provider = key
		clearProbe(wiz)
		if (key === 'instance') wiz.instance.dbName ??= defaultInstanceDbName()
	}

	function suggestedResourceName(): string {
		const base =
			wiz.provider === 'supabase'
				? wiz.supabase.mode === 'create'
					? wiz.supabase.projectName
					: (wiz.supabase.project?.name ?? 'supabase')
				: wiz.review.name
		return `${wiz.provider === 'supabase' ? 'supabase_' : 'pg_'}${base
			.trim()
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '_')
			.replace(/^_|_$/g, '')}`
	}

	function enterReview() {
		if (!wiz.review.resourceName) wiz.review.resourceName = suggestedResourceName()
		wiz.step = 3
	}

	/**
	 * Proves the typed connection works before the wizard writes anything. The Supabase branch
	 * has to read the pooler first: which host a project answers on is assigned by Supabase,
	 * so the value under test has to be the value that will be saved.
	 */
	async function probe() {
		wiz.probe = { checking: true, report: undefined, error: undefined }
		try {
			let value = probeValue(wiz)
			if (!value && wiz.provider === 'supabase' && wiz.supabase.project) {
				const pooler =
					wiz.supabase.connectionMode === 'session'
						? await getSupabasePooler(supaOauth.token!, projectRef(wiz.supabase.project))
						: undefined
				value = {
					...supabaseResourceValue(wiz.supabase.project, '', {
						mode: wiz.supabase.connectionMode,
						pooler
					}),
					password: wiz.supabase.password
				}
			}
			if (!value && wiz.provider === 'resource' && wiz.own.resourcePath) {
				const report = await WorkspaceService.testDataTableResourceConnection({
					workspace: $workspaceStore!,
					resourcePath: wiz.own.resourcePath
				})
				wiz.probe = { checking: false, report, error: undefined }
				return
			}
			if (!value) {
				wiz.probe = { checking: false, report: undefined, error: undefined }
				return
			}
			const report = await WorkspaceService.testDataTableConnectionValue({
				workspace: $workspaceStore!,
				requestBody: value
			})
			wiz.probe = { checking: false, report, error: undefined }
		} catch (err: any) {
			wiz.probe = {
				checking: false,
				report: undefined,
				error: err?.body ?? err?.message ?? String(err)
			}
		}
	}

	/** True when step 2 has nothing to prove before Finish, so Continue is the only action. */
	let probeable = $derived(
		wiz.provider === 'resource' || (wiz.provider === 'supabase' && wiz.supabase.mode === 'existing')
	)
	let probePassed = $derived(!!wiz.probe.report?.can_create_table && !wiz.probe.error)

	async function finish() {
		run = { steps: planSteps(wiz), running: true }
		preventClose = true
		const result = await runSetup(wiz, {
			workspace: $workspaceStore!,
			username: $userStore?.username ?? 'admin',
			supabaseToken: supaOauth.token,
			confirmInstanceSetup: async (dbName) =>
				await confirmationModal.ask({
					title: 'Confirm setup',
					children: `This will create a new database ${dbName} in the Windmill PostgreSQL instance`,
					confirmationText: 'Setup database'
				}),
			onInstanceDbsChanged: async () => {
				await customInstanceDbs.refetch()
			},
			onProgress: (steps) => (run.steps = steps),
			onRowCreated: () => (rowCreated = true)
		})
		preventClose = false
		run = { ...run, running: false, result }
		onDone()
	}

	function close() {
		opened = false
		if (rowCreated && !run.result?.ok) {
			sendUserToast(
				`${wiz.review.name} was saved as incomplete. Open it to finish setting it up.`,
				true
			)
		}
	}

	// The single primary action. Its label says what it is about to do, and doing it is what
	// moves the wizard on.
	let primary = $derived.by(() => {
		if (run.steps.length) {
			if (run.running) return { label: 'Setting things up', disabled: true, busy: true }
			if (run.result?.ok) return { label: 'Done', disabled: false, act: close }
			return { label: 'Try again', disabled: false, act: finish }
		}
		if (wiz.step === 1) {
			if (wiz.provider === 'supabase' && !supaOauth.authed)
				return {
					label: 'Connect to Supabase',
					disabled: false,
					busy: supaOauth.pending,
					act: () => {
						supaOauth.connect()
						wiz.step = 2
					}
				}
			return {
				label: 'Continue',
				disabled: !wiz.provider,
				act: () => (wiz.step = 2)
			}
		}
		if (wiz.step === 2) {
			if (wiz.probe.checking) return { label: 'Checking', disabled: true, busy: true }
			if (probeable && !probePassed)
				return {
					label: wiz.probe.error || wiz.probe.report ? 'Try again' : 'Test connection',
					disabled: !intentComplete(wiz),
					act: probe
				}
			return {
				label: 'Continue',
				disabled: !intentComplete(wiz),
				act: enterReview
			}
		}
		return {
			label:
				wiz.provider === 'supabase' && wiz.supabase.mode === 'create'
					? 'Create project and data table'
					: 'Create data table',
			disabled: !wiz.review.name.trim() || nameTaken || !wiz.review.resourceName.trim(),
			act: finish
		}
	})

	let folderItems = $derived([
		{
			label: `Only me (u/${$userStore?.username ?? 'admin'})`,
			value: `u/${$userStore?.username ?? 'admin'}`
		},
		...folders.map((f) => ({ label: `Anyone with access to f/${f}`, value: `f/${f}` }))
	])

	/** The review step only mints a resource when the wizard is the one creating it. */
	let mintsResource = $derived(
		wiz.provider === 'supabase' || (wiz.provider === 'resource' && wiz.own.mode === 'connstr')
	)
</script>

<Modal2
	bind:isOpen={
		() => opened,
		(v) => {
			if (!v && preventClose) return
			if (!v) close()
			else opened = v
		}
	}
	target="#content"
	title="Add a database"
	contentClasses="flex flex-col"
	fixedWidth="md"
	fixedHeight="lg"
>
	<div class="flex h-full flex-col gap-4">
		<Stepper tabs={STEPS} selectedIndex={wiz.step - 1} maxReachedIndex={wiz.step - 1} small />

		<div class="flex-1 flex flex-col min-h-0">
			<div class="flex-1 overflow-y-auto flex flex-col gap-3">
				{#if run.steps.length}
					<SetupChecklist steps={run.steps} />
					{#if run.running}
						<p class="text-xs text-secondary">
							You can leave this open. {#if rowCreated}<button
									class="text-blue-500 hover:underline"
									onclick={close}>Continue in the background</button
								> — the data table is already saved, and shows as incomplete until this finishes.{/if}
						</p>
					{/if}
					{#if run.result}
						<DataTableConnectionReport
							name={wiz.review.name}
							report={run.result.report}
							error={run.result.error}
							bgClass="border-0"
						/>
					{/if}
				{:else if wiz.step === 1}
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
								'Windmill creates and manages a database on this instance.'
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
								'Create a project, or connect one you already have. Signing in is required, and connecting an existing project needs its database password.'
							)}
						{/if}
						{#snippet ownIcon()}
							<Database size={18} class="text-secondary" />
						{/snippet}
						{@render providerCard(
							'resource',
							ownIcon,
							'Your own database',
							'Any Postgres — RDS, Neon, self-hosted. Pick a resource, or paste a connection string.'
						)}
					</div>
				{:else if wiz.step === 2}
					{#if wiz.provider === 'supabase'}
						{#if !supaOauth.authed}
							<Alert type="info" size="xs" bgClass="border-0" title="">
								{#if supaOauth.pending}
									Sign in and approve Windmill in the Supabase window, then come back here.
								{:else}
									Windmill needs your approval on Supabase to see your databases.
								{/if}
							</Alert>
						{:else}
							<SupabaseProjectStep
								bind:intent={wiz.supabase}
								token={supaOauth.token!}
								onIntentChange={() => clearProbe(wiz)}
							/>
						{/if}
					{:else if wiz.provider === 'instance'}
						{@render instanceStep()}
					{:else}
						{@render ownStep()}
					{/if}

					<DataTableConnectionReport
						name={wiz.review.name}
						report={wiz.probe.report}
						error={wiz.probe.error}
						bgClass="border-0"
					/>
				{:else}
					{@render reviewStep()}
				{/if}
			</div>

			<div class="flex flex-col gap-1 pt-3">
				<div class="flex justify-between items-center gap-2">
					<div>
						{#if wiz.step > 1 && !run.steps.length}
							<Button
								size="xs"
								variant="default"
								onClick={() => (wiz.step = wiz.step === 3 ? 2 : 1)}
							>
								Back
							</Button>
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
				{#if wiz.provider === 'supabase' && !supaOauth.authed}
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
	{@const selected = wiz.provider === key}
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

{#snippet instanceStep()}
	{@const instanceDbs = Object.entries(customInstanceDbs.current ?? {})
		.filter(([_, db]) => db.tag === 'datatable')
		.map(([name, db]) => ({ name, db }))}
	{#if instanceDbs.length}
		<ToggleButtonGroup
			bind:selected={
				() => wiz.instance.mode,
				(v) => {
					wiz.instance.mode = v
					wiz.instance.dbName = v === 'create' ? defaultInstanceDbName() : undefined
				}
			}
		>
			{#snippet children({ item })}
				<ToggleButton value="existing" label="Use an existing one" {item} small />
				<ToggleButton value="create" label="Create a new one" {item} small />
			{/snippet}
		</ToggleButtonGroup>
	{/if}
	{#if wiz.instance.mode === 'existing'}
		{@const shared = (
			customInstanceDbs.current?.[wiz.instance.dbName ?? '']?.used_by_workspaces ?? []
		).filter((w) => w !== $workspaceStore)}
		<!-- Above the list, not under it: the list scrolls, and a warning about sharing another
		workspace's data is worthless if the user has to scroll to reach it. -->
		{#if shared.length}
			<Alert type="warning" size="xs" bgClass="border-0" title="">
				This database is also used by workspace{shared.length > 1 ? 's' : ''}
				<span class="font-semibold">{shared.join(', ')}</span>. Any data written here will be shared
				with {shared.length > 1 ? 'them' : 'it'}.
			</Alert>
		{/if}
		<div class="flex flex-col gap-2 overflow-y-auto flex-1 min-h-24 pr-1">
			{#each instanceDbs as { name, db } (name)}
				{@const selected = wiz.instance.dbName === name}
				{@const others = (db.used_by_workspaces ?? []).filter((w) => w !== $workspaceStore)}
				<button
					class="text-left border rounded-md p-3 flex gap-3 items-start transition-colors {selected
						? 'border-border-selected/50 bg-surface-accent-selected'
						: 'border-border-light hover:bg-surface-hover'}"
					onclick={() => (wiz.instance.dbName = name)}
				>
					<span class="mt-0.5 shrink-0"><Database size={18} class="text-secondary" /></span>
					<span class="flex flex-col gap-0.5 min-w-0">
						<span class="text-xs font-medium {selected ? 'text-accent' : 'text-emphasis'}"
							>{name}</span
						>
						<span class="text-xs text-secondary font-normal">
							{db.success ? 'Ready' : 'Needs setup'}{others.length
								? ` · shared with ${others.length} other workspace${others.length > 1 ? 's' : ''}`
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
				bind:value={() => wiz.instance.dbName ?? '', (v) => (wiz.instance.dbName = v)}
				inputProps={{ placeholder: defaultInstanceDbName() }}
			/>
			<p class="text-2xs text-secondary mt-1">
				Created in the Windmill PostgreSQL instance when you finish. Windmill manages its
				credentials.
			</p>
		</div>
	{/if}
{/snippet}

{#snippet ownStep()}
	<ToggleButtonGroup
		bind:selected={
			() => wiz.own.mode,
			(v) => {
				wiz.own.mode = v
				clearProbe(wiz)
			}
		}
	>
		{#snippet children({ item })}
			<ToggleButton value="pick" label="Use a resource" {item} small />
			<ToggleButton value="connstr" label="Add a connection string" {item} small />
		{/snippet}
	</ToggleButtonGroup>
	{#if wiz.own.mode === 'pick'}
		<div>
			<span class="text-xs font-semibold text-emphasis">Database</span>
			<ResourcePicker
				bind:value={
					() => wiz.own.resourcePath,
					(v) => {
						if (v !== wiz.own.resourcePath) clearProbe(wiz)
						wiz.own.resourcePath = v
					}
				}
				resourceType="postgresql"
			/>
		</div>
	{:else}
		<div>
			<span class="text-xs font-semibold text-emphasis">Connection string</span>
			<TextInput
				bind:value={
					() => wiz.own.connectionString,
					(v) => {
						wiz.own.connectionString = v
						clearProbe(wiz)
					}
				}
				inputProps={{ placeholder: 'postgres://user:password@host:5432/database' }}
			/>
			<p class="text-2xs text-secondary mt-1">
				{#if wiz.own.connectionString && !parsePostgresConnectionString(wiz.own.connectionString)}
					<span class="text-red-500">That is not a Postgres connection string.</span>
				{:else}
					Saved as a Postgres resource in this workspace when you finish.
				{/if}
			</p>
		</div>
	{/if}
{/snippet}

{#snippet reviewStep()}
	<div>
		<span class="text-xs font-semibold text-emphasis">Data table name</span>
		<TextInput bind:value={wiz.review.name} inputProps={{ placeholder: 'main' }} />
		<p class="text-2xs text-secondary mt-1">
			{#if nameTaken}
				<span class="text-red-500"
					>A data table called {wiz.review.name.trim()} already exists in this workspace.</span
				>
			{:else}
				This is how your scripts will refer to it. <span class="font-mono">main</span> is used by default
				when a script does not name one.
			{/if}
		</p>
	</div>

	{#if wiz.provider === 'supabase'}
		<dl
			class="grid grid-cols-[9rem_1fr] gap-y-1 gap-x-3 text-xs border rounded-md p-3 border-border-light"
		>
			<dt class="text-secondary">
				{wiz.supabase.mode === 'create' ? 'New Supabase project' : 'Supabase project'}
			</dt>
			<dd class="text-emphasis">
				{wiz.supabase.mode === 'create'
					? wiz.supabase.projectName
					: (wiz.supabase.project?.name ?? '')}
			</dd>
			<dt class="text-secondary">Organization</dt>
			<dd class="text-emphasis">{originOf(wiz, $userStore?.username ?? '').org ?? '—'}</dd>
			<dt class="text-secondary">Region</dt>
			<dd class="text-emphasis">{originOf(wiz, $userStore?.username ?? '').region ?? '—'}</dd>
			<dt class="text-secondary">Connection</dt>
			<dd class="text-emphasis">
				{wiz.supabase.connectionMode === 'session' ? 'Session pooler' : 'Direct (IPv6)'}
			</dd>
		</dl>
	{:else if wiz.provider === 'instance'}
		<dl
			class="grid grid-cols-[9rem_1fr] gap-y-1 gap-x-3 text-xs border rounded-md p-3 border-border-light"
		>
			<dt class="text-secondary">Windmill database</dt>
			<dd class="text-emphasis font-mono">{wiz.instance.dbName}</dd>
		</dl>
	{:else if wiz.own.mode === 'pick'}
		<dl
			class="grid grid-cols-[9rem_1fr] gap-y-1 gap-x-3 text-xs border rounded-md p-3 border-border-light"
		>
			<dt class="text-secondary">Postgres resource</dt>
			<dd class="text-emphasis font-mono">{wiz.own.resourcePath}</dd>
		</dl>
	{/if}

	{#if mintsResource}
		<div class="grid grid-cols-2 gap-2">
			<div>
				<span class="text-xs font-semibold text-emphasis">Who can use this database</span>
				<Select items={folderItems} bind:value={wiz.review.folder} placeholder="Select" />
			</div>
			<div>
				<span class="text-xs font-semibold text-emphasis">Resource name</span>
				<TextInput bind:value={wiz.review.resourceName} />
			</div>
		</div>
		<p class="text-2xs text-secondary">
			Saved as a Postgres resource at <span class="font-mono">{resourcePath}</span> — usable in any
			SQL step{wiz.review.folder.startsWith('u/') ? ', by you only' : ''}.
		</p>
	{/if}

	{#if sharesDatabaseWith}
		<Alert type="warning" size="xs" bgClass="border-0" title="">
			<span class="font-semibold">{sharesDatabaseWith.name}</span> already uses this database. Both
			data tables would write to the same schema and share one
			<span class="font-mono">_wm_migrations</span> table, so each would see the other's migrations as
			already applied.
		</Alert>
	{/if}

	{#if wiz.provider === 'supabase'}
		<Alert type="info" size="xs" bgClass="border-0" title="">
			Your Supabase sign-in is not stored. If the database password ever changes, anyone with access
			to the project can sign in and reconnect it. Deleting the data table never deletes the
			Supabase project.
		</Alert>
	{/if}
{/snippet}
