<script lang="ts">
	import { parkWizard, type WizardResume } from './wizardParking'
	import type { Snippet } from 'svelte'
	import { Database, ArrowRight, Plus } from 'lucide-svelte'
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
	import Section from '../Section.svelte'
	import SupabaseIcon from '../icons/SupabaseIcon.svelte'
	import {
		FolderService,
		OauthService,
		ResourceService,
		UserService,
		type User,
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
	import { probeDatatableConnection } from './datatableProbe'
	import { logDatatableWizard } from './datatableTelemetry'
	import {
		anythingClaimed,
		claimOf,
		claimsFromJSON,
		claimsToJSON,
		noClaims,
		release,
		type Claims
	} from './setupClaims'
	import { userStore, workspaceStore } from '$lib/stores'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import {
		composePostgresConnectionString,
		parsePostgresConnectionString,
		connectionParamRefusal
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
		type CreatedProject,
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
		/** Name to open with, when the caller needs a table of a particular name rather
		 * than whatever the user picks — the import wizard configures the one a project's
		 * migrations target.
		 *
		 * Locked when `onFinishAlso` is also given, because that work targets this name and
		 * nothing carries an edit through to it: renaming `main` to `other` would create
		 * `other`, then run the migrations against `main`, fail, and leave a data table
		 * nobody asked for. Editable without one, where the name is only a name. */
		initialName?: string
		/** Where the dialog portals to. `#content` is the app shell's scroll container,
		 * which only exists inside the `(logged)` layout; a page reparented out of it
		 * (the hub import wizard) has to say `body` or the portal finds nothing and the
		 * dialog never appears. */
		modalTarget?: string
		/** What the caller does once the table exists, named on the final button so the
		 * user is told before pressing it — the import wizard runs the project's
		 * migrations, which is otherwise invisible until it has already happened. */
		finishAlso?: string
		/** The work `finishAlso` names. Run as the last checklist step, so it reports
		 * where the rest of the run does instead of starting after the dialog closes.
		 * Throwing marks that step failed; the table itself is already made either way. */
		onFinishAlso?: () => Promise<void>
		/** The workspace everything here is created in and checked against.
		 *
		 * Defaults to `$workspaceStore`, which is right for the settings page — it is the
		 * workspace being looked at. The import wizard is the exception: its page is
		 * reparented out of `(logged)`, so nothing re-runs the layout's workspace
		 * persistence, and after a reload the store still names whatever workspace the
		 * user came from while the plan in the URL names the destination. Left ambient,
		 * this would create the data table in one workspace and run the project's
		 * migrations in the other. */
		workspace?: string
	}

	let {
		opened = $bindable(),
		existingNames,
		existingDataTables,
		resume,
		onDone,
		customInstanceDbs,
		confirmationModal,
		defaultInstanceDbName,
		initialName,
		modalTarget = '#content',
		finishAlso,
		onFinishAlso,
		workspace: workspaceProp
	}: Props = $props()

	/**
	 * The caller needs this exact table, and has follow-up work bound to its name.
	 *
	 * Captured when the dialog opens rather than read live: `initialName` is the caller's
	 * `wizardFor`, which it clears from `onDone` — and that fires after a *failed* run too,
	 * while the dialog stays up offering Back. Reading it live releases the lock exactly when
	 * the user is most likely to edit, which is the divergence the lock exists to stop.
	 */
	let nameLocked = $state(false)

	/** Every write and every check goes through this, never `$workspaceStore` directly. */
	const targetWorkspace = $derived(workspaceProp ?? $workspaceStore ?? '')

	/**
	 * Who the caller is *in the destination*, which is not who `$userStore` describes.
	 *
	 * `$userStore` is the membership of the workspace the app is in. Routing the API calls
	 * elsewhere without routing this leaves the username behind: after a reload on the import
	 * wizard's step 4 it names the workspace the user came from, and a resource path built
	 * from it lands on `u/<someone-else>` inside the destination — failing an ownership check,
	 * or for an admin, quietly putting database credentials in another member's namespace.
	 */
	let targetUser = $state<User | undefined>(undefined)
	const aimedElsewhere = $derived(!!workspaceProp && workspaceProp !== $workspaceStore)
	const ambientUsername = $derived($userStore?.username ?? '')
	const targetUsername = $derived(aimedElsewhere ? (targetUser?.username ?? '') : ambientUsername)
	/** The destination's membership could not be read, so nothing here knows who the user is. */
	let membershipFailed = $state(false)

	async function loadTargetUser(): Promise<void> {
		const ws = workspaceProp
		if (!ws || ws === $workspaceStore) {
			targetUser = undefined
			membershipFailed = false
			return
		}
		try {
			targetUser = await UserService.whoami({ workspace: ws })
			membershipFailed = false
		} catch {
			// Recorded rather than swallowed: an unknown username silently becomes `admin` in
			// the default path, which is the wrong namespace to write credentials into. Setup
			// is blocked instead.
			targetUser = undefined
			membershipFailed = true
		}
	}

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
	 * Whichever namespace refused this path. The run writes a resource *and* a secret variable
	 * at the one path, and both writes upsert, so anything already sitting at either would be
	 * overwritten. `Path` reports the resource namespace itself; the effect below covers the
	 * variable one, and the check in `finish()` covers both.
	 */
	let pathTakenError = $state('')
	let variableCheck: ReturnType<typeof setTimeout> | undefined = undefined
	/**
	 * Whether a check for `path` still answers the question on screen. Read once before the
	 * request and again after it: the cleanup can cancel a pending timer but not a request
	 * already in flight, so an answer that arrives late has to re-earn its relevance. `path`
	 * alone is not enough -- it is built from the review step's fields, so picking an existing
	 * resource stops the wizard minting one without changing it.
	 */
	function pathStillChecked(path: string): boolean {
		return (
			path === resourcePath && wiz.step === 3 && mintsResource && !!path && path !== claimedPath
		)
	}

	$effect(() => {
		const path = resourcePath
		// A path this wizard already wrote is not somebody else's to protect -- the same
		// exemption the hard check in `finish()` makes, or a retry would refuse its own secret
		// and leave Finish permanently disabled.
		if (!pathStillChecked(path)) {
			pathTakenError = ''
			return
		}
		clearTimeout(variableCheck)
		variableCheck = setTimeout(async () => {
			const taken = await VariableService.existsVariable({ workspace: targetWorkspace, path })
			// Two checks can be in flight at once and resolve out of order. A `false` for a path
			// nobody is on any more would clear the error guarding the one about to be written;
			// a `true` would disable Finish over a path this run stopped caring about.
			if (!pathStillChecked(path)) return
			pathTakenError = taken ? 'a variable already exists at this path' : ''
		}, 500)
		return () => clearTimeout(variableCheck)
	})

	/**
	 * Both namespaces at once, for the check that has to be right rather than quick. While the
	 * user is still typing the two halves are reported separately -- `Path` does the resource
	 * one, the debounced effect above does the variable one -- and either debounce can still be
	 * in flight when Finish is pressed.
	 */
	async function pathConflictMessage(path: string): Promise<string | undefined> {
		const workspace = targetWorkspace
		// Each namespace answers to its own claim. Holding the secret says nothing about who owns
		// the resource beside it, so one claim must not wave the other's check through.
		const [variable, resource] = await Promise.all([
			claimedPath === path ? false : VariableService.existsVariable({ workspace, path }),
			claimedResourcePath === path ? false : ResourceService.existsResource({ workspace, path })
		])
		if (variable) return 'a variable already exists at this path'
		if (resource) return 'a resource already exists at this path'
		return undefined
	}
	/** Furthest step reached, so going back to check something does not cost the progress. */
	let maxStep = $state(1)

	function defaultProjectName(): string {
		return `windmill-${targetWorkspace || 'workspace'}`
	}

	function defaultTableName(): string {
		// A caller that needs a specific name wins over the usual "main, unless taken":
		// the import wizard's migrations only apply to a table of the name they target.
		if (initialName) return initialName
		return existingNames.includes('main') ? `${targetWorkspace || 'data'}_datatable` : 'main'
	}

	// Takes the list rather than reading it, so the fetch that loads it can seed off its own
	// result before the resource has settled.
	function defaultFolder(list: string[] = folders): string {
		// The first folder this admin can write to, so the resource lands somewhere the team
		// can find and repair. A workspace with no folders falls back to the personal space.
		return list.length ? `f/${list[0]}` : `u/${targetUsername || 'admin'}`
	}

	// A row this run wrote and could not take back out is still its own: `removeRow` reports
	// `kept` when the undo cannot reach the server, and counting that name as taken refuses the
	// retry on the one name the user has every right to reuse.
	let nameError = $derived(
		datatableNameError(
			wiz.review.name,
			existingNames.filter((n) => n !== claimedName)
		)
	)
	// Every database on the instance, not just the data table ones: the name has to be free
	// in PostgreSQL, and a collision with a database created for something else still fails.
	let instanceNameError = $derived(
		wiz.provider === 'instance' && wiz.instance.mode === 'create'
			? instanceDbNameError(
					wiz.instance.dbName ?? '',
					// `setup_custom_instance_db` registers the name whether it succeeds or fails, so
					// after a failed attempt the instance holds a database this very wizard asked
					// for. Counting it as taken would refuse the retry, on the step the user cannot
					// see it from.
					Object.keys(customInstanceDbs.current ?? {}).filter((n) => n !== claimedInstanceDb)
				)
			: undefined
	)
	let connectionStringError = $derived.by(() => {
		const text = wiz.own.connectionString
		if (!text) return undefined
		if (!parsePostgresConnectionString(text)) return 'That is not a Postgres connection string.'
		return connectionParamRefusal(text)
	})
	let resourcePath = $derived(resourcePathOf(wiz))

	/**
	 * Another data table on the same database. They would share one schema, so their tables are
	 * visible to each other and two of the same name collide. Migration bookkeeping is keyed by
	 * data table, so that part stays separate.
	 */
	let sharesDatabaseWith = $derived(
		wiz.provider === 'resource' && !wiz.own.creating && wiz.own.resourcePath
			? existingDataTables.find((d) => d.resourcePath === wiz.own.resourcePath)
			: undefined
	)

	const pgResources = resource(
		() => (opened && wiz.provider === 'resource' ? targetWorkspace : ''),
		async (workspace) => {
			if (!workspace) return undefined
			const list = await ResourceService.listResource({ workspace, resourceType: 'postgresql' })
			// The step opens on a choice rather than on nothing: the first resource where the
			// workspace has any, the creation form where it has none. Only ever fills an empty
			// selection, so it cannot overwrite what the user picked.
			if (!wiz.own.creating && !wiz.own.resourcePath) {
				if (list.length) wiz.own.resourcePath = list[0].path
				else wiz.own.creating = true
			}
			return list
		}
	)

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
	 * A connection string is a way of writing the fields down, so typing one fills them in as it
	 * is read. Unparseable text leaves the last good values alone to be corrected; `sslmode` is
	 * kept when the URI names none, since not mentioning it is not the same as clearing it.
	 */
	function absorbConnectionString(text: string) {
		const parts = parsePostgresConnectionString(text)
		if (parts) wiz.own.fields = { ...parts, sslmode: parts.sslmode ?? wiz.own.fields.sslmode }
	}

	function useFields() {
		wiz.own.form = 'fields'
	}

	/** Composed for the box to start from; the fields stay the connection. */
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

	// The Supabase branch goes through the instance's `supabase_wizard` OAuth client; where a
	// superadmin has not configured one, the connect endpoint dead-ends, so do not offer it.
	const oauthConnects = resource(
		() => opened,
		async (isOpen) => (isOpen ? await OauthService.listOauthConnects() : [])
	)
	let supabaseAvailable = $derived(
		(oauthConnects.current ?? []).some((c) => c.name === 'supabase_wizard')
	)

	const folderNames = resource(
		() => (opened ? targetWorkspace : ''),
		async (workspace) => {
			if (!workspace) return []
			const all = await FolderService.listFolderNames({ workspace })
			const usable = all.filter((x) => !['app_groups', 'app_custom', 'app_themes'].includes(x))
			// The list lands after `open()` has already seeded the folder, so the first open would
			// otherwise always fall back to the personal space. Only re-seed what the user has not
			// reached yet: the review step is where the folder becomes theirs. A resumed run has
			// already chosen one -- reseeding would move its secret out from under the path claim
			// it came back to finish.
			if (wiz.step < 3 && !resumedPath) wiz.review.folder = defaultFolder(usable)
			return usable
		}
	)
	let folders = $derived(folderNames.current ?? [])

	const SUPABASE_SIGNUP_URL = 'https://supabase.com/dashboard/sign-up'

	const supaOauth = useSupabaseOauth({
		// Safe to navigate this tab away: what the wizard had collected is parked first, and the
		// settings page picks it back up when Supabase sends the user home.
		redirectIfBlocked: true,
		onPopupBlocked: () =>
			parkWizard({
				name: wiz.review.name,
				region: wiz.supabase.region,
				projectName: wiz.supabase.projectName,
				resourcePath,
				claims: claimsToJSON(claims),
				createdProjects,
				mode: wiz.supabase.mode,
				org: wiz.supabase.org,
				project: wiz.supabase.project,
				connectionMode: wiz.supabase.connectionMode
			})
	})

	/**
	 * The path a resumed run came back to finish, taken from whatever `reset` was given rather
	 * than the `resume` prop, so the two cannot disagree about which run is being restored.
	 */
	let resumedPath = $state<string | undefined>(undefined)

	function reset(from: WizardResume | undefined) {
		nameLocked = !!initialName && !!onFinishAlso
		resumedPath = from?.resourcePath
		// A pending confirmation that never settled leaves `dismissing` true, and `finally`
		// cannot clear what never resolves — so a fresh open always starts dismissable.
		dismissing = false
		wiz = newWizardState({
			name: from?.name || defaultTableName(),
			projectName: from?.projectName || defaultProjectName(),
			folder: defaultFolder()
		})
		wiz.supabase.region = from?.region ?? DEFAULT_SUPABASE_REGION
		run = { steps: [], running: false }
		maxStep = 1
		// What the last run claimed belongs to the data table it created; a fresh one has to
		// earn the name and the path again, or it would write over its predecessor's secret.
		claims = noClaims
		claimedInstanceDb = undefined
		leftBehind = false
		createdProjects = []
		nameConflictFor = undefined
		lastFailure = ''
		finishAlsoFailed = false
		pathTakenError = ''
		poolerUnavailable = undefined
		if (from) {
			wiz.provider = 'supabase'
			// The clears above are for a fresh run. This one is the same run coming back from the
			// redirect, so what it had already created is still its own to write over.
			claims = claimsFromJSON(from.claims)
			createdProjects = from.createdProjects ?? []
			leftBehind = anythingClaimed(claims) || createdProjects.length > 0
			// Which side of the toggle it was on, and the organization it was pointed at. Left to
			// default, a run that died mid-create comes back asking for the password it generated.
			if (from.mode) wiz.supabase.mode = from.mode
			if (from.org) wiz.supabase.org = from.org
			// The password is deliberately not parked -- it is a secret and sessionStorage is not
			// the place for one. Carrying the project is what stops the resume landing on a
			// different database with an empty password field and no sign anything moved.
			if (from.project) wiz.supabase.project = from.project
			if (from.connectionMode) wiz.supabase.connectionMode = from.connectionMode
			const cut = from.resourcePath?.lastIndexOf('/') ?? -1
			if (from.resourcePath && cut > 0) {
				wiz.review.folder = from.resourcePath.slice(0, cut)
				wiz.review.resourceName = from.resourcePath.slice(cut + 1)
			}
			enterStep(2)
		}
	}

	/**
	 * Opened by the settings page. A run coming back from the Supabase redirect is handed in
	 * rather than read off the `resume` prop: the caller has it, and taking it as an argument is
	 * what keeps the restore independent of when the prop it was assigned to reaches this
	 * component.
	 */
	export async function open(parked?: WizardResume) {
		// Awaited before `reset`, which seeds the resource path from `defaultFolder()` and so
		// needs the destination's username. Seeding first and correcting later loses whenever
		// the folder list resolves first, and never corrects at all if `whoami` fails.
		await loadTargetUser()
		reset(parked ?? resume)
		opened = true
		logDatatableWizard({ step: 'opened' })
	}

	function selectProvider(key: Provider) {
		if (key === wiz.provider) return
		wiz.provider = key
		logDatatableWizard({ step: 'picked', provider: key })
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
		// Read off one attempt against one project, so it does not survive a change of inputs:
		// the review step would otherwise warn about a limitation that does not apply to what
		// it is describing, while claiming session pooling right above it.
		poolerUnavailable = undefined
		// Same for the failure carried back to the review step: it names inputs that have since
		// been edited, so it would describe a run nobody can still act on.
		lastFailure = ''
		finishAlsoFailed = false
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
	 * Proves the connection before anything is written. Supabase reads the pooler first: the
	 * value under test has to be the value that will be saved.
	 */
	// Fields stay editable while a check is out. Without a token the older answer lands after
	// an edit and unlocks Continue for a connection nobody proved.
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
			// An existing resource goes as a reference the worker resolves, exactly as a Postgres
			// step would take it; anything unsaved goes as the value itself.
			const database = value ?? (wiz.own.resourcePath ? `$res:${wiz.own.resourcePath}` : undefined)
			if (!database) {
				settle({ checking: false, report: undefined, error: undefined })
				return
			}
			const report = await probeDatatableConnection(targetWorkspace, database)
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
	 * A run in this session left something behind -- a row, a secret, a database, a project.
	 * It outlives the checklist, which `backToReview` clears, because what was created does too.
	 */
	let leftBehind = $state(false)
	/**
	 * Every Supabase project this session created, each guarding the path holding its only
	 * password. Held here rather than read off `run.result`: the checklist is cleared by every
	 * return to review, and the projects are not. A set, because two attempts at two paths each
	 * leave a password nothing later may write over.
	 */
	let createdProjects = $state<CreatedProject[]>([])
	/** The instance database this session asked for, which is registered even when it failed. */
	let claimedInstanceDb = $state<string | undefined>(undefined)
	/**
	 * What this run created and may write over again. `writeRow` merges into whatever the
	 * server holds under the name, so a name free in the table on screen but taken on the
	 * server would repoint someone else's data table at this database.
	 */
	let claims = $state<Claims>(noClaims)
	/** The path whose secret and resource this run holds, for the gates that ask by path. */
	let claimedPath = $derived(claimOf(claims, 'secret', resourcePath)?.path)
	let claimedResourcePath = $derived(claimOf(claims, 'resource', resourcePath)?.path)
	let claimedName = $derived(claims.find((c) => c.kind === 'row')?.path)
	/**
	 * The name the pre-flight refused, kept with the message so editing the name retires it.
	 * Storing only the message would need something to clear it, and the check that raises it
	 * runs where a clear cannot see the edit that follows.
	 */
	let nameConflictFor = $state<{ name: string; message: string } | undefined>(undefined)
	let nameConflict = $derived(
		nameConflictFor?.name === wiz.review.name.trim() ? nameConflictFor.message : ''
	)
	/** Why the last run failed, kept on the review step after the checklist is dropped. */
	let lastFailure = $state('')
	/**
	 * The appended `onFinishAlso` step failed while `runSetup` itself succeeded. Tracked apart
	 * from `run.result`, which stays the setup's own verdict: the data table really was
	 * created, so a retry must re-run only this last step. Re-running the setup would ask for
	 * the table name it has just taken, and be refused as a duplicate.
	 */
	let finishAlsoFailed = $state(false)

	/**
	 * A refused pre-flight means nothing ran, so the checklist from a previous attempt has to
	 * give way -- it is the only thing rendered while it exists, and the refusal is shown on
	 * the review step.
	 */
	function backToReview() {
		// The checklist is the only place a failure is reported, and this drops it -- so what it
		// said has to come with us. It is the whole reason the user is on their way back: which
		// project to rename to, which path not to write over.
		lastFailure = run.result?.ok ? '' : (run.result?.error ?? '')
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

	/**
	 * A failed run can always be sent back to be corrected. What a run already created is
	 * protected by `runSetup` refusing to mint a second Supabase password over the first one's
	 * variable, not by keeping the fields out of reach -- a lock here would only take away the
	 * rename that undoes the mistake.
	 */
	let canEditAfterFailure = $derived(
		!!run.steps.length && !run.running && !submitting && !run.result?.ok
	)

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
				const settings = await WorkspaceService.getSettings({ workspace: targetWorkspace })
				if (settings.datatable?.datatables?.[name]) {
					nameConflictFor = {
						name,
						message: `A data table called ${name} already exists in this workspace.`
					}
					backToReview()
					return
				}
				nameConflictFor = undefined
			}
			// The debounced path check is advisory -- it can still be in flight when Finish is
			// pressed -- and the writes that follow replace a secret and a resource in place. Ask
			// once more here, where refusing costs nothing; what this run already holds is exempt
			// per namespace inside, so Try again can repair its own half-finished attempt.
			if (mintsResource) {
				const conflict = await pathConflictMessage(resourcePath)
				if (conflict) {
					pathTakenError = conflict
					backToReview()
					return
				}
			}
		} catch (err: any) {
			// Everything inside the run reports through the checklist; this runs before there is
			// one, so it has to speak for itself rather than fail silently.
			nameConflictFor = {
				name,
				message: `Could not check the name: ${err?.body ?? err?.message ?? String(err)}`
			}
			backToReview()
			return
		}
		run = { steps: planSteps(wiz), running: true }
		lastFailure = ''
		finishAlsoFailed = false
		// The database is registered by the call whatever it answers, so asking for one is
		// already leaving something behind.
		if (wiz.provider === 'instance' && wiz.instance.mode === 'create') {
			claimedInstanceDb = wiz.instance.dbName?.trim()
			leftBehind = true
		}
		let result: RunResult | undefined = undefined
		try {
			result = await runSetup(wiz, {
				workspace: targetWorkspace,
				supabaseToken: supaOauth.token,
				onInstanceDbsChanged: async () => {
					await customInstanceDbs.refetch()
				},
				onProgress: (steps) => (run.steps = steps),
				onPoolerUnavailable: (reason) => (poolerUnavailable = reason),
				createdProjects,
				claims,
				username: targetUsername
			})
		} finally {
			// The caller's own finishing work, appended to the same checklist. It only runs
			// on a clean setup: there is no table for it to act on otherwise.
			if (result?.ok && onFinishAlso && finishAlso) {
				const title = finishAlso.charAt(0).toUpperCase() + finishAlso.slice(1)
				run.steps = [...run.steps, { title, status: 'running' }]
				try {
					await onFinishAlso()
					run.steps = run.steps.map((s, i) =>
						i === run.steps.length - 1 ? { ...s, status: 'done' as const } : s
					)
				} catch (err: any) {
					const description = err?.body ?? err?.message ?? String(err)
					run.steps = run.steps.map((s, i) =>
						i === run.steps.length - 1 ? { ...s, status: 'failed' as const, description } : s
					)
					finishAlsoFailed = true
				}
			}
			// `runSetup` catches per step, but anything escaping it would otherwise leave the
			// button spinning with a page reload the only way out.
			// Kept, not replaced: what an earlier attempt wrote is still out there, so a later
			// one failing sooner must not hand its own objects back to the collision checks.
			createdProjects = result?.createdProjects ?? createdProjects
			claims = result?.claims ?? claims
			// A row taken back out frees its name again, and free is somebody else's to take.
			if (result?.rowRolledBack) claims = release(claims, 'row', name)
			leftBehind = anythingClaimed(claims) || createdProjects.length > 0 || !!claimedInstanceDb
			run = {
				...run,
				running: false,
				result: result ?? {
					ok: false,
					error: 'The setup stopped unexpectedly.',
					claims,
					createdProjects
				}
			}
			// The setup's own verdict, so a data table that exists counts as done even when the
			// caller's appended `onFinishAlso` step failed after it.
			if (wiz.provider) {
				logDatatableWizard({ step: result?.ok ? 'done' : 'failed', provider: wiz.provider })
			}
			onDone()
		}
	}

	/**
	 * Whether closing would throw away work. A failed run counts: its inputs are still editable
	 * and it may have left something behind. A run in flight cannot be closed at all, and one
	 * that made its data table has nothing left to lose — including when `onFinishAlso` failed
	 * afterwards, because the table is real and working and the caller owns what is left. The
	 * import step, the only caller that passes one, shows that failure on its own row with a
	 * way to run it again and will not let Finish through while it stands.
	 */
	function hasUnfinishedIntent(): boolean {
		return wiz.provider !== undefined && !run.running && !run.result?.ok
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
		// `finally`, because the flag is what blocks a second attempt: an `ask` that throws
		// would otherwise leave the dialog permanently undismissable — the backdrop, Escape
		// and the close button all return early here, so the only way out would be a reload.
		try {
			const confirmed = await confirmationModal.ask({
				title: 'Leave without adding a data table?',
				// A run that failed and was sent back to be edited leaves whatever it got through
				// behind it, so promising otherwise would be a lie exactly when it matters most.
				children: leftBehind
					? 'The setup that already ran left what it created behind, and what you have filled in here will be lost.'
					: 'Nothing has been created yet, and what you have filled in here will be lost.',
				confirmationText: 'Discard'
			})
			// Re-read rather than trust the entry check: a run can start while the dialog is up, and
			// answering Discard would otherwise tear the modal down in the middle of it.
			if (confirmed && !preventClose) close()
		} finally {
			dismissing = false
		}
	}

	/** Re-runs only the appended step, which is the only thing that failed. */
	async function retryFinishAlso() {
		if (!onFinishAlso || !finishAlso) return
		const title = finishAlso.charAt(0).toUpperCase() + finishAlso.slice(1)
		finishAlsoFailed = false
		run = {
			...run,
			running: true,
			steps: [...run.steps.slice(0, -1), { title, status: 'running' as const }]
		}
		try {
			await onFinishAlso()
			run.steps = run.steps.map((s, i) =>
				i === run.steps.length - 1 ? { ...s, status: 'done' as const } : s
			)
		} catch (err: any) {
			const description = err?.body ?? err?.message ?? String(err)
			run.steps = run.steps.map((s, i) =>
				i === run.steps.length - 1 ? { ...s, status: 'failed' as const, description } : s
			)
			finishAlsoFailed = true
		} finally {
			run = { ...run, running: false }
			onDone()
		}
	}

	function close() {
		opened = false
	}

	// The single primary action. Its label says what it is about to do, and doing it is what
	// moves the wizard on.
	let primary = $derived.by(() => {
		// Ahead of everything: without the destination's membership the resource path would be
		// guessed, and a guess here writes database credentials into somebody else's namespace.
		if (aimedElsewhere && membershipFailed)
			return { label: 'Cannot read your access to this workspace', disabled: true }
		if (submitting && !run.running)
			return { label: 'Setting things up', disabled: true, busy: true }
		if (run.steps.length) {
			if (run.running) return { label: 'Setting things up', disabled: true, busy: true }
			// Before the `ok` check: the setup succeeded and the step after it did not, so
			// "Done" would be offered over a failed row.
			if (finishAlsoFailed)
				return { label: 'Try again', disabled: false, act: retryFinishAlso }
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
		const created =
			wiz.provider === 'supabase' && wiz.supabase.mode === 'create'
				? 'Create project and data table'
				: 'Create data table'
		return {
			label: finishAlso ? `${created} and ${finishAlso}` : created,
			disabled:
				// Guards the way back as well as the way forward: the stepper can return to step 2,
				// and not every control there invalidates the review it just made stale.
				!intentComplete(wiz) ||
				!wiz.review.name.trim() ||
				!!nameError ||
				!wiz.review.resourceName.trim() ||
				!!resourcePathError ||
				!!pathTakenError ||
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
	target={modalTarget}
	formStyling
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
					<div class="flex flex-col gap-8">
						{@render reviewStep()}
					</div>
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
						{:else if canEditAfterFailure}
							<!-- Try again repeats the same inputs, so it cannot help a run that failed on
							one of them -- a database name already taken on the instance, a project name
							Supabase refuses. Dropping the checklist puts the earlier steps back within
							reach, which is the only way to edit those. -->
							<Button size="xs" variant="default" onClick={backToReview}>Back</Button>
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
							class="text-accent hover:underline">create one for free</a
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
		).filter((w) => w !== targetWorkspace)}
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
				{@const others = (db.used_by_workspaces ?? []).filter((w) => w !== targetWorkspace)}
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
						absorbConnectionString(v)
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
		<Section label="Advanced" small collapsable>
			<div class="flex flex-col gap-2">
				<div>
					<span class="text-2xs text-secondary">Root certificate</span>
					<TextInput
						underlyingInputEl="textarea"
						bind:value={
							() => wiz.own.advanced.root_certificate_pem,
							(v) => setAdvanced('root_certificate_pem', String(v ?? ''))
						}
						class="min-h-16 font-mono text-2xs resize-y"
						inputProps={{ placeholder: '-----BEGIN CERTIFICATE-----', rows: 3 }}
					/>
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
		</Section>
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
	{#if lastFailure}
		<Alert type="error" size="xs" bgClass="border-0" title="">{lastFailure}</Alert>
	{/if}
	<Label label="Data table name" class="gap-1">
		<p class="text-2xs text-secondary">
			This is how your scripts reach the database behind it:
			<span class="font-mono">datatable://{wiz.review.name.trim() || 'main'}</span>
			reads and writes it, whatever it is connected to.
			<span class="font-mono">main</span> is used by default when a script does not name one.
		</p>
		<TextInput
			bind:value={wiz.review.name}
			error={!!nameError || !!nameConflict}
			inputProps={{ placeholder: 'main', disabled: nameLocked }}
		/>
		{#if nameLocked}
			<p class="text-2xs text-secondary">
				Fixed: the project's migrations target this name, and they run against it whatever
				this table ends up called.
			</p>
		{/if}
		<InputError error={nameError ?? (nameConflict || undefined)} />
	</Label>

	{#if wiz.provider === 'supabase'}
		<!-- A card carrying Supabase's own mark rather than a disabled field: this is the one
		thing on the step that will not live in Windmill, and it should not read as a greyed-out
		Windmill input. -->
		<Label
			label={wiz.supabase.mode === 'create' ? 'New Supabase project' : 'Supabase project'}
			class="gap-1"
		>
			<p class="text-2xs text-secondary">
				{#if wiz.supabase.mode === 'create'}
					It does not exist yet — Windmill creates it on Supabase when you finish. The project is
					yours, and you can open and manage it from the Supabase dashboard.
				{:else}
					A project already in your Supabase account. Windmill saves its connection and changes
					nothing about the project itself.
				{/if}
			</p>
			<div class="border border-border-light rounded-md p-3 flex gap-3 items-start">
				<span class="mt-0.5 shrink-0"><SupabaseIcon height="18px" width="18px" /></span>
				<span class="flex flex-col gap-0.5 min-w-0">
					<span class="text-xs font-medium text-emphasis">
						{wiz.supabase.mode === 'create'
							? wiz.supabase.projectName
							: (wiz.supabase.project?.name ?? '')}
					</span>
					<span class="text-xs text-secondary">
						{[
							supabaseSummary(wiz).org,
							supabaseSummary(wiz).region,
							wiz.supabase.connectionMode === 'session' ? 'session pooler' : 'direct (IPv6)'
						]
							.filter(Boolean)
							.join(' · ')}
					</span>
				</span>
			</div>
		</Label>
		{@render poolerWarning()}
	{:else if wiz.provider === 'instance'}
		<Label label="Windmill database" class="gap-1">
			<p class="text-2xs text-secondary">
				{#if wiz.instance.mode === 'create'}
					It does not exist yet — Windmill creates it on this instance's PostgreSQL server when you
					finish, and manages its credentials.
				{:else}
					A database already on this instance's PostgreSQL server, managed by Windmill.
				{/if}
			</p>
			<TextInput value={wiz.instance.dbName ?? ''} inputProps={{ disabled: true }} />
		</Label>
	{:else if !wiz.own.creating}
		<Label label="Postgres resource" class="gap-1">
			<p class="text-2xs text-secondary">
				The connection already in this workspace. The data table points at it; nothing is written to
				it here.
			</p>
			<TextInput value={wiz.own.resourcePath ?? ''} inputProps={{ disabled: true }} />
		</Label>
	{/if}

	{#if mintsResource}
		<Label label="Resource path" class="gap-1">
			<p class="text-2xs text-secondary">
				The connection {wiz.provider === 'supabase' ? 'to the Supabase project ' : ''}is saved here
				as a Postgres resource, with its password in a secret variable beside it.
			</p>
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
				allowedExistingPath={claimedResourcePath}
				namePlaceholder="database"
				kind="resource"
				autofocus={false}
			/>
			<InputError error={pathTakenError} />
		</Label>
	{/if}

	{#if sharesDatabaseWith}
		<Alert type="warning" size="xs" bgClass="border-0" title="">
			<span class="font-semibold">{sharesDatabaseWith.name}</span> already uses this database. Both data
			tables would write to the same schema, so each one's tables are visible to the other and two tables
			of the same name collide. Migrations are tracked per data table, so those stay separate.
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
