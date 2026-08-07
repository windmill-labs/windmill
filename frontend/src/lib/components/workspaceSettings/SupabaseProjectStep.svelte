<script lang="ts" module>
	/** What the host's primary button should say and do right now. */
	export type SupabaseAction = {
		label: string
		disabled: boolean
		busy?: boolean
		act?: () => void
	}

	export type SupabasePick = { project: SupabaseProject; password: string }
</script>

<script lang="ts">
	import Alert from '../common/alert/Alert.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import SetupChecklist, { type SetupStep } from '../wizards/SetupChecklist.svelte'
	import { Database, Loader2 } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { useSupabaseOauth } from './supabaseOauth.svelte'
	import {
		DEFAULT_SUPABASE_REGION,
		SUPABASE_REGIONS,
		createSupabaseProject,
		generateDbPassword,
		listSupabaseOrgs,
		listSupabaseProjects,
		orgSlug,
		supabaseSetupSteps,
		waitUntilSupabaseHealthy,
		type SupabaseOrg,
		type SupabaseProject
	} from './supabaseProvisioning'

	type Props = {
		/** Set once a project is usable and its password is known. */
		result?: SupabasePick | undefined
		defaultProjectName?: string
		/** Resuming after a popup-blocked redirect. */
		resume?: { region: string; projectName: string } | undefined
		/** Called before the redirect fallback, so the host can park what the user had chosen. */
		onPopupBlocked?: (state: { region: string; projectName: string }) => void
		/** Appended below the provisioning steps, for work the host does after this one. */
		extraSteps?: SetupStep[]
		/** True while the host is busy with `result`, so the action reflects it. */
		hostBusy?: boolean
		/** Overrides the action label once everything here is done. */
		continueLabel?: string
	}

	let {
		result = $bindable(undefined),
		defaultProjectName,
		resume,
		onPopupBlocked,
		extraSteps,
		hostBusy = false,
		continueLabel = 'Continue'
	}: Props = $props()

	const oauth = useSupabaseOauth({
		onPopupBlocked: () => onPopupBlocked?.({ region, projectName })
	})

	let mode: 'create' | 'existing' = $state('create')

	let orgs: SupabaseOrg[] | undefined = $state(undefined)
	let projects: SupabaseProject[] | undefined = $state(undefined)
	let selectedOrg: string | undefined = $state(undefined)
	let region = $state(resume?.region ?? DEFAULT_SUPABASE_REGION)
	let projectName = $state(resume?.projectName ?? defaultProjectName ?? '')
	let selectedProject: SupabaseProject | undefined = $state(undefined)
	let existingPassword = $state('')

	// 0 idle, 1 creating, 2 starting, 3 ready
	let provisioning = $state(0)
	let provisionStatus = $state('')
	/** Set when a project was created but could not be handed over; the password is otherwise lost. */
	let strandedPassword = $state('')

	// Nothing but a spinner until *both* lists are in. Which mode to open on depends on the
	// projects, so clearing this when only the orgs have landed is what makes the toggle flip
	// under the user a moment later.
	let loading = $state(false)

	$effect(() => {
		if (oauth.token && orgs === undefined) load(oauth.token)
	})

	async function load(t: string) {
		loading = true
		try {
			orgs = await listSupabaseOrgs(t)
			if (orgs?.length && !selectedOrg) selectedOrg = orgSlug(orgs[0])
			projects = await listSupabaseProjects(t)
			// Someone who already has a Supabase database almost always means to connect it
			// rather than make a second one. Decided before anything renders, so the toggle
			// never visibly flips under the user; a resumed run was already mid-creation.
			if (!resume && projects?.length) mode = 'existing'
		} catch (err) {
			sendUserToast(String(err), true)
			orgs = orgs ?? []
		} finally {
			loading = false
		}
	}

	/** Supabase statuses are SCREAMING_SNAKE; only surface one that is not the happy path. */
	function projectStatus(p: SupabaseProject): string | undefined {
		if (!p.status || p.status === 'ACTIVE_HEALTHY') return undefined
		return p.status === 'INACTIVE' ? 'paused' : p.status.toLowerCase().replace(/_/g, ' ')
	}

	async function provision() {
		if (!oauth.token || !selectedOrg || !projectName) return
		provisioning = 1
		try {
			const dbPass = generateDbPassword()
			// Surface the password before waiting: Supabase never hands it back, so a failure
			// after this point would leave a project whose password nobody holds.
			strandedPassword = dbPass
			const created = await createSupabaseProject(oauth.token, {
				name: projectName,
				organizationSlug: selectedOrg,
				region,
				dbPass
			})
			provisioning = 2
			const healthy = await waitUntilSupabaseHealthy(
				oauth.token,
				created.id ?? (created as any).ref,
				(st) => (provisionStatus = st ?? '')
			)
			provisioning = 3
			strandedPassword = ''
			result = { project: healthy, password: dbPass }
		} catch (err) {
			provisioning = 0
			sendUserToast(`Could not create the Supabase project: ${err}`, true)
		}
	}

	function useExisting() {
		if (!selectedProject || !existingPassword) return
		result = { project: selectedProject, password: existingPassword }
	}

	// Only the two stages this component drives; whatever the host does with the finished
	// project is appended by the host as its own step.
	let steps = $derived([...supabaseSetupSteps(provisioning).slice(0, 2), ...(extraSteps ?? [])])

	let action = $derived.by((): SupabaseAction => {
		if (!oauth.authed)
			return {
				label: oauth.pending ? 'Continue' : 'Connect to Supabase',
				disabled: false,
				act: () => oauth.connect()
			}
		if (loading) return { label: 'Loading', disabled: true, busy: true }
		if (mode === 'create') {
			if (provisioning === 0)
				return {
					label: 'Create database',
					disabled: !projectName || !selectedOrg,
					act: provision
				}
			if (provisioning < 3) return { label: 'Setting it up', disabled: true, busy: true }
			return { label: continueLabel, disabled: false, busy: hostBusy }
		}
		return {
			label: result ? continueLabel : continueLabel,
			disabled: !selectedProject || !existingPassword,
			busy: hostBusy,
			act: useExisting
		}
	})

	export function getAction(): SupabaseAction {
		return action
	}
	export function isAuthed(): boolean {
		return oauth.authed
	}
</script>

{#if !oauth.authed}
	<Alert type="info" size="xs" bgClass="border-0" title="">
		{#if oauth.pending}
			Sign in and approve Windmill in the Supabase window, then come back here.
		{:else}
			Windmill needs your approval on Supabase to see your databases.
		{/if}
	</Alert>
{:else if loading}
	<div class="flex items-center gap-2 text-xs text-secondary py-2">
		<Loader2 size={16} class="animate-spin" />
		Loading your Supabase projects...
	</div>
{:else}
	<ToggleButtonGroup bind:selected={mode}>
		{#snippet children({ item })}
			<ToggleButton value="existing" label="Use an existing one" {item} small />
			<ToggleButton value="create" label="Create a new project" {item} small />
		{/snippet}
	</ToggleButtonGroup>

	{#if mode === 'create'}
		{#if provisioning === 0}
			<div class="grid grid-cols-2 gap-2">
				<div>
					<span class="text-xs font-semibold text-emphasis">Organization</span>
					<Select
						items={(orgs ?? []).map((o) => ({ label: o.name, value: orgSlug(o) }))}
						bind:value={selectedOrg}
						placeholder={orgs === undefined ? 'Loading...' : 'Select'}
					/>
				</div>
				<div>
					<span class="text-xs font-semibold text-emphasis">Region</span>
					<Select
						items={SUPABASE_REGIONS.map((r) => ({ label: r, value: r }))}
						bind:value={region}
						placeholder="Region"
					/>
				</div>
			</div>
			<div>
				<span class="text-xs font-semibold text-emphasis">Project name</span>
				<TextInput bind:value={projectName} inputProps={{ placeholder: defaultProjectName }} />
			</div>
		{:else}
			<SetupChecklist {steps} />
			{#if provisioning < 3}
				<p class="text-xs text-secondary">
					This usually takes a minute or two. You can leave this open.{provisionStatus
						? ` (${provisionStatus})`
						: ''}
				</p>
			{/if}
		{/if}
	{:else}
		<div class="flex flex-col gap-2 overflow-y-auto flex-1 min-h-24 pr-1">
			{#each projects ?? [] as p (p.id)}
				{@const selected = selectedProject?.id === p.id}
				<button
					class="text-left border rounded-md p-3 flex gap-3 items-start transition-colors {selected
						? 'border-border-selected/50 bg-surface-accent-selected'
						: 'border-border-light hover:bg-surface-hover'}"
					onclick={() => (selectedProject = p)}
				>
					<span class="mt-0.5 shrink-0"><Database size={18} class="text-secondary" /></span>
					<span class="flex flex-col gap-0.5 min-w-0">
						<span class="text-xs font-medium {selected ? 'text-accent' : 'text-emphasis'}"
							>{p.name}</span
						>
						<span class="text-xs text-secondary font-normal">
							{p.region}{projectStatus(p) ? ` · ${projectStatus(p)}` : ''}
						</span>
					</span>
				</button>
			{/each}
		</div>
		{#if selectedProject}
			<div>
				<span class="text-xs font-semibold text-emphasis">Database password</span>
				<TextInput
					bind:value={existingPassword}
					inputProps={{ type: 'password', placeholder: '••••••••' }}
				/>
				<p class="text-2xs text-secondary mt-1">
					Find it in your Supabase project settings, under Database.
				</p>
			</div>
		{/if}
	{/if}

	{#if strandedPassword}
		<Alert type="warning" size="xs" bgClass="border-0" title="Save this password">
			<span class="font-mono select-all">{strandedPassword}</span>
			<br />
			The project exists but Windmill could not finish. Supabase never shows this password again.
		</Alert>
	{/if}
{/if}
