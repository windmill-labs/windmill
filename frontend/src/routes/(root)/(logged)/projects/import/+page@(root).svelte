<script lang="ts">
	import { page } from '$app/stores'
	import { base } from '$app/paths'
	import { goto } from '$lib/navigation'
	import { Button } from '$lib/components/common'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import RadioCard from '$lib/components/common/radioCard/RadioCard.svelte'
	import ImportProjectCard, {
		type ImportProjectSummary
	} from '$lib/components/ImportProjectCard.svelte'
	import { fetchHubProject, hubBrowserUrl } from '$lib/hubProject'
	import ImportProjectStep from '$lib/components/ImportProjectStep.svelte'
	import ImportSetupStep from '$lib/components/ImportSetupStep.svelte'
	import ImportWizardSteps from '$lib/components/ImportWizardSteps.svelte'
	import type { ImportExecution } from '$lib/importWizard/execution.svelte'
	import WorkspaceTreeView from '$lib/components/workspace/WorkspaceTreeView.svelte'
	import { superadmin, usersWorkspaceStore } from '$lib/stores'
	import { get } from 'svelte/store'
	import { resource } from 'runed'
	import { isCloudHosted } from '$lib/cloud'
	import { WorkspaceService, type UserWorkspaceList } from '$lib/gen'
	import { canCreateWorkspace, loadUsernamePolicy } from '$lib/workspaceCreation'
	import { toWorkspaceId, validateWorkspaceId } from '$lib/utils/workspaceId'
	import { validateUsername } from '$lib/utils'
	import {
		readPlan,
		planToSearch,
		planWorkspaceId,
		type ImportDestination,
		type ImportPlan,
		type WizardStep
	} from '$lib/importWizard/plan'
	import {
		ArrowLeft,
		Building,
		ChevronsDownUp,
		ChevronsUpDown,
		Loader2,
		Plus,
		Search
	} from 'lucide-svelte'

	// This page only ever *describes* an import. The plan lives in the URL, so the
	// browser's back button and the stepper are the same operation — a URL change —
	// and neither can strand a half-created workspace, because nothing is created
	// until the last step runs it. See lib/importWizard/plan.ts.
	const parsed = $derived(readPlan($page.url))
	const plan = $derived(parsed.plan)
	const step = $derived(parsed.step)
	const slug = $derived(plan.slug)
	const logoutReturnTo = $derived($page.url.pathname + $page.url.search)

	// `replace` for anything the page decides on its own — a correction, or mirroring
	// a field into the plan. Those must not become history entries: the user did not
	// ask for them, and a pushed entry is one the Back button has to walk through
	// before it reaches the step they actually came from. `goto` forwards its options
	// to SvelteKit, which defaults `replaceState` to false.
	function go(next: Partial<ImportPlan>, step: WizardStep, opts?: { replace?: boolean }) {
		goto(`/projects/import${planToSearch({ ...plan, ...next }, step)}`, {
			replaceState: opts?.replace ?? false
		})
	}

	// ---------------------------------------------------------------- permissions
	let canCreate = $state($superadmin || isCloudHosted())
	$effect(() => {
		if ($superadmin) canCreate = true
	})
	if (!canCreate) {
		void canCreateWorkspace(false).then((c) => (canCreate = c))
	}

	let automateUsername = $state(true)
	let username = $state('')
	void loadUsernamePolicy()
		.then((p) => {
			automateUsername = p.automate
			if (p.suggested && !username) username = p.suggested
		})
		.catch(() => {})

	// ------------------------------------------------------------------- the project
	// Straight from the hub, cross-origin: this runs before there is a workspace to
	// proxy through. A failure is not fatal — the wizard still works, the card just
	// shows the slug and the choices drop their item counts.
	const projectResource = resource(
		() => slug || undefined,
		async (s): Promise<ImportProjectSummary | undefined> =>
			s ? await fetchHubProject(s) : undefined
	)
	const project = $derived(projectResource.current)
	const projectError = $derived(!!projectResource.error)

	let hubHost = $state('hub.windmill.dev')
	void hubBrowserUrl()
		.then((u) => (hubHost = new URL(u).host))
		.catch(() => {})

	const itemCount = $derived(
		project ? Object.values(project.counts).reduce((a: number, b: number) => a + b, 0) : 0
	)
	const itemsLabel = $derived(itemCount > 0 ? `the ${itemCount} items` : 'everything in it')

	// --------------------------------------------------------------------- step 1
	let chosen = $state<'new' | 'existing' | undefined>(undefined)
	// The plan is consulted before the default so coming back from step 2 shows the
	// choice that was made there. `canCreate` starts false and only turns true once
	// the superadmin refresh or the settings fetch lands, so the fallback is derived
	// rather than written at init.
	const choice = $derived(chosen ?? plan.destination?.kind ?? (canCreate ? 'new' : 'existing'))

	// --------------------------------------------------------------------- step 2
	let name = $state(plan.destination?.kind === 'new' ? plan.destination.name : '')
	let id = $state(plan.destination?.kind === 'new' ? plan.destination.id : toWorkspaceId(plan.slug))
	let idTaken = $state(false)
	let checkingId = $state(false)
	// The hub's own name for the project, once it arrives, unless the user has typed.
	$effect(() => {
		const p = projectResource.current
		if (p && !name) name = p.name
	})

	/** Free id nearest the prefill: `-2`, `-3`, … so re-importing a project works. */
	async function freeId(candidate: string): Promise<string> {
		for (let n = 1; n <= 20; n++) {
			const next = n === 1 ? candidate : `${candidate}-${n}`
			if (validateWorkspaceId(next)) break
			if (!(await WorkspaceService.existsWorkspace({ requestBody: { id: next } }))) return next
		}
		return candidate
	}

	async function checkId() {
		const candidate = id.trim()
		if (!candidate) return
		checkingId = true
		try {
			idTaken = await WorkspaceService.existsWorkspace({ requestBody: { id: candidate } })
		} catch {
			idTaken = false
		} finally {
			checkingId = false
		}
	}

	const idProblem = $derived(id.trim() ? validateWorkspaceId(id.trim()) : undefined)
	// Only when the instance does not derive it, which is the only case the field is shown.
	// `create_workspace` accepts whatever it is sent — `Some("")` passes its one check and is
	// written to `usr.username` verbatim — so this is the only thing standing between a
	// cleared field and a workspace whose owner has no username.
	const usernameProblem = $derived(
		automateUsername
			? undefined
			: !username.trim()
				? 'A username is required'
				: validateUsername(username.trim()) || undefined
	)
	// Step 2 shows the workspace list when step 1 chose "one I already have".
	const choiceIsExisting = $derived(plan.destination?.kind === 'existing')

	let filter = $state('')
	let allExpanded = $state(false)
	let hasForks = $state(false)
	let expandCollapseAll = $state<(() => void) | undefined>(undefined)

	// Loaded once step 2 actually asks for a workspace. The store is read through
	// `get` rather than `$usersWorkspaceStore`: the fetcher writes that store, and a
	// tracked read of it here would make the resource re-run its own result.
	const workspaceList = resource(
		() => (step === 2 && choiceIsExisting ? true : undefined),
		async (needed) => {
			if (!needed) return undefined
			const list = get(usersWorkspaceStore) ?? (await WorkspaceService.listUserWorkspaces())
			usersWorkspaceStore.set(list)
			return list.workspaces.filter((w) => !w.disabled)
		}
	)
	const workspaces = $derived<UserWorkspaceList['workspaces']>(workspaceList.current ?? [])

	// ----------------------------------------------------------------- transitions
	// Steps 2 and 3 both refine an answer step 1 gives, so a URL that reaches them
	// without a destination is missing that answer rather than holding a default.
	$effect(() => {
		if (step > 1 && !plan.destination) go({}, 1, { replace: true })
	})

	// Suffixing happens here rather than reactively on step 2: it is a consequence of
	// choosing to create a workspace, and `-2`, `-3`, … let a project be imported
	// twice. Checking is a read — the wizard still creates nothing at this point.
	async function step1Continue() {
		if (choice === 'new') {
			checkingId = true
			try {
				id = await freeId(id.trim() || toWorkspaceId(slug))
				idTaken = false
			} finally {
				checkingId = false
			}
		}
		const destination: ImportDestination =
			choice === 'new'
				? { kind: 'new', name: name.trim() || slug, id: id.trim(), username: username || undefined }
				: // Which workspace is step 2's question; the plan records the kind now so
					// the two answers stay distinguishable in the URL.
					{
						kind: 'existing',
						workspaceId:
							plan.destination?.kind === 'existing' ? plan.destination.workspaceId : undefined
					}
		go({ destination }, 2)
	}

	/** Step 2, new workspace: records the name. Still creates nothing. */
	function confirmNewWorkspace() {
		go(
			{
				destination: {
					kind: 'new',
					name: name.trim(),
					id: id.trim(),
					username: automateUsername ? undefined : username.trim()
				}
			},
			3
		)
	}

	/** Step 2, existing workspace: records the choice, exactly like the real picker's click. */
	async function pickExisting(workspaceId: string) {
		go({ destination: { kind: 'existing', workspaceId } }, 3)
	}

	/**
	 * The wizard sits outside the `(logged)` layout, so leaving it mounts that layout
	 * and the workspace home for the first time — a second or more of loading with the
	 * finished wizard still on screen, which reads as a dead button. Hand the screen
	 * over to a loader on the way out; the navigation unmounts it.
	 */
	let leaving = $state(false)
	function finish() {
		leaving = true
		// The run has already switched to the destination workspace.
		goto('/')
	}

	// Whether a fourth step exists. Known only once the run has fetched the export and
	// the destination's data tables can be compared against it, so it is false for the
	// whole wizard until the import finishes — which is exactly when it is first read.
	let execution = $state<ImportExecution | undefined>(undefined)
	let setupNeeded = $state(false)
	// True while the answer is still being fetched. Without it the run reads as finished
	// with no fourth step, and Finish leaves for the workspace before the check comes back
	// and discovers a data table that is missing.
	let setupUndecided = $state(false)
	$effect(() => {
		const names = execution?.datatableNames ?? []
		const workspace = planWorkspaceId(plan)
		if (!execution?.done || !workspace) {
			setupNeeded = false
			setupUndecided = false
			return
		}
		// `resourceCount` is the referenced subset — the resources something in the project
		// points at — and each one arrives as an empty stub, so any project that has them has
		// something to fill in. The step itself re-checks and shows only what is genuinely
		// outstanding, which is what makes a re-import quiet.
		if (execution.resourceCount > 0) {
			setupNeeded = true
			setupUndecided = false
			return
		}
		if (names.length === 0) {
			setupNeeded = false
			setupUndecided = false
			return
		}
		let cancelled = false
		setupUndecided = true
		void WorkspaceService.listDataTables({ workspace })
			.then((tables) => {
				if (cancelled) return
				const present = new Set(tables.map((t) => t.name))
				setupNeeded = names.some((n) => !present.has(n))
			})
			.catch(() => {
				// Can't tell — don't invent a step the user then cannot complete.
				if (!cancelled) setupNeeded = false
			})
			.finally(() => {
				if (!cancelled) setupUndecided = false
			})
		return () => (cancelled = true)
	})
</script>

{#if leaving}
	<CenteredModal title="Opening your workspace" centerVertically={false}>
		<div class="flex items-center gap-2 text-xs text-secondary">
			<Loader2 size={16} class="animate-spin" />
			Taking you to {planWorkspaceId(plan) ?? 'your workspace'}…
		</div>
	</CenteredModal>
{:else if !slug}
	<CenteredModal title="Nothing to import" centerVertically={false}>
		<p class="text-xs text-secondary">
			This page needs a <span class="font-mono">?hub=&lt;slug&gt;</span> to know which project to import.
			Open it from a project on the hub.
		</p>
	</CenteredModal>
{:else}
	<!-- One width for all three steps: `large` on step 3 alone made the modal jump
	     from 640px to 1024px mid-wizard, which reads as a different page. -->
	<!-- Same configuration as the workspace picker page: the modal sizes to its
	     content and the workspace tree scrolls inside its own max-h-[50vh].
	     `containOverflow` would stretch the modal to the full screen height. -->
	<CenteredModal title="Import a project" centerVertically={false}>
		{#snippet subtitleSnippet()}
			<span class="text-xs text-tertiary">
				Connected as <span class="font-medium text-secondary">{$usersWorkspaceStore?.email}</span>
				{#if step === 1}
					·
					<a
						class="text-blue-500 hover:underline"
						href="{base}/user/logout?rd={encodeURIComponent(logoutReturnTo)}"
					>
						Switch account
					</a>
				{/if}
			</span>
		{/snippet}
		<!-- Once the run is gone, the import step behind us has nothing to resume and would
		     offer to run the whole bundle again — over items already in, or over a create
		     whose workspace now exists. That is only reachable after a reload on step 4, so
		     the floor rises exactly then. -->
		<ImportWizardSteps
			{step}
			hasSetup={setupNeeded || step === 4}
			lowestStep={step === 4 && !execution ? 4 : 1}
		/>

		{#if step === 1}
			<div class="flex flex-col gap-6">
				{#if project}
					<ImportProjectCard {project} {hubHost} />
				{:else if projectError}
					<p class="text-xs text-secondary">
						Could not read <span class="font-mono">{slug}</span> from the hub. You can still choose a
						destination — the project is fetched again before it is imported.
					</p>
				{:else}
					<div class="flex items-center gap-2 text-xs text-secondary">
						<Loader2 size={14} class="animate-spin" /> Loading {slug}…
					</div>
				{/if}

				<div class="flex flex-col gap-3">
					<h2 class="text-sm font-semibold text-emphasis">Where should it go?</h2>

					<!-- The whole card is the control: one choice at a time, shown by the
					     border and tint rather than a dot, hence `showRadio={false}`. -->
					<div role="radiogroup" aria-label="Where should it go?" class="flex flex-col gap-3">
						{#if canCreate}
							<RadioCard
								label="A new workspace"
								showRadio={false}
								selected={choice === 'new'}
								onSelect={() => (chosen = 'new')}
							>
								{#snippet icon()}
									<Plus size={14} class="text-secondary" />
								{/snippet}
								{#snippet description()}
									Creates <span class="font-medium text-primary">{name || slug}</span> and imports
									{itemsLabel} into it.
								{/snippet}
							</RadioCard>
						{/if}

						<RadioCard
							label="A workspace I already have"
							description="Imports {itemsLabel} into a workspace you already use."
							showRadio={false}
							selected={choice === 'existing'}
							onSelect={() => (chosen = 'existing')}
						>
							{#snippet icon()}
								<Building size={14} class="text-secondary" />
							{/snippet}
						</RadioCard>
					</div>

					<div class="mt-2 flex items-center justify-end">
						<Button unifiedSize="sm" variant="accent" onClick={step1Continue}>Continue →</Button>
					</div>
				</div>
			</div>
		{:else if step === 2}
			<div class="flex flex-col gap-4">
				{#if !choiceIsExisting}
					<div>
						<h2 class="text-sm font-semibold text-emphasis">Name the new workspace</h2>
					</div>

					<div class="grid grid-cols-2 gap-3">
						<label class="flex flex-col gap-1">
							<span class="text-xs font-normal text-secondary">Workspace name</span>
							<TextInput size="sm" bind:value={name} />
						</label>
						<label class="flex flex-col gap-1">
							<span class="text-xs font-normal text-secondary">Workspace ID</span>
							<TextInput size="sm" bind:value={id} inputProps={{ onblur: checkId }} />
							{#if idProblem}
								<span class="text-2xs font-normal text-red-500">{idProblem}</span>
							{:else if idTaken}
								<span class="text-2xs font-normal text-red-500">ID already exists.</span>
							{/if}
						</label>
					</div>

					{#if !automateUsername}
						<label class="flex max-w-[50%] flex-col gap-1">
							<span class="text-xs font-normal text-secondary">Your username in it</span>
							<TextInput size="sm" bind:value={username} />
							{#if usernameProblem && username.trim()}
								<span class="text-2xs font-normal text-red-500">{usernameProblem}</span>
							{/if}
						</label>
					{/if}
				{:else}
					<div>
						<h2 class="text-sm font-semibold text-emphasis">Pick a workspace</h2>
						<p class="mt-0.5 text-xs text-secondary">The project is imported into this one.</p>
					</div>

					{#if workspaceList.loading}
						<div class="flex items-center gap-2 text-xs text-secondary">
							<Loader2 size={14} class="animate-spin" /> Loading your workspaces…
						</div>
					{:else if workspaceList.error}
						<p class="text-xs text-red-500">
							Could not list your workspaces. Reload the page, or go back and create a new one.
						</p>
					{:else if workspaces.length === 0}
						<p class="text-xs text-secondary">
							You are not a member of any workspace yet. Go back and create one, or ask an admin to
							invite you.
						</p>
					{:else}
						<!-- The same tree the workspace picker renders: forks nested under their
						     parent, search, colours, roles. Clicking a row is the choice, exactly as
						     clicking one there enters the workspace — so this branch has no Continue. -->
						{#if workspaces.length > 1}
							<div class="flex items-center gap-2">
								<div class="relative flex-1 text-primary">
									<TextInput
										size="sm"
										bind:value={filter}
										inputProps={{ placeholder: 'Search workspaces...' }}
										class="!pr-8"
									/>
									<Search size={14} class="absolute right-2 top-2 text-secondary" />
								</div>
								{#if hasForks}
									<Button
										onClick={() => expandCollapseAll?.()}
										title={allExpanded ? 'Collapse all' : 'Expand all'}
										startIcon={{ icon: allExpanded ? ChevronsDownUp : ChevronsUpDown }}
										unifiedSize="2xs"
										variant="default"
									>
										{allExpanded ? 'Collapse' : 'Expand'}
									</Button>
								{/if}
							</div>
						{/if}

						<!-- No scroll wrapper here: the tree's own root is `max-h-[50vh]
						     overflow-auto` and doubles as its keyboard scroll container, so
						     wrapping it produced two scrollbars, the outer one tighter. -->
						<WorkspaceTreeView
							{workspaces}
							onEnterWorkspace={pickExisting}
							bind:searchFilter={filter}
							bind:allExpanded
							bind:hasForks
							bind:onExpandCollapseAll={expandCollapseAll}
						/>
					{/if}
				{/if}

				<div class="mt-2 flex items-center justify-between gap-2">
					<Button
						unifiedSize="sm"
						variant="subtle"
						startIcon={{ icon: ArrowLeft }}
						onClick={() => go({}, 1)}
					>
						Back
					</Button>
					{#if !choiceIsExisting}
						<Button
							unifiedSize="sm"
							variant="accent"
							disabled={!name.trim() ||
								!id.trim() ||
								!!idProblem ||
								!!usernameProblem ||
								idTaken ||
								checkingId}
							onClick={confirmNewWorkspace}
						>
							Continue →
						</Button>
					{/if}
				</div>
			</div>
		{:else if step === 3}
			<ImportProjectStep
				{plan}
				{project}
				setupPending={setupNeeded}
				{setupUndecided}
				onFolderChange={(folder) => go({ folder }, 3, { replace: true })}
				onFinish={() =>
					setupNeeded
						? // Replaces rather than pushes: after a reload on step 4 the run is gone, and
							// a step-3 entry in history is a browser-Back route to the same fresh import
							// the stepper is now blocked from reaching.
							go({}, 4, { replace: true })
						: finish()}
				onBack={() => go({}, 2)}
				onExecution={(e) => (execution = e)}
				resume={execution}
			/>
		{:else}
			<ImportSetupStep
				workspace={planWorkspaceId(plan) ?? ''}
				{slug}
				folder={plan.folder}
				onSkip={finish}
				onFinish={finish}
				onBack={// Only while this page still holds the run. After a reload it does not, and a
				// step 3 with no run offers Import again — over a bundle that is already in,
				// and on a new workspace over a create that would now fail, because the
				// finished run cleared its parking.
				execution ? () => go({}, 3) : undefined}
			/>
		{/if}
	</CenteredModal>
{/if}
