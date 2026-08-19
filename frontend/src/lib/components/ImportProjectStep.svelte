<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Alert, Button, Drawer, DrawerContent } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import FolderPicker from '$lib/components/FolderPicker.svelte'
	import { WorkspaceService } from '$lib/gen'
	import ProjectContentBadges from '$lib/components/ProjectContentBadges.svelte'
	import type { ProjectMigration } from '$lib/components/workspaceSettings/projectBundle'
	import MigrationSqlEditor from '$lib/components/workspaceSettings/MigrationSqlEditor.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '$lib/components/common/confirmationModal/asyncConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { ImportExecution } from '$lib/importWizard/execution.svelte'
	import { beforeNavigate } from '$app/navigation'
	import { FOLDER_NAME_RE, planProblem, type ImportPlan } from '$lib/importWizard/plan'
	import type { ImportProjectSummary } from '$lib/components/ImportProjectCard.svelte'
	import { ArrowLeft, Check, Download, Loader2, X } from 'lucide-svelte'

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
		/** Hands the run to the page, which needs the export's data tables to know
		 * whether a setup step follows this one. */
		onExecution?: (execution: ImportExecution | undefined) => void
		onBack: () => void
	}

	let {
		plan,
		project,
		onFolderChange,
		onFinish,
		onBack,
		setupPending = false,
		onExecution
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

	const destinationLabel = $derived(
		plan.destination?.kind === 'new'
			? `${plan.destination.id} (new)`
			: (plan.destination?.workspaceId ?? 'nowhere yet')
	)

	// --- the run ---------------------------------------------------------------
	let reviewDrawer = $state<Drawer | undefined>()
	let reviewList = $state<
		{ datatable_name: string; sql: string; sql_down: string; run: boolean }[]
	>([])
	// Bumped per review session so the Monaco editors re-mount with the new SQL.
	let reviewGeneration = $state(0)
	let reviewResolve: ((run: boolean) => void) | undefined

	function openMigrationReview(migs: ProjectMigration[]): Promise<boolean> {
		reviewList = migs.map((m) => ({
			datatable_name: m.datatable_name,
			sql: m.sql,
			sql_down: m.sql_down ?? '',
			run: true
		}))
		reviewGeneration++
		reviewDrawer?.openDrawer()
		return new Promise((resolve) => (reviewResolve = resolve))
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
	let run = $state<{ key: string; execution: ImportExecution } | undefined>(undefined)
	const planKey = $derived(JSON.stringify(plan.destination) + plan.slug)
	const execution = $derived(run?.key === planKey ? run.execution : undefined)
	$effect(() => onExecution?.(execution))

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

	// The browser's own back/forward, which the stepper's guard cannot see. Leaving
	// mid-run unmounts the migration review the executor may be awaiting.
	beforeNavigate((nav) => {
		if (execution?.running) nav.cancel()
	})

	// If this step is torn down while the review drawer is open, resolve the promise
	// the executor is waiting on rather than leaving it pending forever.
	$effect(() => () => reviewResolve?.(false))

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
	<div>
		<h2 class="text-sm font-semibold text-emphasis">
			{project?.name ?? plan.slug} → <span class="font-mono">{destinationLabel}</span>
		</h2>
		{#if project}
			<p class="mt-0.5 text-xs text-secondary">{project.summary}</p>
		{/if}
	</div>

	{#if project}
		<!-- Triggers and migrations only become known once the export is fetched, so the
		     row grows mid-run rather than starting complete. -->
		<ProjectContentBadges counts={{ ...project.counts, ...(execution?.extraCounts ?? {}) }} />
	{/if}

	<!-- Only when the destination already exists. A workspace created by this run is
	     empty, so there is nothing for the project to sit next to and nothing to
	     choose between — asking would be a question with one answer. It lands in
	     f/<slug>/ either way; `installProject` creates the folder as it imports. -->
	{#if existingWorkspace}
		<div class="max-w-sm">
			<span class="mb-1 block text-xs font-normal text-secondary">Folder</span>
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
			{:else}
				<p class="mt-1 text-xs text-tertiary">
					Items import under <span class="font-mono">f/{folder.trim() || plan.slug}/</span>.
				</p>
			{/if}
		</div>
	{/if}

	{#if execution}
		<!-- The run, task by task, so a failure says which part failed. The paths the
		     import writes hang off the import task rather than forming a second list:
		     they are that task's output, not a parallel account of the same run. No
		     box around it — one run, one component. -->
		<ul class="flex flex-col gap-1.5">
			{#each execution.tasks as task (task.key)}
				<li class="flex flex-col gap-1">
					<div class="flex items-center gap-2 text-xs">
						{#if task.status === 'running'}
							<Loader2 size={13} class="animate-spin text-blue-500" />
						{:else if task.status === 'done'}
							<Check size={13} class="text-emerald-600" />
						{:else if task.status === 'failed'}
							<X size={13} class="text-red-600" />
						{:else}
							<span class="h-[13px] w-[13px] rounded-full border border-border-light"></span>
						{/if}
						<span class={task.status === 'pending' ? 'text-tertiary' : 'text-primary'}>
							{task.label}
						</span>
						{#if task.detail}
							<span class="truncate text-tertiary">— {task.detail}</span>
						{/if}
					</div>

					{#if task.key === 'import' && execution.results.length}
						<!-- Indented to the task's label, so the rule down the left reads as
						     "these came from the line above". -->
						<ul
							class="ml-[6px] flex max-h-52 flex-col gap-1 overflow-y-auto border-l border-border-light pl-4 text-xs"
						>
							{#each execution.results as r}
								<li class="flex items-center gap-2">
									<span class={r.ok ? 'text-emerald-600' : 'text-red-600'}>{r.ok ? '✓' : '✗'}</span>
									<span class="truncate font-mono">{r.path}</span>
									{#if !r.ok}<span class="shrink-0 text-red-600">— {r.error}</span>{/if}
								</li>
							{/each}
						</ul>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}

	{#if execution?.error}
		<Alert type="error" title="The import did not finish cleanly" size="xs">
			{execution.error}
		</Alert>
	{/if}

	<Alert type="warning" title="What import does to resources and triggers" size="xs" collapsible>
		Resources are imported as empty stubs — set their values after import; a resource whose path
		already exists is reported as failed (existing values are never overwritten). Trigger kinds are
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
				<Button variant="accent" unifiedSize="sm" onClick={onFinish}>
					{setupPending ? 'Continue →' : 'Finish setup →'}
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
					{:else if execution}
						Retry
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
</Portal>

<Drawer bind:this={reviewDrawer} size="700px" on:close={() => closeMigrationReview(false)}>
	<DrawerContent title="Data table migrations" on:close={() => closeMigrationReview(false)}>
		<div class="flex flex-col gap-4">
			<p class="text-xs text-secondary">
				This project ships migrations that recreate the data tables it uses. Review and edit the
				SQL, then choose which to run. A migration runs against the data table of the same name in
				the destination workspace; if that data table has migrations enabled it is recorded,
				otherwise it runs once as a preview job.
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
