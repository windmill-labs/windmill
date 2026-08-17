<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Alert, Button, Drawer, DrawerContent } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { WorkspaceService } from '$lib/gen'
	import ProjectContentBadges from '$lib/components/ProjectContentBadges.svelte'
	import type { ProjectMigration } from '$lib/components/workspaceSettings/projectBundle'
	import MigrationSqlEditor from '$lib/components/workspaceSettings/MigrationSqlEditor.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '$lib/components/common/confirmationModal/asyncConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { ImportExecution } from '$lib/importWizard/execution.svelte'
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
		onBack: () => void
	}

	let { plan, project, onFolderChange, onFinish, onBack }: Props = $props()

	let folder = $state(plan.folder ?? plan.slug)
	const folderValid = $derived(FOLDER_NAME_RE.test(folder.trim()))
	const problem = $derived(planProblem({ ...plan, folder: folder.trim() }))

	const destinationLabel = $derived(
		plan.destination?.kind === 'new'
			? `${plan.destination.id} (new)`
			: (plan.destination?.workspaceId ?? 'nowhere yet')
	)

	// --- the run ---------------------------------------------------------------
	const missingDatatableModal = createAsyncConfirmationModal()
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
		const runnable = enabled.filter((m) => present.has(m.datatable_name))
		const missing = [
			...new Set(enabled.filter((m) => !present.has(m.datatable_name)).map((m) => m.datatable_name))
		]

		if (missing.length > 0) {
			const proceed = await missingDatatableModal.ask({
				title: 'Some data tables are missing',
				confirmationText: 'Import without them',
				children: `This project uses data table(s) "${missing.join(
					'", "'
				)}" that don't exist in this workspace, so their migrations will be skipped. To apply them, cancel, create the data table(s) with the same name in Workspace settings → Data tables, then re-run this import.`
			})
			if (!proceed) return null
		}

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

	// One execution per plan. Going back and choosing a different destination makes a
	// new one, so a previous run's outcome can never be shown against another plan.
	let execution = $state<ImportExecution | undefined>(undefined)
	const planKey = $derived(JSON.stringify(plan.destination) + plan.slug)
	let executionKey: string | undefined = undefined
	$effect(() => {
		if (executionKey === planKey) return
		executionKey = planKey
		execution = undefined
	})

	function start() {
		const current =
			execution ??
			new ImportExecution(
				{ ...plan, folder: folder.trim() },
				{ reviewMigrations, hasEeLicense: !!$enterpriseLicense }
			)
		execution = current
		void current.run()
	}

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
		<ProjectContentBadges counts={project.counts} />
	{/if}

	<div class="max-w-sm">
		<p class="mb-1 text-[11px] font-medium uppercase tracking-wide text-tertiary">Folder</p>
		<!-- A plain field, not FolderPicker: that component lists and creates folders in
		     `$workspaceStore`, and until this step runs there may be no such workspace. -->
		<TextInput
			size="sm"
			bind:value={folder}
			inputProps={{
				disabled: execution?.running || execution?.done,
				onblur: () => onFolderChange(folder.trim())
			}}
		/>
		{#if folder.trim() && !folderValid}
			<p class="mt-1 text-[11px] text-red-600">Letters, digits, dashes and underscores only.</p>
		{:else}
			<p class="mt-1 text-xs text-tertiary">
				Items import under <span class="font-mono">f/{folder.trim() || plan.slug}/</span>.
			</p>
		{/if}
	</div>

	{#if execution}
		<!-- The run, task by task, so a failure says which part failed. -->
		<ul class="flex flex-col gap-1.5 rounded-md border border-border-light p-3">
			{#each execution.tasks as task (task.key)}
				<li class="flex items-center gap-2 text-xs">
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
				</li>
			{/each}
		</ul>
	{/if}

	{#if execution?.results.length}
		<ul class="flex max-h-52 flex-col gap-1 overflow-y-auto text-xs">
			{#each execution.results as r}
				<li class="flex items-center gap-2">
					<span class={r.ok ? 'text-emerald-600' : 'text-red-600'}>{r.ok ? '✓' : '✗'}</span>
					<span class="truncate font-mono">{r.path}</span>
					{#if !r.ok}<span class="shrink-0 text-red-600">— {r.error}</span>{/if}
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
				<Button variant="accent" unifiedSize="sm" onClick={onFinish}>Finish setup →</Button>
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
	<ConfirmationModal {...missingDatatableModal.props} />
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
			<Button variant="border" onclick={() => closeMigrationReview(false)}>Skip migrations</Button>
			<Button
				variant="accent"
				disabled={!reviewList.some((m) => m.run && m.sql.trim() !== '')}
				onclick={() => closeMigrationReview(true)}
			>
				Run selected
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
