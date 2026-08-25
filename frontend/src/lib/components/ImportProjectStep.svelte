<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Alert, Button, Drawer, DrawerContent } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import FolderPicker from '$lib/components/FolderPicker.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { contentSummary } from '$lib/components/ProjectContentBadges.svelte'
	import type { ProjectMigration } from '$lib/components/workspaceSettings/projectBundle'
	import MigrationSqlEditor from '$lib/components/workspaceSettings/MigrationSqlEditor.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '$lib/components/common/confirmationModal/asyncConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { ImportExecution, plannedTasks } from '$lib/importWizard/execution.svelte'
	import SetupChecklist, { type SetupStep } from '$lib/components/wizards/SetupChecklist.svelte'
	import { beforeNavigate, goto } from '$app/navigation'
	import { untrack } from 'svelte'
	import { FOLDER_NAME_RE, planProblem, type ImportPlan } from '$lib/importWizard/plan'
	import type { ImportProjectSummary } from '$lib/components/ImportProjectCard.svelte'
	import { ArrowLeft, Download, Loader2 } from 'lucide-svelte'

	// The last step: it shows the plan, and it is the only step that can act on it.
	// All the acting lives in ImportExecution — this file decides what the user sees
	// and supplies the one interaction the run needs (the migration review).
	interface Props {
		plan: ImportPlan
		/** From the hub, for the counts — the export is only fetched during the run. */
		project?: ImportProjectSummary
		onFolderChange: (folder: string) => void
		onFinish: () => void
		/** True once the run reveals data tables the destination has yet to configure. */
		setupPending?: boolean
		/** The page has not settled whether a setup step follows. Finishing now would skip it. */
		setupUndecided?: boolean
		/** Hands the run to the page, which needs the export's data tables to know
		 * whether a setup step follows this one. */
		onExecution?: (execution: ImportExecution | undefined) => void
		/**
		 * The run this step already made, handed back when it is remounted. Step 4 unmounts
		 * this component, so returning from it would otherwise arrive at a fresh step with no
		 * run — offering Import again over a bundle that is already in, and on a new
		 * workspace failing at create because the finished run cleared its parking.
		 */
		resume?: ImportExecution | undefined
		onBack: () => void
	}

	let {
		plan,
		project,
		onFolderChange,
		onFinish,
		onBack,
		setupPending = false,
		setupUndecided = false,
		onExecution,
		resume
	}: Props = $props()

	let folder = $state(plan.folder ?? plan.slug)
	// The workspace the import will land in, when it is one that already exists. A
	// `new` destination has no workspace to list folders from until the run creates it.
	const existingWorkspace = $derived(
		plan.destination?.kind === 'existing' ? plan.destination.workspaceId : undefined
	)

	// The picker binds `folder`, so there is no blur event to report on. Mirror every
	// settled change into the plan. The page replaces rather than pushes for this
	// (`go(..., { replace: true })`), because a mirrored field is not a step the Back
	// button should have to walk through; a value equal to what the plan already holds
	// is skipped so this cannot loop.
	$effect(() => {
		const next = folder.trim()
		if (next && next !== (plan.folder ?? '')) onFolderChange(next)
	})
	const folderValid = $derived(FOLDER_NAME_RE.test(folder.trim()))
	const problem = $derived(planProblem({ ...plan, folder: folder.trim() }))

	// --- the run ---------------------------------------------------------------
	let reviewDrawer = $state<Drawer | undefined>()
	let reviewList = $state<
		{ datatable_name: string; sql: string; sql_down: string; run: boolean }[]
	>([])
	// Bumped per review session so the Monaco editors re-mount with the new SQL.
	let reviewGeneration = $state(0)
	/** `abort` stops the whole import; `false` only skips the migrations. */
	let reviewResolve: ((run: boolean | 'abort') => void) | undefined

	function openMigrationReview(migs: ProjectMigration[]): Promise<boolean | 'abort'> {
		reviewList = migs.map((m) => ({
			datatable_name: m.datatable_name,
			sql: m.sql,
			sql_down: m.sql_down ?? '',
			run: true
		}))
		reviewGeneration++
		reviewDrawer?.openDrawer()
		return new Promise<boolean | 'abort'>((resolve) => (reviewResolve = resolve))
	}
	function closeMigrationReview(run: boolean) {
		// Capture + clear first so the `on:close` fired by closeDrawer() (which would
		// call this again with run=false) can't override an explicit Run/Skip choice.
		const resolve = reviewResolve
		reviewResolve = undefined
		reviewDrawer?.closeDrawer()
		resolve?.(run)
	}

	/**
	 * Migrations are keyed by data table name and only apply to a target table of
	 * the same name. Returns what to run, or null to abort the import.
	 */
	async function reviewMigrations(
		workspace: string,
		migrations: ProjectMigration[]
	): Promise<ProjectMigration[] | null> {
		const enabled = migrations.filter((m) => m.enabled && (m.sql ?? '').trim() !== '')
		if (enabled.length === 0) return []

		let present: Set<string>
		try {
			present = new Set((await WorkspaceService.listDataTables({ workspace })).map((d) => d.name))
		} catch {
			// Can't read the target's data tables — skip migrations rather than guess.
			return []
		}
		// A migration whose data table does not exist here is not a reason to stop: the
		// setup step after this one configures those tables and runs them. Only the
		// ones that can run now are worth reviewing.
		const runnable = enabled.filter((m) => present.has(m.datatable_name))
		if (runnable.length === 0) return []
		const run = await openMigrationReview(runnable)
		// `abort` is the teardown case: the step is gone, so stop rather than import the
		// items without the tables the review was about.
		if (run === 'abort') return null
		if (!run) return []
		return reviewList
			.filter((r) => r.run && r.sql.trim() !== '')
			.map((r) => ({
				datatable_name: r.datatable_name,
				sql: r.sql,
				sql_down: r.sql_down,
				enabled: true
			}))
	}

	// One execution per plan, tagged with the plan it belongs to. Going back and
	// choosing a different destination leaves the old run behind the tag rather than
	// clearing it from an effect, so a previous run's outcome can never be shown
	// against another plan. The folder is deliberately not part of the tag: it is
	// pushed onto the existing run instead (see `start`).
	// Seeded from the handed-back run under *its own* tag, so the `planKey` guard below
	// still rejects it when the destination changed while this component was unmounted —
	// tagging it with the current plan would make that check pass by construction and show
	// one destination's finished checklist against another's plan.
	// `untrack`, because this is a mount-time snapshot on purpose.
	let run = $state<{ key: string; execution: ImportExecution } | undefined>(
		untrack(() => (resume ? { key: resume.planTag, execution: resume } : undefined))
	)
	const planKey = $derived(JSON.stringify(plan.destination) + plan.slug)
	const execution = $derived(run?.key === planKey ? run.execution : undefined)
	$effect(() => onExecution?.(execution))

	// What the import will bring, named on the row that brings it. Triggers and
	// migrations only become known once the export is fetched, so the phrase grows
	// mid-run rather than starting complete.
	const importSummary = $derived(
		project ? contentSummary({ ...project.counts, ...(execution?.extraCounts ?? {}) }) : ''
	)

	// The same rows before and during the run: the step states what it is about to do,
	// and the run fills those rows in rather than replacing a paragraph with a list.
	const tasks = $derived(execution?.tasks ?? plannedTasks(plan))

	// `SetupStep` carries no detail field, so what the row reports goes in the title
	// beside the label. The import row says what it is importing; every other row keeps
	// whatever the run reported. The breakdown supersedes the run's own "N items" here,
	// being the same total said in a more useful way.
	const checklist = $derived<SetupStep[]>(
		tasks.map((task) => {
			// The breakdown says what the import *will* bring, so it belongs to the row only
			// until the run has an outcome of its own. Left in place it would go on claiming
			// "2 apps, 4 scripts" over a run that wrote none of them because they were
			// already there.
			const detail = task.key === 'import' ? task.detail || importSummary : task.detail
			return {
				title: detail ? `${task.label} — ${detail}` : task.label,
				status: task.status,
				// Only under the row that wrote them. A failed item carries its error as the
				// description, which the checklist opens by itself.
				substeps:
					task.key === 'import'
						? execution?.itemResults.map((r) => ({
								title: r.path,
								// `skipped`, not `done`: nothing was written, and a green tick over an
								// item this run left alone claims an import that did not happen.
								status: !r.ok
									? ('failed' as const)
									: r.skipped
										? ('skipped' as const)
										: ('done' as const),
								description: r.skipped ? 'Already in the workspace — left as it is.' : r.error
							}))
						: undefined
			}
		})
	)
	/** A run that has been attempted — what makes the button read Retry rather than Import. */
	const attempted = $derived(!!execution)

	function start() {
		const current =
			execution ??
			new ImportExecution(
				{ ...plan, folder: folder.trim() },
				{ reviewMigrations, hasEeLicense: !!$enterpriseLicense }
			)
		// A retry reuses the execution — that is what keeps a created workspace and a
		// fetched export from being redone — so the folder, the one field still
		// editable after a failure, has to be pushed onto it before running again.
		current.setFolder(folder.trim())
		run = { key: planKey, execution: current }
		void current.run()
	}

	const leaveModal = createAsyncConfirmationModal()
	/** The question is on screen; a second attempt must not stack another one. */
	let askingToLeave = false
	/** The navigation the question approved, which has to get past this guard. */
	let leaveApproved = false

	// The browser's own back/forward, which the stepper's guard cannot see. Leaving
	// mid-run unmounts the migration review the executor may be awaiting, so it is
	// worth stopping for — but silently refusing reads as a broken back button, so
	// cancel, ask, and re-navigate if the answer is yes.
	beforeNavigate((nav) => {
		if (leaveApproved) return
		// Nothing in flight has anything to lose.
		if (!execution?.running) return
		if (askingToLeave) {
			nav.cancel()
			return
		}
		// Leaving the app entirely cannot be resumed from here — the browser owns that
		// prompt — so there is nothing to ask and nowhere to navigate back to.
		const to = nav.to?.url
		if (!to) {
			nav.cancel()
			return
		}
		nav.cancel()
		void confirmLeave(to)
	})

	async function confirmLeave(to: URL): Promise<void> {
		askingToLeave = true
		// `finally`, because this flag is what blocks a second attempt: an `ask` that threw
		// would otherwise leave the step permanently unleavable, since every path above
		// returns early on it.
		try {
			const landed = execution?.itemResults.length ?? 0
			const confirmed = await leaveModal.ask({
				title: 'Leave while the import is running?',
				confirmationText: 'Leave',
				type: 'danger',
				// A run that has already written items leaves them behind, so promising
				// otherwise would be a lie exactly when it matters most.
				children:
					(landed === 0
						? 'Nothing has been imported into the workspace yet.'
						: `${landed} item${landed === 1 ? '' : 's'} already imported into the workspace will stay there.`) +
					'<br /><br />The import stops where it is. Coming back to this link picks it up ' +
					'again without redoing what finished.'
			})
			if (!confirmed) return
			// Deliberately not re-read against `running`: the answer was about leaving, and a
			// run that finished in the meantime only makes leaving safer.
			leaveApproved = true
			// Stop the run before navigating. Nothing can abort a request already in flight,
			// so this stops it at the next phase boundary and keeps the workspace parked, so
			// the link the message promises actually resumes instead of failing on create.
			execution?.abandon()
			await goto(to)
		} finally {
			askingToLeave = false
		}
	}

	// Torn down with the review drawer open, the executor is still awaiting an answer.
	// Abort rather than resolve: resolving to `false` means "skip the migrations", which
	// would let the orphaned run import every item *without* the tables they need — the
	// opposite of leaving it where it was.
	$effect(() => () => reviewResolve?.('abort'))

	const deleteModal = createAsyncConfirmationModal()
	async function deleteWorkspace() {
		const id = execution?.workspaceId
		if (!id) return
		const ok = await deleteModal.ask({
			title: `Delete workspace ${id}?`,
			confirmationText: 'Delete it',
			children: 'It was created for this import. Deleting it cannot be undone.'
		})
		if (!ok) return
		try {
			await execution?.deleteCreatedWorkspace()
			sendUserToast(`Deleted workspace ${id}`)
			onBack()
		} catch (e: any) {
			sendUserToast(`Could not delete ${id}: ${e?.body ?? e}`, true)
		}
	}
</script>

<div class="flex flex-col gap-4">
	<!-- Only when the destination already exists. A workspace created by this run is
	     empty, so there is nothing for the project to sit next to and nothing to
	     choose between — asking would be a question with one answer. It lands in
	     f/<slug>/ either way; `installProject` creates the folder as it imports. -->
	{#if existingWorkspace}
		<div class="max-w-sm">
			<!-- The workspace rides on the field label rather than getting a line of its own:
			     the folder is the only thing being chosen, and naming its container is what
			     the label is for. Step 2 chose the workspace a screen ago, so this is a
			     reminder, not a control. -->
			<span class="block text-xs font-semibold text-emphasis">
				Folder inside <span class="font-mono">{existingWorkspace}</span>
			</span>
			<!-- Says where the items land, now that the path hint under the picker is gone. -->
			<p class="mb-1 text-xs font-normal text-secondary">
				Everything the project ships is imported into this folder.
			</p>
			<!-- Pointed at the destination rather than the active workspace: the run is
			     what enters it, and that has not happened yet on this step. -->
			<FolderPicker
				bind:folderName={folder}
				workspace={existingWorkspace}
				disabled={execution?.running || execution?.done}
				size="sm"
			/>
			{#if folder.trim() && !folderValid}
				<p class="mt-1 text-2xs font-normal text-red-500"
					>Letters, digits, dashes and underscores only.</p
				>
			{/if}
		</div>
	{/if}

	<!-- Heads the step the way the others do ("Where should it go?", "Name the new
	     workspace"), and in the same voice: a sentence, not a label. -->
	<h2 class="text-sm font-semibold text-emphasis">What this will do</h2>

	<!-- The run, task by task, so a failure says which part failed. Shown before the
	     run too, as the plan: every row starts pending and turns green in place. The
	     paths the import writes hang off the import task rather than forming a second
	     list: they are that task's output, not a parallel account of the same run.
	     `substepsClass` caps that list: a project ships tens of items where a data
	     table wizard step has a handful of checks. -->
	<SetupChecklist steps={checklist} substepsClass="max-h-52 overflow-y-auto" />

	{#if execution?.error}
		<Alert type="error" title="The import did not finish cleanly" size="xs">
			{execution.error}
		</Alert>
	{/if}

	<!-- `info`, not `warning`: nothing here has gone wrong, it is what import does. Borderless
	     so the collapsed row sits under the checklist as a note rather than competing with it
	     — `bgClass` is the only lever, the border is baked into each type's classes. -->
	<Alert
		type="info"
		title="What import does to resources and triggers"
		size="xs"
		bgClass="border-0"
		collapsible
	>
		Resources are imported as empty stubs — set their values after import; one whose path is
		already in the workspace is left exactly as it is and reported as already there, so a value
		you have since filled in is never overwritten. Trigger kinds are
		recreated disabled, except GCP and Azure triggers, which manage cloud subscriptions at creation
		and must be re-created manually after filling their resource. Kafka, NATS, SQS, GCP and Azure
		triggers all require Enterprise. Triggers that reference a resource depend on stubs imported
		empty, so fill in the resource value before re-enabling the trigger.
	</Alert>

	<div class="mt-2 flex items-center justify-between gap-2">
		<!-- Back is disabled mid-run, and gone once the import has landed: at that point
		     the plan has already happened and re-answering it would say nothing. -->
		{#if !execution?.done}
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: ArrowLeft }}
				disabled={execution?.running}
				onClick={onBack}
			>
				Back
			</Button>
		{:else}
			<span></span>
		{/if}

		<div class="flex items-center gap-2">
			<!-- Deleting is offered only while the run has not finished: once the items are
			     in, removing the workspace is not a cancel, it is a different decision. -->
			{#if execution?.createdWorkspace && !execution.done}
				<Button
					variant="subtle"
					unifiedSize="sm"
					disabled={execution.running}
					onClick={deleteWorkspace}
				>
					Delete workspace
				</Button>
			{/if}

			{#if execution?.done}
				<!-- A finished run that reports failures is still finished — what landed is
				     real — but it must stay actionable: without this the only way out of a
				     failed migration or a failed item is to leave, and nothing downstream
				     can run the SQL. Offered beside Finish rather than instead of it, so a
				     migration that fails every time cannot trap the user short of step 4. -->
				{#if execution.error}
					<Button
						variant="subtle"
						unifiedSize="sm"
						disabled={execution.running}
						startIcon={{ icon: execution.running ? Loader2 : Download }}
						onClick={start}
					>
						Retry
					</Button>
				{/if}
				<!-- Disabled while the page is still deciding whether a setup step follows:
				     finishing in that window leaves for the workspace and skips a step that
				     the answer, a moment later, says was needed. -->
				<Button
					variant="accent"
					unifiedSize="sm"
					disabled={setupUndecided}
					startIcon={setupUndecided ? { icon: Loader2 } : undefined}
					onClick={onFinish}
				>
					{setupUndecided ? 'Checking…' : setupPending ? 'Continue →' : 'Finish setup →'}
				</Button>
			{:else}
				<Button
					variant="accent"
					unifiedSize="sm"
					startIcon={{ icon: execution?.running ? Loader2 : Download }}
					disabled={!!problem || !folderValid || execution?.running}
					title={problem}
					onClick={start}
				>
					{#if execution?.running}
						Importing…
					{:else if attempted}
						Retry
					{:else if plan.destination?.kind === 'new'}
						Create workspace and import
					{:else}
						Import
					{/if}
				</Button>
			{/if}
		</div>
	</div>
</div>

<Portal>
	<ConfirmationModal {...deleteModal.props} />
	<ConfirmationModal {...leaveModal.props} />
</Portal>

<Drawer bind:this={reviewDrawer} size="700px" on:close={() => closeMigrationReview(false)}>
	<DrawerContent title="Data table migrations" on:close={() => closeMigrationReview(false)}>
		<div class="flex flex-col gap-4">
			<!-- Unconditional, because this drawer cannot open for anything else: `reviewMigrations`
			     keeps only migrations whose data table is already present in the destination, and a
			     workspace this run just created has none. Everything listed here therefore targets a
			     table that already exists and may already hold rows. -->
			<Alert type="warning" title="These run against data tables that already exist" size="xs">
				{reviewList.length === 1 ? 'This data table is' : 'These data tables are'} already set up{existingWorkspace
					? ` in ${existingWorkspace}`
					: ''} and may already hold data. These migrations were written to create the project's tables,
				so running them here can alter or drop what is in them. Read the SQL before you run it, and skip
				anything you are unsure of.
			</Alert>
			<p class="text-xs text-secondary">
				Review and edit the SQL, then choose which to run. A migration runs against the data table
				of the same name in the destination workspace; if that data table has migrations enabled it
				is recorded, otherwise it runs once as a preview job.
			</p>
			{#each reviewList as m (m.datatable_name)}
				<div class="flex flex-col gap-1.5 rounded border bg-surface-secondary p-2 text-xs">
					<div class="flex items-center justify-between gap-2">
						<span class="font-mono text-primary">{m.datatable_name}</span>
						<Toggle bind:checked={m.run} size="xs" options={{ right: 'Run' }} />
					</div>
					{#if m.run}
						<MigrationSqlEditor
							bind:up={m.sql}
							bind:down={m.sql_down}
							generation={reviewGeneration}
						/>
					{/if}
				</div>
			{/each}
		</div>
		{#snippet actions()}
			<Button variant="subtle" unifiedSize="sm" onClick={() => closeMigrationReview(false)}>
				Skip migrations
			</Button>
			<Button
				variant="accent"
				unifiedSize="sm"
				disabled={!reviewList.some((m) => m.run && m.sql.trim() !== '')}
				onClick={() => closeMigrationReview(true)}
			>
				Run selected
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
