<script lang="ts">
	import { parkWizard, type WizardResume } from './wizardParking'
	import type { Snippet } from 'svelte'
	import { Database, ArrowRight, Plus, ChevronRight } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import Modal2 from '../common/modal/Modal2.svelte'
	import Stepper from '../common/stepper/Stepper.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import InputError from '../InputError.svelte'
	import Password from '../Password.svelte'
	import Select from '../select/Select.svelte'
	import Toggle from '../Toggle.svelte'
	import Path from '../Path.svelte'
	import Label from '../Label.svelte'
	import SupabaseIcon from '../icons/SupabaseIcon.svelte'
	import {
		FolderService,
		OauthService,
		ResourceService,
		VariableService,
		WorkspaceService
	} from '$lib/gen'
	import type { ListCustomInstanceDbsResponse } from '$lib/gen'
	import { resource, type ResourceReturn } from 'runed'
	import type { ConfirmationModalHandle } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import SetupChecklist, { type SetupStep } from '../wizards/SetupChecklist.svelte'
	import SupabaseProjectStep from './SupabaseProjectStep.svelte'
	import DataTableConnectionReport from './DataTableConnectionReport.svelte'
	import { useSupabaseOauth } from './supabaseOauth.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import {
		composePostgresConnectionString,
		parsePostgresConnectionString
	} from '$lib/utils/postgresConnectionString'
	import {
		clearProbe,
		datatableNameError,
		hasAdvanced,
		instanceDbNameError,
		intentComplete,
		newResourceParts,
		newWizardState,
		supabaseSummary,
		planSteps,
		probeValue,
		resourcePathOf,
		runSetup,
		type Provider,
		type RunResult,
		type WizardState
	} from './addDataTableModel'
	import {
		resolveSupabaseConnection,
		supabaseResourceValue,
		DEFAULT_SUPABASE_REGION
	} from './supabaseProvisioning'

	type Props = {
		opened: boolean
		existingNames: string[]
		/** Every data table already configured, so the review step can warn about sharing one. */
		existingDataTables: { name: string; resourcePath: string | undefined }[]
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
	let preventClose = $state(false)
	/** A dismissal dialog is up, so a second one must not stack on it. */
	let dismissing = false

	/** Set once Finish has been pressed; the modal shows the checklist from then on. */
	let run: { steps: SetupStep[]; running: boolean; result?: Awaited<ReturnType<typeof runSetup>> } =
		$state({ steps: [], running: false })
	/** Why the session pooler could not be used, once something has tried to read it. */
	let poolerUnavailable: string | undefined = $state(undefined)
	let initialResourcePath = $state('')
	let resourcePathError = $state('')
	/**
	 * The run writes a resource *and* a secret variable at this one path, and both writes
	 * upsert so a retry can repair its own half-finished attempt. `Path` only knows about the
	 * resource namespace, so without this a variable already sitting there is overwritten
	 * with no warning.
	 */
	let variablePathError = $state('')
	let variableCheck: ReturnType<typeof setTimeout> | undefined = undefined
	$effect(() => {
		const path = resourcePath
		if (wiz.step !== 3 || !mintsResource || !path) {
			variablePathError = ''
			return
		}
		// A path this wizard already wrote is not somebody else's to protect -- the same
		// exemption the hard check in `finish()` makes, or a retry would refuse its own secret
		// and leave Finish permanently disabled.
		if (path === claimedPath) {
			variablePathError = ''
			return
		}
		clearTimeout(variableCheck)
		variableCheck = setTimeout(async () => {
			const taken = await VariableService.existsVariable({ workspace: $workspaceStore!, path })
			// Two checks can be in flight at once and resolve out of order. Answering for a path
			// that is no longer the one on screen is not merely stale: a `false` for an older
			// path would clear the error guarding the path actually about to be written.
			if (path !== resourcePath) return
			variablePathError = taken ? 'a variable already exists at this path' : ''
		}, 500)
		return () => clearTimeout(variableCheck)
	})
	/** Furthest step reached, so going back to check something does not cost the progress. */
	let maxStep = $state(1)

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

	let nameError = $derived(datatableNameError(wiz.review.name, existingNames))
	$effect(() => {
		if (wiz.review.name.trim() !== claimedName) nameConflict = ''
	})
	// Every database on the instance, not just the data table ones: the name has to be free
	// in PostgreSQL, and a collision with a database created for something else still fails.
	let instanceNameError = $derived(
		wiz.provider === 'instance' && wiz.instance.mode === 'create'
			? instanceDbNameError(wiz.instance.dbName ?? '', Object.keys(customInstanceDbs.current ?? {}))
			: undefined
	)
	let connectionStringError = $derived(
		wiz.own.connectionString && !parsePostgresConnectionString(wiz.own.connectionString)
			? 'That is not a Postgres connection string.'
			: undefined
	)
	let resourcePath = $derived(resourcePathOf(wiz))

	/**
	 * Another data table on the same database. They would share one `_wm_migrations` table,
	 * so each would see the other's migrations as already applied.
	 */
	let sharesDatabaseWith = $derived(
		wiz.provider === 'resource' && !wiz.own.creating && wiz.own.resourcePath
			? existingDataTables.find((d) => d.resourcePath === wiz.own.resourcePath)
			: undefined
	)

	// Undefined, not [], until the workspace has actually been asked: an empty list is the
	// answer "this workspace has none", and the default selection below acts on it.
	const pgResources = resource(
		() => (opened && wiz.provider === 'resource' ? ($workspaceStore ?? '') : ''),
		async (workspace) =>
			workspace
				? await ResourceService.listResource({ workspace, resourceType: 'postgresql' })
				: undefined
	)

	// The step opens on a choice rather than on nothing: the first resource when the workspace
	// has any, the creation form when it has none. Only ever fills an empty selection, so it
	// cannot overwrite what the user picked. Waits for the fetch to settle -- deciding off the
	// previous answer would read a workspace with resources as one without.
	$effect(() => {
		const resources = pgResources.current
		if (wiz.provider !== 'resource' || pgResources.loading || !resources) return
		if (wiz.own.creating || wiz.own.resourcePath) return
		if (resources.length) wiz.own.resourcePath = resources[0].path
		else wiz.own.creating = true
	})

	/** Exclusive: one list, and a card is either the selection or it is not. */
	function selectResource(path: string) {
		if (wiz.own.resourcePath !== path || wiz.own.creating) invalidate()
		wiz.own.resourcePath = path
		wiz.own.creating = false
	}

	function selectNewResource() {
		if (!wiz.own.creating) invalidate()
		wiz.own.creating = true
		wiz.own.resourcePath = undefined
	}

	/**
	 * The two notations describe one connection, so switching carries the values across rather
	 * than starting the other one blank -- the round trip is what shows they are the same
	 * object. Unparseable text is left alone to be corrected.
	 */
	function useFields() {
		const parts = parsePostgresConnectionString(wiz.own.connectionString)
		// The string is authoritative for every field it can express, so a password dropped from
		// it must clear the one held here rather than linger. `sslmode` is the exception:
		// composing omits `prefer`, so an absent one means "unchanged", not "cleared".
		if (parts) wiz.own.fields = { ...parts, sslmode: parts.sslmode ?? wiz.own.fields.sslmode }
		wiz.own.form = 'fields'
	}

	function useConnectionString() {
		const parts = newResourceParts(wiz)
		if (parts) wiz.own.connectionString = composePostgresConnectionString(parts)
		wiz.own.form = 'string'
	}

	const SSL_MODES = ['disable', 'allow', 'prefer', 'require', 'verify-ca', 'verify-full']

	// `accept_invalid_certs` is a tri-state, and its unset state is not a default anyone would
	// guess -- name all three rather than offer a checkbox that cannot express one of them.
	const CERT_VERIFICATION = [
		{ value: 'default', label: 'Verify only when a root certificate is set' },
		{ value: 'enforce', label: 'Always verify' },
		{ value: 'accept', label: 'Accept any certificate' }
	]

	let advancedOpen = $state(false)
	let certVerification = $derived(
		wiz.own.advanced.accept_invalid_certs === undefined
			? 'default'
			: wiz.own.advanced.accept_invalid_certs
				? 'accept'
				: 'enforce'
	)

	function setField<K extends keyof typeof wiz.own.fields>(
		key: K,
		value: (typeof wiz.own.fields)[K]
	) {
		wiz.own.fields = { ...wiz.own.fields, [key]: value }
		invalidate()
	}

	function setAdvanced<K extends keyof typeof wiz.own.advanced>(
		key: K,
		value: (typeof wiz.own.advanced)[K]
	) {
		wiz.own.advanced = { ...wiz.own.advanced, [key]: value }
		invalidate()
	}

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
			.then((f) => {
				folders = f.filter((x) => !['app_groups', 'app_custom', 'app_themes'].includes(x))
				// The list lands after `reset()` has already seeded the folder, so the first open
				// would otherwise always fall back to the personal space. Only re-seed what the
				// user has not reached yet: the review step is where the folder becomes theirs.
				if (wiz.step < 3) wiz.review.folder = defaultFolder()
			})
			.catch(() => {})
	})

	const SUPABASE_SIGNUP_URL = 'https://supabase.com/dashboard/sign-up'

	const supaOauth = useSupabaseOauth({
		// Safe to navigate this tab away: what the wizard had collected is parked first, and the
		// settings page picks it back up when Supabase sends the user home.
		redirectIfBlocked: true,
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
		maxStep = 1
		// What the last run claimed belongs to the data table it created; a fresh one has to
		// earn the name and the path again, or it would write over its predecessor's secret.
		claimedName = undefined
		claimedPath = undefined
		nameConflict = ''
		variablePathError = ''
		poolerUnavailable = undefined
		if (resume) {
			wiz.provider = 'supabase'
			enterStep(2)
		}
	}

	function selectProvider(key: Provider) {
		if (key === wiz.provider) return
		wiz.provider = key
		invalidate()
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

	/** Moving forward through the primary action, which is what validated the step. */
	function enterStep(step: 1 | 2 | 3) {
		wiz.step = step
		if (step > maxStep) maxStep = step
	}

	/**
	 * The stepper reaches any step already passed, in either direction -- going back to check
	 * something should not cost the progress. It cannot reach past `maxStep`: the primary
	 * action is what validates a step, and on step 2 what probes the connection.
	 */
	function goToStep(index: number) {
		const target = index + 1
		if (run.steps.length || target > maxStep || target === wiz.step) return
		wiz.step = target as 1 | 2 | 3
	}

	/** Intent changed, so the review built from it is stale and stops being reachable. */
	function invalidate() {
		// Also retires any check still in flight, so its answer cannot land on the edited value.
		probeToken++
		clearProbe(wiz)
		// Read off one attempt against one project; the review step would otherwise warn about
		// a limitation that no longer applies while claiming session pooling right above it.
		poolerUnavailable = undefined
		if (maxStep > wiz.step) maxStep = wiz.step
	}

	function enterReview() {
		if (!wiz.review.resourceName) wiz.review.resourceName = suggestedResourceName()
		// Path seeds itself from initialPath, so it has to be the suggestion as it stood when
		// the step opened, not a live view of it -- a moving initialPath fights the typing.
		initialResourcePath = resourcePathOf(wiz)
		enterStep(3)
	}

	/**
	 * Proves the typed connection works before the wizard writes anything. The Supabase branch
	 * has to read the pooler first: which host a project answers on is assigned by Supabase,
	 * so the value under test has to be the value that will be saved.
	 */
	// The fields stay editable while a check is out, and editing clears the verdict. Without a
	// token the older answer lands afterwards and marks the edited connection as tested, so
	// Continue unlocks for something nobody proved.
	let probeToken = 0
	async function probe() {
		const token = ++probeToken
		const settle = (probe: WizardState['probe']) => {
			if (token === probeToken) wiz.probe = probe
		}
		wiz.probe = { checking: true, report: undefined, error: undefined }
		try {
			let value = probeValue(wiz)
			if (!value && wiz.provider === 'supabase' && wiz.supabase.project) {
				const connection = await resolveSupabaseConnection(
					supaOauth.token!,
					wiz.supabase.project,
					wiz.supabase.connectionMode
				)
				// Retired while the pooler lookup was out: the user has changed something since,
				// and writing the resolved mode back would silently undo their choice.
				if (token !== probeToken) return
				wiz.supabase.connectionMode = connection.mode
				poolerUnavailable = connection.unavailable
				value = {
					...supabaseResourceValue(wiz.supabase.project, '', connection),
					password: wiz.supabase.password
				}
			}
			if (!value && wiz.provider === 'resource' && wiz.own.resourcePath) {
				const report = await WorkspaceService.testDataTableResourceConnection({
					workspace: $workspaceStore!,
					resourcePath: wiz.own.resourcePath
				})
				settle({ checking: false, report, error: undefined })
				return
			}
			if (!value) {
				settle({ checking: false, report: undefined, error: undefined })
				return
			}
			const report = await WorkspaceService.testDataTableConnectionValue({
				workspace: $workspaceStore!,
				requestBody: value
			})
			settle({ checking: false, report, error: undefined })
		} catch (err: any) {
			settle({
				checking: false,
				report: undefined,
				error: err?.body ?? err?.message ?? String(err)
			})
		}
	}

	/** True when step 2 has nothing to prove before Finish, so Continue is the only action. */
	let probeable = $derived(
		wiz.provider === 'resource' || (wiz.provider === 'supabase' && wiz.supabase.mode === 'existing')
	)
	let probePassed = $derived(!!wiz.probe.report?.can_create_table && !wiz.probe.error)

	/**
	 * The name this run has already claimed. `writeRow` merges into whatever the server holds
	 * under the name, so a name that is free in the table on screen but taken on the server --
	 * an unsaved rename here, or another admin since the page loaded -- would repoint someone
	 * else's data table at this database. Checked against the server before the run starts, then
	 * remembered only if the run got as far as writing the row, so Try again can overwrite what
	 * it wrote itself.
	 */
	let claimedName = $state<string | undefined>(undefined)
	/** The secret path a previous attempt wrote, which this one is allowed to write over. */
	let claimedPath = $state<string | undefined>(undefined)
	let nameConflict = $state('')

	/**
	 * A refused pre-flight means nothing ran, so the checklist from a previous attempt has to
	 * give way -- it is the only thing rendered while it exists, and the refusal is shown on
	 * the review step.
	 */
	function backToReview() {
		run = { steps: [], running: false }
		enterStep(3)
	}

	/**
	 * Latched before the pre-flight's first await. `run.running` -- what otherwise disables the
	 * button -- is only set a round trip later, and on the Supabase create branch two clicks in
	 * that window bill two projects and leave the first one's password overwritten, which
	 * Supabase can never give back.
	 */
	let submitting = $state(false)

	async function finish() {
		if (submitting) return
		submitting = true
		// Held from here, not from where the run starts: the pre-flight awaits twice first, and
		// a dismissal landing in that window would otherwise let the modal close under a setup
		// that is about to create things.
		preventClose = true
		try {
			await runFinish()
		} finally {
			submitting = false
			preventClose = false
		}
	}

	async function runFinish() {
		const name = wiz.review.name.trim()
		try {
			if (claimedName !== name) {
				const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
				if (settings.datatable?.datatables?.[name]) {
					nameConflict = `A data table called ${name} already exists in this workspace.`
					backToReview()
					return
				}
				nameConflict = ''
			}
			// The debounced path check is advisory -- it can still be in flight when Finish is
			// pressed -- and the write that follows replaces a secret in place. Ask once more
			// here, where refusing costs nothing. Skipped for a path this wizard already wrote:
			// Try again has to be able to repair its own half-finished attempt.
			if (
				mintsResource &&
				claimedPath !== resourcePath &&
				(await VariableService.existsVariable({ workspace: $workspaceStore!, path: resourcePath }))
			) {
				variablePathError = 'a variable already exists at this path'
				backToReview()
				return
			}
		} catch (err: any) {
			// Everything inside the run reports through the checklist; this runs before there is
			// one, so it has to speak for itself rather than fail silently.
			nameConflict = `Could not check the name: ${err?.body ?? err?.message ?? String(err)}`
			backToReview()
			return
		}
		run = { steps: planSteps(wiz), running: true }
		let result: RunResult | undefined = undefined
		try {
			result = await runSetup(wiz, {
				workspace: $workspaceStore!,
				supabaseToken: supaOauth.token,
				onInstanceDbsChanged: async () => {
					await customInstanceDbs.refetch()
				},
				onProgress: (steps) => (run.steps = steps),
				onPoolerUnavailable: (reason) => (poolerUnavailable = reason)
			})
		} finally {
			// `runSetup` catches per step, but anything escaping it would otherwise leave the
			// button spinning with a page reload the only way out.
			// Kept, not replaced: what an earlier attempt wrote is still out there, so a later
			// one failing sooner must not hand its own row or secret back to the collision
			// checks. Claimed only once the row exists.
			if (result?.rowWritten) claimedName = name
			claimedPath = result?.mintedPath ?? claimedPath
			run = {
				...run,
				running: false,
				result: result ?? { ok: false, error: 'The setup stopped unexpectedly.' }
			}
			onDone()
		}
	}

	/**
	 * Whether closing would throw away work. Nothing is written before Finish, so the loss is
	 * only what was typed -- but that includes a pasted database password and a project about
	 * to be created, and a backdrop click is easy to do by accident. A run cannot be closed
	 * at all while it is going, and once it has a result there is nothing left to lose.
	 */
	function hasUnfinishedIntent(): boolean {
		return wiz.provider !== undefined && !run.running && !run.result
	}

	/** Backdrop, Escape and the close button all arrive here. */
	async function requestClose() {
		// `dismissing` is its own flag rather than a second use of `preventClose`: a run started
		// while the dialog was up would have its hold cleared here, and the modal would become
		// closable in the middle of creating things.
		if (preventClose || dismissing) return
		if (!hasUnfinishedIntent()) {
			close()
			return
		}
		dismissing = true
		const confirmed = await confirmationModal.ask({
			title: 'Leave without adding a data table?',
			children: 'Nothing has been created yet, and what you have filled in here will be lost.',
			confirmationText: 'Discard'
		})
		dismissing = false
		// Re-read rather than trust the entry check: a run can start while the dialog is up, and
		// answering Discard would otherwise tear the modal down in the middle of it.
		if (confirmed && !preventClose) close()
	}

	function close() {
		opened = false
	}

	// The single primary action. Its label says what it is about to do, and doing it is what
	// moves the wizard on.
	let primary = $derived.by(() => {
		if (submitting && !run.running)
			return { label: 'Setting things up', disabled: true, busy: true }
		if (run.steps.length) {
			if (run.running) return { label: 'Setting things up', disabled: true, busy: true }
			if (run.result?.ok) return { label: 'Done', disabled: false, act: close }
			// A run that died because the Supabase token expired would retry into the same 401
			// forever; authorizing again is the only thing that can move it on.
			if (wiz.provider === 'supabase' && !supaOauth.authed)
				return {
					label: 'Connect to Supabase',
					disabled: false,
					busy: supaOauth.pending,
					act: () => supaOauth.connect()
				}
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
						enterStep(2)
					}
				}
			return {
				label: 'Continue',
				disabled: !wiz.provider,
				act: () => enterStep(2)
			}
		}
		if (wiz.step === 2) {
			// Reachable by closing the Supabase window without consenting -- which this flow
			// invites, since creating an account happens over there. Without a way back in, the
			// step offers only a Test connection that can never enable.
			if (wiz.provider === 'supabase' && !supaOauth.authed)
				return {
					label: 'Connect to Supabase',
					disabled: false,
					busy: supaOauth.pending,
					act: () => supaOauth.connect()
				}
			if (wiz.probe.checking) return { label: 'Checking', disabled: true, busy: true }
			if (probeable && !probePassed)
				return {
					label: wiz.probe.error || wiz.probe.report ? 'Try again' : 'Test connection',
					disabled: !intentComplete(wiz),
					act: probe
				}
			return {
				label: 'Continue',
				disabled: !intentComplete(wiz) || !!instanceNameError,
				act: enterReview
			}
		}
		return {
			label:
				wiz.provider === 'supabase' && wiz.supabase.mode === 'create'
					? 'Create project and data table'
					: 'Create data table',
			disabled:
				// Guards the way back as well as the way forward: the stepper can return to step 2,
				// and not every control there invalidates the review it just made stale.
				!intentComplete(wiz) ||
				!wiz.review.name.trim() ||
				!!nameError ||
				!wiz.review.resourceName.trim() ||
				!!resourcePathError ||
				!!variablePathError ||
				!!instanceNameError,
			act: finish
		}
	})

	/** The review step only mints a resource when the wizard is the one creating it. */
	let mintsResource = $derived(
		wiz.provider === 'supabase' || (wiz.provider === 'resource' && wiz.own.creating)
	)
</script>

<Modal2
	bind:isOpen={
		() => opened,
		(v) => {
			if (!v) requestClose()
			else opened = v
		}
	}
	target="#content"
	title="Add a data table"
	contentClasses="flex flex-col"
	fixedWidth="md"
	fixedHeight="lg"
>
	<div class="flex h-full flex-col gap-4">
		<Stepper
			tabs={STEPS}
			selectedIndex={wiz.step - 1}
			maxReachedIndex={run.steps.length ? -1 : maxStep - 1}
			small
			on:click={(e) => goToStep(e.detail.index)}
		/>

		<div class="flex-1 flex flex-col min-h-0">
			<div class="flex-1 overflow-y-auto flex flex-col gap-3">
				{#if run.steps.length}
					<SetupChecklist steps={run.steps} />
					{#if run.running}
						<p class="text-xs text-secondary">
							Setting up. This can take a few minutes &mdash; leave this open until it finishes.
						</p>
					{/if}
					{#if run.result}
						{@render poolerWarning()}
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
								onIntentChange={() => invalidate()}
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
				error={!!instanceNameError}
				inputProps={{ placeholder: defaultInstanceDbName() }}
			/>
			<InputError error={instanceNameError} />
			{#if !instanceNameError}
				<p class="text-2xs text-secondary mt-1">
					Created in the Windmill PostgreSQL instance when you finish. Windmill manages its
					credentials.
				</p>
			{/if}
		</div>
	{/if}
{/snippet}

{#snippet ownStep()}
	{@const resources = pgResources.loading ? undefined : pgResources.current}
	{#if resources === undefined}
		<p class="text-xs text-secondary">Loading resources...</p>
	{:else}
		{#if resources.length}
			<span class="text-xs font-semibold text-emphasis">Postgres resources in this workspace</span>
		{:else}
			<p class="text-xs text-secondary">A resource is a saved connection your scripts can use.</p>
		{/if}
		<div class="flex flex-col gap-2 overflow-y-auto flex-1 min-h-24 pr-1">
			{#each resources as r (r.path)}
				{@const selected = !wiz.own.creating && wiz.own.resourcePath === r.path}
				<button
					class="shrink-0 text-left border rounded-md p-3 flex gap-3 items-start transition-colors {selected
						? 'border-border-selected/50 bg-surface-accent-selected'
						: 'border-border-light hover:bg-surface-hover'}"
					onclick={() => selectResource(r.path)}
				>
					<span class="mt-0.5 shrink-0"><Database size={18} class="text-secondary" /></span>
					<span class="flex flex-col gap-0.5 min-w-0">
						<span
							class="text-xs font-medium font-mono truncate {selected
								? 'text-accent'
								: 'text-emphasis'}">{r.path}</span
						>
						{#if r.description}
							<span class="text-xs text-secondary font-normal truncate">{r.description}</span>
						{/if}
					</span>
				</button>
			{/each}
			<!-- shrink-0 or the flex column squeezes the cards to fit instead of letting the list
			scroll, and the selected one loses its expanded form to the clip. -->
			<div
				class="shrink-0 border rounded-md overflow-hidden transition-colors {wiz.own.creating
					? 'border-border-selected/50 bg-surface-accent-selected'
					: 'border-border-light'}"
			>
				<button
					class="w-full text-left p-3 flex gap-3 items-start {wiz.own.creating
						? ''
						: 'hover:bg-surface-hover'}"
					onclick={selectNewResource}
				>
					<span class="mt-0.5 shrink-0"><Plus size={18} class="text-secondary" /></span>
					<span class="flex flex-col gap-0.5 min-w-0">
						<span class="text-xs font-medium {wiz.own.creating ? 'text-accent' : 'text-emphasis'}"
							>New resource</span
						>
						<span class="text-xs text-secondary font-normal"
							>Windmill saves it as a Postgres resource</span
						>
					</span>
				</button>
				{#if wiz.own.creating}
					<div class="px-3 pb-3">{@render newResourceForm()}</div>
				{/if}
			</div>
		</div>
	{/if}
{/snippet}

{#snippet newResourceForm()}
	<div class="flex flex-col gap-2">
		<div class="flex items-baseline justify-between gap-2">
			<span class="text-xs font-semibold text-emphasis">
				{wiz.own.form === 'string' ? 'Connection string' : 'Connection'}
			</span>
			<Button
				variant="subtle"
				size="xs2"
				onclick={() => (wiz.own.form === 'string' ? useFields() : useConnectionString())}
			>
				{wiz.own.form === 'string'
					? 'Enter fields individually'
					: 'Paste a connection string instead'}
			</Button>
		</div>
		{#if wiz.own.form === 'string'}
			<TextInput
				bind:value={
					() => wiz.own.connectionString,
					(v) => {
						wiz.own.connectionString = v
						invalidate()
					}
				}
				error={!!connectionStringError}
				inputProps={{ placeholder: 'postgres://user:password@host:5432/database' }}
			/>
			<InputError error={connectionStringError} />
		{:else}
			<div class="grid grid-cols-[2fr_1fr] gap-2">
				<div>
					<span class="text-2xs text-secondary">Host</span>
					<TextInput
						bind:value={() => wiz.own.fields.host, (v) => setField('host', v)}
						inputProps={{ placeholder: 'db.example.com' }}
					/>
				</div>
				<div>
					<span class="text-2xs text-secondary">Port</span>
					<TextInput
						bind:value={
							() => wiz.own.fields.port ?? '',
							(v) => setField('port', v === '' ? undefined : Number(v))
						}
						inputProps={{ placeholder: '5432', type: 'number' }}
					/>
				</div>
				<div>
					<span class="text-2xs text-secondary">Database</span>
					<TextInput
						bind:value={() => wiz.own.fields.dbname ?? '', (v) => setField('dbname', v)}
						inputProps={{ placeholder: 'postgres' }}
					/>
				</div>
				<div>
					<span class="text-2xs text-secondary">SSL mode</span>
					<Select
						items={SSL_MODES.map((value) => ({ value }))}
						bind:value={() => wiz.own.fields.sslmode ?? '', (v) => setField('sslmode', v)}
						clearable={false}
					/>
				</div>
				<div>
					<span class="text-2xs text-secondary">User</span>
					<TextInput
						bind:value={() => wiz.own.fields.user, (v) => setField('user', v)}
						inputProps={{ placeholder: 'postgres' }}
					/>
				</div>
				<div>
					<span class="text-2xs text-secondary">Password</span>
					<Password
						bind:password={
							() => wiz.own.fields.password ?? '', (v) => setField('password', v ?? '')
						}
						placeholder="••••••••"
					/>
				</div>
			</div>
			{@render advancedGroup()}
		{/if}
		{#if wiz.own.form === 'string' && !connectionStringError && hasAdvanced(wiz.own.advanced)}
			<p class="text-2xs text-secondary">
				Certificate and IAM settings from the fields view still apply &mdash; a connection string
				cannot show them.
			</p>
		{/if}
	</div>
{/snippet}

{#snippet advancedGroup()}
	<div class="border-t border-border-light pt-2">
		<Button
			variant="subtle"
			size="xs2"
			startIcon={{
				icon: ChevronRight,
				classes: `transition-transform ${advancedOpen ? 'rotate-90' : ''}`
			}}
			onclick={() => (advancedOpen = !advancedOpen)}
		>
			Advanced
		</Button>
		{#if advancedOpen}
			<div class="flex flex-col gap-2 mt-2">
				<div>
					<span class="text-2xs text-secondary">Root certificate</span>
					<textarea
						value={wiz.own.advanced.root_certificate_pem}
						oninput={(e) => setAdvanced('root_certificate_pem', e.currentTarget.value)}
						placeholder="-----BEGIN CERTIFICATE-----"
						class="w-full min-h-16 p-2 border border-border-light rounded-md bg-surface text-primary font-mono text-2xs resize-y"
						rows="3"
					></textarea>
					<p class="text-2xs text-secondary mt-1">
						The CA to check the server against. Needed for <span class="font-mono">verify-ca</span>
						and <span class="font-mono">verify-full</span> when the certificate is not signed by a public
						authority.
					</p>
				</div>
				<div>
					<span class="text-2xs text-secondary">Certificate verification</span>
					<Select
						items={CERT_VERIFICATION}
						bind:value={
							() => certVerification,
							(v) =>
								setAdvanced('accept_invalid_certs', v === 'default' ? undefined : v === 'accept')
						}
						clearable={false}
					/>
				</div>
				<Toggle
					size="xs"
					checked={wiz.own.advanced.use_iam_auth}
					on:change={(e) => setAdvanced('use_iam_auth', e.detail)}
					options={{ right: 'Authenticate with AWS IAM' }}
				/>
				{#if wiz.own.advanced.use_iam_auth}
					<div>
						<span class="text-2xs text-secondary">Region</span>
						<TextInput
							bind:value={() => wiz.own.advanced.region, (v) => setAdvanced('region', v)}
							inputProps={{ placeholder: 'us-east-1' }}
						/>
					</div>
				{/if}
			</div>
		{/if}
	</div>
{/snippet}

{#snippet poolerWarning()}
	{#if poolerUnavailable}
		<Alert type="warning" size="xs" title="Session pooling is not available">
			<div class="flex flex-col gap-1">
				<span>{poolerUnavailable}</span>
				<span>
					Windmill connects directly instead, which needs IPv6 from the workers, or the IPv4 add-on
					on the project. Granting the Supabase OAuth app
					<span class="font-mono">database_pooling_config_read</span> and connecting again restores the
					pooler.
				</span>
			</div>
		</Alert>
	{/if}
{/snippet}

{#snippet reviewStep()}
	<div>
		<span class="text-xs font-semibold text-emphasis">Data table name</span>
		<TextInput
			bind:value={wiz.review.name}
			error={!!nameError || !!nameConflict}
			inputProps={{ placeholder: 'main' }}
		/>
		<InputError error={nameError ?? (nameConflict || undefined)} />
		{#if !nameError && !nameConflict}
			<p class="text-2xs text-secondary mt-1">
				This is how your scripts will refer to it. <span class="font-mono">main</span> is used by default
				when a script does not name one.
			</p>
		{/if}
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
			<dd class="text-emphasis">{supabaseSummary(wiz).org ?? '—'}</dd>
			<dt class="text-secondary">Region</dt>
			<dd class="text-emphasis">{supabaseSummary(wiz).region ?? '—'}</dd>
			<dt class="text-secondary">Connection</dt>
			<dd class="text-emphasis">
				{wiz.supabase.connectionMode === 'session' ? 'Session pooler' : 'Direct (IPv6)'}
			</dd>
		</dl>
		{@render poolerWarning()}
	{:else if wiz.provider === 'instance'}
		<dl
			class="grid grid-cols-[9rem_1fr] gap-y-1 gap-x-3 text-xs border rounded-md p-3 border-border-light"
		>
			<dt class="text-secondary">Windmill database</dt>
			<dd class="text-emphasis font-mono">{wiz.instance.dbName}</dd>
		</dl>
	{:else if !wiz.own.creating}
		<dl
			class="grid grid-cols-[9rem_1fr] gap-y-1 gap-x-3 text-xs border rounded-md p-3 border-border-light"
		>
			<dt class="text-secondary">Postgres resource</dt>
			<dd class="text-emphasis font-mono">{wiz.own.resourcePath}</dd>
		</dl>
	{/if}

	{#if mintsResource}
		<Label label="Resource path" class="gap-2">
			<Path
				bind:path={
					() => resourcePath,
					(p) => {
						const cut = p.lastIndexOf('/')
						wiz.review.folder = cut > 0 ? p.slice(0, cut) : p
						wiz.review.resourceName = cut > 0 ? p.slice(cut + 1) : ''
					}
				}
				bind:error={resourcePathError}
				initialPath={initialResourcePath}
				checkInitialPathExistence
				namePlaceholder="database"
				kind="resource"
				autofocus={false}
			/>
			<InputError error={variablePathError} />
			<p class="text-2xs text-secondary">
				The connection is saved here as a Postgres resource, with its password in a secret variable
				beside it. Every script in the workspace can use
				<span class="font-mono">datatable://{wiz.review.name.trim()}</span> wherever you put it — the
				folder decides who can see and edit the connection itself, and who can reference the resource
				directly in a SQL step.
			</p>
		</Label>
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
