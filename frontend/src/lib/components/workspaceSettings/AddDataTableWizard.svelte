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
	import { base } from '$lib/base'
	import { Database, Check, ArrowRight, Loader2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import Badge from '../common/badge/Badge.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Stepper from '../common/stepper/Stepper.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import ResourcePicker from '../ResourcePicker.svelte'
	import SupabaseIcon from '../icons/SupabaseIcon.svelte'
	import { OauthService, ResourceService, VariableService, WorkspaceService } from '$lib/gen'
	import type { TestDataTableConnectionResponse } from '$lib/gen'
	import { oauthStore, userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import {
		DEFAULT_SUPABASE_REGION,
		SUPABASE_REGIONS,
		createSupabaseProject,
		generateDbPassword,
		listSupabaseOrgs,
		listSupabaseProjects,
		orgSlug,
		supabaseResourceValue,
		waitUntilSupabaseHealthy,
		type SupabaseOrg,
		type SupabaseProject
	} from './supabaseProvisioning'

	type Props = {
		opened: boolean
		existingNames: string[]
		/** Set when Supabase redirected the user back here mid-flow. */
		resume?: WizardResume | undefined
		onDone: () => void
		/** Instance databases are provisioned by a superadmin from the row itself, so that
		 * branch hands back to the existing inline editor rather than duplicating it. */
		onUseInstance: () => void
	}

	let { opened = $bindable(), existingNames, resume, onDone, onUseInstance }: Props = $props()

	type Provider = 'supabase' | 'existing' | 'instance'

	let step = $state(1)
	let provider: Provider | undefined = $state(undefined)
	let supaMode: 'create' | 'existing' = $state('create')
	let supaModeChosen = $state(false)

	let orgs: SupabaseOrg[] | undefined = $state(undefined)
	let selectedOrg: string | undefined = $state(undefined)
	let region: string = $state(DEFAULT_SUPABASE_REGION)
	let projectName = $state('')
	let projects: SupabaseProject[] | undefined = $state(undefined)
	let selectedProject: SupabaseProject | undefined = $state(undefined)
	let existingPassword = $state('')

	let resourcePath: string | undefined = $state(undefined)
	let dataTableName = $state('main')

	let provisioning = $state(0) // 0 idle, 1 created, 2 starting, 3 checking, 4 ready
	let provisionStatus = $state('')
	let checking = $state(false)
	let checkReport: TestDataTableConnectionResponse | undefined = $state(undefined)
	let checkError = $state('')
	let finishing = $state(false)
	// Set only when a Supabase project was created but its credentials could not be saved.
	let strandedPassword = $state('')
	// Which password the saved resource holds, so a retry only re-creates it when it changed.
	let savedPassword = $state('')

	let token = $derived($oauthStore?.access_token)
	let authed = $derived(!!token)

	// Supabase statuses are SCREAMING_SNAKE; only worth showing when it is not the happy path,
	// since a paused project (free tier pauses after a week idle) fails the connection check.
	function projectStatus(p: SupabaseProject): string | undefined {
		if (!p.status || p.status === 'ACTIVE_HEALTHY') return undefined
		return p.status === 'INACTIVE' ? 'paused' : p.status.toLowerCase().replace(/_/g, ' ')
	}

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
			// Names are derived on open, not at construction: the wizard is mounted for the
			// lifetime of the page, so the existing data tables are not known until then.
			dataTableName = resume?.name || defaultTableName()
			projectName = resume?.projectName || defaultProjectName()
		}
		if (resume) {
			provider = 'supabase'
			supaMode = 'create'
			step = 2
			region = resume.region
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

	$effect(() => {
		if (opened && provider === 'supabase' && token && orgs === undefined) {
			loadSupabase(token)
		}
	})

	async function loadSupabase(t: string) {
		try {
			orgs = await listSupabaseOrgs(t)
			if (orgs?.length && !selectedOrg) selectedOrg = orgSlug(orgs[0])
			projects = await listSupabaseProjects(t)
			// Someone who already has a Supabase database almost always means to connect it
			// rather than make a second one. Only pre-empt the choice they have not made yet:
			// a resumed run was already mid-creation, and a manual pick stands.
			if (!resume && !supaModeChosen && projects?.length) supaMode = 'existing'
		} catch (err) {
			sendUserToast(String(err), true)
			orgs = orgs ?? []
		}
	}

	const OAUTH_WINDOW = 'windmill_supabase_oauth'

	let oauthWindow: Window | null = null
	let oauthPending = $state(false)

	/**
	 * A full-page redirect unmounts the wizard, so a user who stops to create a Supabase
	 * account lands on their dashboard with nothing left pointing back here. Driving the flow
	 * from a popup keeps this modal on screen, and keeps the window ours to steer: after they
	 * sign up we send the same popup back through the connect endpoint and consent follows.
	 */
	function startOauth() {
		const url = `${base}/api/oauth/connect/supabase_wizard`
		oauthWindow = window.open(url, OAUTH_WINDOW, 'width=600,height=820')
		if (!oauthWindow) {
			// Popups blocked: fall back to the redirect, parking what the user had chosen.
			parkWizard({ name: dataTableName, region, projectName })
			window.location.href = url
			return
		}
		oauthPending = true
		step = 2
	}

	$effect(() => {
		function onMessage(e: MessageEvent) {
			if (e.origin !== window.location.origin || e.data?.type !== 'supabase_oauth') return
			$oauthStore = e.data.res
			oauthPending = false
			oauthWindow?.close()
		}
		window.addEventListener('message', onMessage)
		return () => window.removeEventListener('message', onMessage)
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

	async function provision() {
		if (!token || !selectedOrg || !projectName) return
		provisioning = 1
		provisionStatus = ''
		checkError = ''
		let createdPassword = ''
		try {
			const dbPass = generateDbPassword()
			// Before the call, not after: a throw here can still leave a project behind, and
			// Supabase will never show its password again.
			createdPassword = dbPass
			const created = await createSupabaseProject(token, {
				name: projectName,
				organizationSlug: selectedOrg,
				region,
				dbPass
			})
			provisioning = 2
			const healthy = await waitUntilSupabaseHealthy(
				token,
				created.id ?? (created as any).ref,
				(s) => (provisionStatus = s ?? '')
			)
			provisioning = 3
			// Save the credentials before checking: Supabase never hands the password back, so
			// losing it here would leave a project nobody can log into.
			resourcePath = await saveSupabaseResource(healthy, dbPass)
			await runCheck()
			provisioning = 4
		} catch (err) {
			checkError = String(err)
			provisioning = 0
			// The project may already exist on Supabase by now, and its password cannot be read
			// back from there. Show it rather than let the database become unusable.
			if (createdPassword) strandedPassword = createdPassword
			// The project may exist now; refresh so it can be picked up from the other tab
			// instead of provisioning a second one.
			listSupabaseProjects(token)
				.then((p) => (projects = p))
				.catch(() => {})
			sendUserToast(String(err), true)
		}
	}

	async function connectExistingSupabase() {
		if (!token || !selectedProject || !existingPassword) return
		checking = true
		checkError = ''
		try {
			// Only create the credentials once: a retry after fixing GRANTs re-checks what is
			// already saved rather than leaving another orphan variable behind each time.
			if (!resourcePath || savedPassword !== existingPassword) {
				resourcePath = await saveSupabaseResource(selectedProject, existingPassword)
				savedPassword = existingPassword
			}
			await runCheck()
			if (canAdvanceFromSetup()) step = 3
		} catch (err) {
			checkError = String(err)
			sendUserToast(String(err), true)
		} finally {
			checking = false
		}
	}

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

	function checkPassed(): boolean {
		return !!checkReport && checkReport.can_create_table && !checkError
	}

	function canAdvanceFromSetup(): boolean {
		if (provider === 'supabase' && supaMode === 'create') return provisioning === 4 && checkPassed()
		return checkPassed()
	}

	async function finish() {
		if (!resourcePath && provider !== 'instance') return
		finishing = true
		try {
			// editDataTableConfig replaces the whole map, so the existing entries have to be
			// read back and merged or they are silently dropped.
			const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
			const datatables: Record<string, any> = { ...(settings.datatable?.datatables ?? {}) }
			datatables[dataTableName] = {
				database: { resource_type: 'postgresql', resource_path: resourcePath }
			}
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
		supaMode = 'create'
		supaModeChosen = false
		orgs = undefined
		projects = undefined
		selectedOrg = undefined
		selectedProject = undefined
		existingPassword = ''
		resourcePath = undefined
		provisioning = 0
		provisionStatus = ''
		checkReport = undefined
		checkError = ''
		strandedPassword = ''
		savedPassword = ''
		projectName = ''
		dataTableName = defaultTableName()
	}

	// The single primary action. Its label says what it is about to do, and doing it is what
	// moves the wizard on -- there is no separate "run the check" or "create" button.
	let primary = $derived.by(() => {
		if (step === 1) {
			// Signing in is the whole of the Supabase setup step, so go straight there rather
			// than spending a screen telling the user what the button is about to do.
			if (provider === 'supabase' && !authed)
				return { label: 'Connect to Supabase', disabled: false, act: startOauth }
			return {
				label: 'Continue',
				disabled: !provider,
				act: () => {
					if (provider === 'instance') {
						opened = false
						reset()
						onUseInstance()
					} else {
						step = 2
					}
				}
			}
		}
		if (step === 2) {
			if (provider === 'supabase') {
				// Reached while the popup is still open, or if the redirect came back without a
				// token. Either way the action is the same: send the popup through consent again,
				// which is immediate once the user has an account and is signed in.
				if (!authed)
					return {
						label: oauthPending ? 'Continue' : 'Connect to Supabase',
						disabled: false,
						act: startOauth
					}
				if (supaMode === 'create') {
					if (provisioning === 0)
						return {
							label: 'Create database',
							disabled: !projectName || !selectedOrg,
							act: provision
						}
					if (provisioning < 4) return { label: 'Setting it up', disabled: true, busy: true }
					if (!checkPassed()) return { label: 'Try again', disabled: false, act: runCheck }
					return { label: 'Continue', disabled: false, act: () => (step = 3) }
				}
				if (checking) return { label: 'Checking', disabled: true, busy: true }
				return {
					label: checkReport && !checkPassed() ? 'Try again' : 'Continue',
					disabled: !selectedProject || !existingPassword,
					act: connectExistingSupabase
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
								'Sign in and Windmill sets up a database for you.',
								'free'
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
					{#if provider === 'supabase' && !authed}
						<Alert type="info" size="xs" bgClass="border-0" title="">
							{#if oauthPending}
								Sign in and approve Windmill in the Supabase window, then come back here.
							{:else}
								Windmill needs your approval on Supabase to see your databases.
							{/if}
						</Alert>
					{:else if provider === 'supabase'}
						<ToggleButtonGroup
							bind:selected={
								() => supaMode,
								(v) => {
									supaMode = v
									supaModeChosen = true
								}
							}
						>
							{#snippet children({ item })}
								<ToggleButton value="existing" label="Use an existing one" {item} small />
								<ToggleButton value="create" label="Create a new project" {item} small />
							{/snippet}
						</ToggleButtonGroup>
						{#if supaMode === 'create'}
							{#if provisioning === 0}
								<div class="grid grid-cols-2 gap-2">
									<div>
										<span class="text-xs font-semibold text-primary">Organization</span>
										<Select
											items={(orgs ?? []).map((o) => ({ label: o.name, value: orgSlug(o) }))}
											bind:value={selectedOrg}
											placeholder={orgs === undefined ? 'Loading...' : 'Select'}
										/>
									</div>
									<div>
										<span class="text-xs font-semibold text-primary">Region</span>
										<Select
											items={SUPABASE_REGIONS.map((r) => ({ label: r, value: r }))}
											bind:value={region}
											placeholder="Region"
										/>
									</div>
								</div>
								<div>
									<span class="text-xs font-semibold text-primary">Project name</span>
									<TextInput
										bind:value={projectName}
										inputProps={{ placeholder: defaultProjectName() }}
									/>
									<p class="text-2xs text-secondary mt-1">
										Named after your workspace. Change it if you like.
									</p>
								</div>
								<p class="text-2xs text-secondary">
									Free on Supabase. You can upgrade later without changing anything here.
								</p>
							{:else}
								<div class="flex flex-col gap-1.5">
									{@render progress(provisioning >= 2, provisioning === 1, 'Created on Supabase')}
									{@render progress(provisioning >= 3, provisioning === 2, 'Starting it up')}
									{@render progress(
										provisioning >= 4,
										provisioning === 3,
										'Checking Windmill can store data'
									)}
								</div>
								{#if provisioning < 4}
									<p class="text-xs text-secondary">
										This usually takes a minute or two. You can leave this open.{provisionStatus
											? ` (${provisionStatus})`
											: ''}
									</p>
								{:else if checkPassed()}
									<Alert
										type="success"
										size="xs"
										bgClass="border-0"
										title="{projectName} is ready"
									/>
								{/if}
							{/if}
						{:else}
							<div class="flex flex-col gap-2">
								{#each projects ?? [] as p}
									<button
										class="text-left border rounded-md p-2 transition-colors {selectedProject?.id ===
										p.id
											? 'border-border-selected/50 bg-surface-accent-selected'
											: 'border-border-light hover:bg-surface-hover'}"
										onclick={() => (selectedProject = p)}
									>
										<span
											class="text-xs font-medium {selectedProject?.id === p.id
												? 'text-accent'
												: 'text-emphasis'}">{p.name}</span
										>
										<span class="block text-xs text-secondary font-normal">
											{p.region}{#if projectStatus(p)}&nbsp;&middot; {projectStatus(p)}{/if}
										</span>
									</button>
								{/each}
								{#if selectedProject}
									<div>
										<span class="text-xs font-semibold text-primary"
											>Database password for {selectedProject.name}</span
										>
										<TextInput
											bind:value={existingPassword}
											inputProps={{ type: 'password', placeholder: '••••••••' }}
										/>
										<p class="text-2xs text-secondary mt-1">
											Find it in your Supabase project settings, under Database.
										</p>
									</div>
								{/if}
							</div>
						{/if}
					{:else}
						<div>
							<span class="text-xs font-semibold text-primary">Database</span>
							<ResourcePicker bind:value={resourcePath} resourceType="postgresql" />
							<p class="text-2xs text-secondary mt-1">
								Pick one, or add a new one with its connection string.
							</p>
						</div>
					{/if}

					{@render checkResult()}
				{:else}
					<div>
						<span class="text-xs font-semibold text-primary">Name this data table</span>
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

			<div class="flex justify-between items-center pt-3 gap-2">
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
		</div>
	</div>
</Modal2>

{#snippet providerCard(
	key: Provider,
	icon: Snippet,
	title: string,
	subtitle: string,
	badge?: string
)}
	{@const selected = provider === key}
	<button
		class="text-left border rounded-md p-3 flex gap-3 items-start transition-colors {selected
			? 'border-border-selected/50 bg-surface-accent-selected'
			: 'border-border-light hover:bg-surface-hover'}"
		onclick={() => (provider = key)}
	>
		<span class="mt-0.5 shrink-0">{@render icon()}</span>
		<span class="flex flex-col gap-0.5 min-w-0">
			<span class="flex items-center gap-1.5">
				<span class="text-xs font-medium {selected ? 'text-accent' : 'text-emphasis'}">{title}</span
				>
				{#if badge}
					<Badge color="green" small>{badge}</Badge>
				{/if}
			</span>
			<span class="text-xs text-secondary font-normal">{subtitle}</span>
		</span>
	</button>
{/snippet}

{#snippet progress(done: boolean, running: boolean, label: string)}
	<div class="flex items-center gap-2 text-xs bg-surface-secondary rounded-md px-2 py-1.5">
		{#if done}
			<Check size={14} class="text-green-500" />
		{:else if running}
			<Loader2 size={14} class="animate-spin text-blue-500" />
		{:else}
			<span class="w-3.5 h-3.5 rounded-full border border-gray-300"></span>
		{/if}
		<span>{label}</span>
	</div>
{/snippet}

{#snippet checkResult()}
	{#if strandedPassword}
		<Alert type="error" size="xs" bgClass="border-0" title="Save this password">
			<div class="flex flex-col gap-2">
				<div>
					The project was created on Supabase but Windmill could not store its credentials. Supabase
					cannot show this password again - copy it now, or reset it from the project's database
					settings.
				</div>
				<pre class="whitespace-pre-wrap select-all text-2xs">{strandedPassword}</pre>
			</div>
		</Alert>
	{/if}
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
