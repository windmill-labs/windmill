<script lang="ts">
	import { page } from '$app/stores'
	import { base } from '$app/paths'
	import { goto } from '$lib/navigation'
	import { Button } from '$lib/components/common'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import ImportProjectCard, {
		type ImportProjectSummary
	} from '$lib/components/ImportProjectCard.svelte'
	import { fetchHubProject, hubBrowserUrl } from '$lib/hubProject'
	import ImportProjectStep from '$lib/components/ImportProjectStep.svelte'
	import ImportWizardSteps from '$lib/components/ImportWizardSteps.svelte'
	import WorkspaceTreeView from '$lib/components/workspace/WorkspaceTreeView.svelte'
	import { superadmin, usersWorkspaceStore } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { SettingService, UserService, WorkspaceService, type UserWorkspaceList } from '$lib/gen'
	import {
		readPlan,
		planToSearch,
		toWorkspaceId,
		WORKSPACE_ID_RE,
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

	function go(next: Partial<ImportPlan>, step: WizardStep) {
		goto(`/projects/import${planToSearch({ ...plan, ...next }, step)}`)
	}

	// ---------------------------------------------------------------- permissions
	// Same gate the workspace picker uses for its own create button: on a self-hosted
	// instance CREATE_WORKSPACE_REQUIRE_SUPERADMIN defaults to true, and an offer that
	// ends in a 403 is worse than no offer.
	let canCreate = $state($superadmin || isCloudHosted())
	$effect(() => {
		if ($superadmin) canCreate = true
	})
	if (!canCreate) {
		fetch(base + '/api/workspaces/create_workspace_require_superadmin')
			.then((r) => r.text())
			.then((t) => (canCreate = t != 'true'))
			.catch(() => {})
	}

	// createWorkspace rejects a username when the instance automates them, and
	// requires one when it does not (backend users.rs:1166) — so the field only
	// exists in the second case, and only then does it travel in the plan.
	let automateUsername = $state(true)
	let username = $state('')
	SettingService.getGlobal({ key: 'automate_username_creation' })
		.then((v) => {
			automateUsername = (v as boolean | null) ?? true
			if (!automateUsername && !username) {
				UserService.globalWhoami()
					.then((u) => (username = (u.name?.split(' ')[0] || u.email.split('@')[0]).toLowerCase()))
					.catch(() => {})
			}
		})
		.catch(() => {})

	// ------------------------------------------------------------------- the project
	// Straight from the hub, cross-origin: this runs before there is a workspace to
	// proxy through. A failure is not fatal — the wizard still works, the card just
	// shows the slug and the choices drop their item counts.
	let project = $state<ImportProjectSummary | undefined>(undefined)
	let projectError = $state(false)
	let fetchedFor: string | undefined = undefined
	let hubHost = $state('hub.windmill.dev')
	void hubBrowserUrl()
		.then((u) => (hubHost = new URL(u).host))
		.catch(() => {})

	$effect(() => {
		if (!slug || fetchedFor === slug) return
		fetchedFor = slug
		projectError = false
		void fetchHubProject(slug)
			.then((p) => {
				project = p
				if (!name) name = p.name
			})
			.catch(() => (projectError = true))
	})

	const itemCount = $derived(
		project ? Object.values(project.counts).reduce((a: number, b: number) => a + b, 0) : 0
	)
	const itemsLabel = $derived(itemCount > 0 ? `the ${itemCount} items` : 'everything in it')

	// --------------------------------------------------------------------- step 1
	let chosen = $state<'new' | 'existing' | undefined>(undefined)
	// `canCreate` starts false and only turns true once the superadmin refresh or the
	// settings fetch lands, so the default is derived rather than written at init.
	const choice = $derived(chosen ?? (canCreate ? 'new' : 'existing'))

	// --------------------------------------------------------------------- step 2
	let name = $state(plan.destination?.kind === 'new' ? plan.destination.name : '')
	let id = $state(plan.destination?.kind === 'new' ? plan.destination.id : '')
	let idTaken = $state(false)
	let checkingId = $state(false)
	$effect(() => {
		if (!slug || id) return
		id = toWorkspaceId(slug)
	})

	/** Free id nearest the prefill: `-2`, `-3`, … so re-importing a project works. */
	async function freeId(candidate: string): Promise<string> {
		for (let n = 1; n <= 20; n++) {
			const next = n === 1 ? candidate : `${candidate}-${n + 1}`
			if (!(await WorkspaceService.existsWorkspace({ requestBody: { id: next } }))) return next
		}
		return candidate
	}

	// Suffix once when step 2 opens on a taken prefill; after that the user owns the
	// field and only gets the inline error. Checking is a read, not a change — the
	// wizard still creates nothing here.
	let suffixedFor: string | undefined = undefined
	$effect(() => {
		if (step !== 2 || choiceIsExisting || !id || suffixedFor === slug) return
		suffixedFor = slug
		void (async () => {
			checkingId = true
			try {
				id = await freeId(id)
				idTaken = false
			} finally {
				checkingId = false
			}
		})()
	})

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

	const idValid = $derived(WORKSPACE_ID_RE.test(id.trim()))
	// Step 2 shows the workspace list when step 1 chose "one I already have", which
	// the plan records by *not* carrying a new-workspace destination.
	const choiceIsExisting = $derived(plan.destination?.kind !== 'new')

	let workspaces = $state<UserWorkspaceList['workspaces']>([])
	let listLoading = $state(false)
	let listError = $state(false)
	let filter = $state('')
	let allExpanded = $state(false)
	let hasForks = $state(false)
	let expandCollapseAll = $state<(() => void) | undefined>(undefined)

	$effect(() => {
		if (step !== 2 || !choiceIsExisting || workspaces.length > 0 || listLoading) return
		listLoading = true
		void (async () => {
			try {
				const list = $usersWorkspaceStore ?? (await WorkspaceService.listUserWorkspaces())
				usersWorkspaceStore.set(list)
				workspaces = list.workspaces.filter((w) => !w.disabled)
			} catch {
				listError = true
			} finally {
				listLoading = false
			}
		})()
	})

	// ----------------------------------------------------------------- transitions
	function step1Continue() {
		const destination: ImportDestination | undefined =
			choice === 'new'
				? { kind: 'new', name: name.trim() || slug, id: id.trim(), username: username || undefined }
				: undefined
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

	function finish() {
		// The run has already switched to the destination workspace.
		goto('/')
	}
</script>

{#if !slug}
	<CenteredModal title="Nothing to import" centerVertically={false}>
		<p class="text-sm text-secondary">
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
		<ImportWizardSteps {step} />

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
					     border and tint rather than a dot. Radio semantics for screen readers. -->
					{#if canCreate}
						<button
							type="button"
							role="radio"
							aria-checked={choice === 'new'}
							class="w-full rounded-lg border p-4 text-left font-normal transition {choice === 'new'
								? 'border-blue-400/70 bg-blue-50/60 dark:bg-blue-900/20'
								: 'border-border-light hover:bg-surface-hover'}"
							onclick={() => (chosen = 'new')}
						>
							<h3 class="flex items-center gap-1.5 text-xs font-semibold text-emphasis">
								<Plus size={14} class="text-secondary" /> A new workspace
							</h3>
							<p class="mt-0.5 text-xs text-secondary">
								Creates <span class="font-medium text-primary">{name}</span> and imports {itemsLabel}
								into it.
							</p>
						</button>
					{/if}

					<button
						type="button"
						role="radio"
						aria-checked={choice === 'existing'}
						class="w-full rounded-lg border p-4 text-left font-normal transition {choice ===
						'existing'
							? 'border-blue-400/70 bg-blue-50/60 dark:bg-blue-900/20'
							: 'border-border-light hover:bg-surface-hover'}"
						onclick={() => (chosen = 'existing')}
					>
						<h3 class="flex items-center gap-1.5 text-xs font-semibold text-emphasis">
							<Building size={14} class="text-secondary" /> A workspace I already have
						</h3>
						<p class="mt-0.5 text-xs text-secondary">
							Imports {itemsLabel} into a workspace you already use.
						</p>
					</button>

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
						<div class="flex flex-col gap-1">
							<span class="text-[11px] font-medium uppercase tracking-wide text-tertiary">Name</span
							>
							<TextInput size="sm" bind:value={name} />
						</div>
						<div class="flex flex-col gap-1">
							<span class="text-[11px] font-medium uppercase tracking-wide text-tertiary">ID</span>
							<TextInput size="sm" bind:value={id} inputProps={{ onblur: checkId }} />
							{#if id.trim() && !idValid}
								<span class="text-[11px] text-red-600">
									Lowercase letters, digits and dashes only.
								</span>
							{:else if idTaken}
								<span class="text-[11px] text-red-600">ID already exists.</span>
							{/if}
						</div>
					</div>

					{#if !automateUsername}
						<div class="flex max-w-[50%] flex-col gap-1">
							<span class="text-[11px] font-medium uppercase tracking-wide text-tertiary">
								Your username in it
							</span>
							<TextInput size="sm" bind:value={username} />
						</div>
					{/if}
				{:else}
					<div>
						<h2 class="text-sm font-semibold text-emphasis">Pick a workspace</h2>
						<p class="mt-0.5 text-xs text-secondary">The project is imported into this one.</p>
					</div>

					{#if listLoading}
						<div class="flex items-center gap-2 text-xs text-secondary">
							<Loader2 size={14} class="animate-spin" /> Loading your workspaces…
						</div>
					{:else if listError}
						<p class="text-xs text-red-600">
							Could not list your workspaces. Reload the page, or go back and create a new one.
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
										size="xs2"
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
						onClick={() => go({ destination: undefined }, 1)}
					>
						Back
					</Button>
					{#if !choiceIsExisting}
						<Button
							unifiedSize="sm"
							variant="accent"
							disabled={!name.trim() || !idValid || idTaken || checkingId}
							onClick={confirmNewWorkspace}
						>
							Continue →
						</Button>
					{/if}
				</div>
			</div>
		{:else}
			<ImportProjectStep
				{plan}
				{project}
				onFolderChange={(folder) => go({ folder }, 3)}
				onFinish={finish}
				onBack={() => go({}, 2)}
			/>
		{/if}
	</CenteredModal>
{/if}
