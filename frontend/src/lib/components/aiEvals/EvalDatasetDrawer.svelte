<script lang="ts">
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import { AiEvalsService, type EvalCase, type EvalDataset, type Scorer } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Plus, Trash } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import EvalCasesGrid from './EvalCasesGrid.svelte'
	import EvalScorers from './EvalScorers.svelte'
	import { caseLabel, emptyCase, fromStoredCase, summaryToName, type CaseDraft } from './evalUtils'
	import { randomUUID } from '$lib/utils/uuid'

	let {
		workspace,
		agentPath,
		datasetPath,
		dataset,
		datasets,
		cases,
		onCreated,
		onRenamed,
		onDeleted,
		onCasesChanged,
		onScorersChanged
	}: {
		workspace: string | undefined
		/** The agent the dataset is named after and belongs to. */
		agentPath: string
		/** The dataset being edited. Absent while the drawer is creating one. */
		datasetPath: string | undefined
		dataset: EvalDataset | undefined
		/** Every dataset in the workspace, for naming the next one. */
		datasets: EvalDataset[]
		/** The dataset's cases, held by the pane that lists them. All of them: a dataset is capped
		 *  at what one page returns, so the drawer always holds the whole set. */
		cases: EvalCase[]
		onCreated: (path: string) => void | Promise<void>
		onRenamed: (path: string) => void | Promise<void>
		onDeleted: (path: string) => void | Promise<void>
		onCasesChanged: () => void | Promise<void>
		onScorersChanged: () => void | Promise<void>
	} = $props()

	let drawer: Drawer | undefined = $state()

	let removingDataset = $state(false)
	let deleting = $state(false)
	async function deleteDataset() {
		if (!workspace || !datasetPath) return
		deleting = true
		const deleted = datasetPath
		try {
			await AiEvalsService.deleteEvalDataset({ workspace, path: deleted })
			await onDeleted(deleted)
			drawer?.closeDrawer()
		} catch (e) {
			sendUserToast(`Failed to delete the dataset: ${e}`, true)
		} finally {
			deleting = false
		}
	}
	let mode = $state<'new' | 'edit'>('new')
	let path = $state('')
	let summary = $state('')
	let pathError = $state('')
	// Set once the path is typed in, after which the summary stops driving it.
	let pathDirty = $state(false)
	let pathInput: Path | undefined = $state(undefined)
	// Bumped on every open so the path picker is seeded for the dataset it was opened for, rather
	// than carrying the one before it.
	let formGeneration = $state(0)
	let creating = $state(false)
	let saving = $state(false)
	/** The drawer is writing the dataset, by either route: nothing may change until it lands. */
	/** A scorer write in flight: Save would otherwise resend the columns as they were before it. */
	let scorersWriting = $state(false)
	let writing = $derived(creating || saving || deleting || scorersWriting)
	/** The columns chosen while naming a dataset that does not exist yet, sent with the create. */
	let pendingScorers = $state<Scorer[]>([])
	/** The drawer's own copy of the cases, which is what the editor edits and what Save writes.
	 *  Ids that are not in `storedIds` are the drawer's own, given to cases it is holding for a
	 *  dataset that has yet to be told about them. */
	let workingCases = $state<CaseDraft[]>([])
	let storedIds = $state<Set<string>>(new Set())

	let removingCase = $state<CaseDraft | undefined>(undefined)

	/** The next free `<agent>_datasetN`, which is what a dataset is called until it is named. */
	function nextDatasetIndex(): number {
		const taken = new Set(datasets.map((d) => d.path))
		let index = 1
		while (taken.has(`${agentPath}_dataset_${index}`)) index++
		return index
	}

	export function openDrawer(next: 'new' | 'edit') {
		mode = next
		pathError = ''
		// Or the case added last time opens itself when the drawer is next opened on this dataset:
		// ids survive `fromStoredCase`, so the id would still match a row.
		focusCaseId = undefined
		// What was collected for a dataset that was never created belongs to that attempt.
		pendingScorers = []
		workingCases = next === 'edit' ? cases.map((c) => fromStoredCase(c) as CaseDraft) : []
		storedIds = new Set(next === 'edit' ? cases.map((c) => c.id) : [])
		if (next === 'edit') {
			path = datasetPath ?? ''
			summary = dataset?.summary ?? ''
			// A dataset that has a path keeps it: the summary names one that does not have one yet.
			pathDirty = true
		} else {
			// Seeded rather than left empty: an empty path makes the picker invent a random name.
			// Left underived so the summary keeps driving the path.
			const index = nextDatasetIndex()
			path = `${agentPath}_dataset_${index}`
			summary = `Dataset ${index}`
			pathDirty = false
		}
		formGeneration += 1
		drawer?.openDrawer()
	}

	/** The summary names the dataset, as it does a script, until the path is typed in. */
	$effect(() => {
		const current = summary
		untrack(() => {
			if (pathDirty || !current) return
			const agentName = agentPath.split('/').pop() ?? agentPath
			pathInput?.setName(`${agentName}_${summaryToName(current)}`)
		})
	})

	async function createDataset() {
		if (!workspace || !path || pathError) return
		creating = true
		const created = path
		const submittedSummary = summary || undefined
		try {
			await AiEvalsService.createEvalDataset({
				workspace,
				requestBody: {
					path: created,
					summary: submittedSummary,
					scorers: pendingScorers,
					cases: workingCases.map(({ id: _id, ...rest }) => rest)
				}
			})
		} catch (e) {
			sendUserToast(`Failed to create the dataset: ${e}`, true)
			creating = false
			return
		}
		// The pane moves onto the created dataset while the drawer is still open — the Run dialog
		// stands underneath and follows that selection. A refresh that fails must still not read as
		// a create that failed, or the retry hits "already exists".
		try {
			await onCreated(created)
		} catch (e) {
			sendUserToast(
				`Created the dataset, but the view could not refresh: ${e}. Reload to see it.`,
				true
			)
		}
		drawer?.closeDrawer()
		creating = false
	}

	/** The cases as the drawer now holds them. A local id is the drawer's own, for a row the
	 *  dataset has never been told about, and goes out as no id. */
	function casesToSave() {
		return $state.snapshot(workingCases).map((c) => ({
			id: c.id != undefined && storedIds.has(c.id) ? c.id : undefined,
			input: c.input,
			expected: c.expected
		}))
	}

	/** One request carries the rename, the summary and the cases, so a rename the server refuses
	 *  refuses the case edits with it instead of leaving them written under the old name. */
	async function saveDataset() {
		if (!workspace || !datasetPath || !dataset || !path || pathError) return
		// Read once, so every step of the save agrees on what was submitted: the fields are locked
		// while it runs, and the one the drawer navigates to afterwards must be the one written.
		// `summary` always travels: the server keeps the stored one when it is absent, so clearing
		// it means sending the empty string.
		const submitted = { path, summary }
		saving = true
		try {
			await AiEvalsService.updateEvalDataset({
				workspace,
				path: datasetPath,
				requestBody: {
					path: submitted.path,
					summary: submitted.summary,
					scorers: dataset.scorers,
					cases: casesToSave()
				}
			})
		} catch (e) {
			sendUserToast(`Failed to save the dataset: ${e}`, true)
			saving = false
			return
		}
		// The pane moves onto the saved name before the drawer closes: a re-read under the old name
		// is a 404 on a save that succeeded. A refresh that fails must still not read as a save that
		// failed, or the retry renames from the obsolete path.
		try {
			await onRenamed(submitted.path)
			await onCasesChanged()
		} catch (e) {
			sendUserToast(
				`Saved the dataset, but the view could not refresh: ${e}. Reload to see it.`,
				true
			)
		}
		drawer?.closeDrawer()
		saving = false
	}

	/** The case the grid should open for typing: the one just added. */
	let focusCaseId = $state<string | undefined>(undefined)

	function addCase() {
		const id = randomUUID()
		workingCases = [...workingCases, { ...emptyCase(), id }]
		focusCaseId = id
	}

	function deleteCase(id: string) {
		workingCases = workingCases.filter((c) => c.id !== id)
	}

	/** A row the dataset has never been told about goes without being asked about: what the
	 *  confirmation warns of is the runs that executed the case, and nothing has executed this one. */
	function removeCase(c: CaseDraft) {
		if (c.id != undefined && !storedIds.has(c.id)) {
			deleteCase(c.id)
			return
		}
		removingCase = c
	}

	let metadataUnchanged = $derived(
		path === (datasetPath ?? '') && (summary || '') === (dataset?.summary ?? '')
	)
	/** Whether the list holds anything the dataset does not: added, edited or dropped. */
	let casesChanged = $derived.by(() => {
		if (workingCases.length !== cases.length) return true
		const stored = new Map(cases.map((c) => [c.id, JSON.stringify(fromStoredCase(c))]))
		return workingCases.some(
			(c) => !c.id || stored.get(c.id) !== JSON.stringify($state.snapshot(c))
		)
	})
	let nothingToSave = $derived(metadataUnchanged && !casesChanged)
	/** A dataset with no cases cannot be run — the server refuses it (`run.rs`, "has no case to
	 *  run") — so there is no point creating one. Only creation is blocked: a saved dataset must
	 *  stay editable down to its last case, or removing that case, or renaming while it is the only
	 *  one left, would have no way to be saved. */
	let noCases = $derived(workingCases.length === 0)
	let noCasesTitle = $derived(noCases ? 'Add at least one case first' : undefined)
</script>

<Drawer bind:this={drawer} size="900px">
	<!-- Inside the drawer, not beside it. The drawer is portalled to the body and stacked above the
	     pane that opened it, so a modal rendered from here is otherwise trapped under it. -->
	<ConfirmationModal
		open={removingCase != undefined}
		title="Delete this case"
		confirmationText="Delete"
		on:canceled={() => (removingCase = undefined)}
		on:confirmed={() => {
			const target = removingCase
			removingCase = undefined
			if (target?.id) deleteCase(target.id)
		}}
	>
		<span class="text-sm">
			{caseLabel(removingCase ?? { input: {} })} goes from the dataset. The runs that executed it keep
			their results: a run that happened is not undone by curating the case away.
		</span>
	</ConfirmationModal>
	<ConfirmationModal
		open={removingDataset}
		title="Delete this dataset"
		confirmationText="Delete"
		on:canceled={() => (removingDataset = false)}
		on:confirmed={() => {
			removingDataset = false
			deleteDataset()
		}}
	>
		<span class="text-sm">
			{datasetPath} goes with its cases and every run recorded against it. The jobs those runs produced
			are kept.
		</span>
	</ConfirmationModal>
	<DrawerContent
		title={mode === 'edit' ? 'Edit dataset' : 'New dataset'}
		on:close={() => drawer?.closeDrawer()}
	>
		<div class="flex flex-col gap-6 h-full min-h-0">
			<span class="text-xs text-secondary">
				{mode === 'edit'
					? 'The cases this agent is measured on. Editing them leaves the runs that already executed them as they were.'
					: 'A set of cases to measure this agent on, and the scorers that read them.'}
			</span>
			{#key formGeneration}
				<!-- `inert` rather than each field's `disabled`, which `Path` owns as its own transient
				     state. -->
				<div class="flex flex-col gap-6" inert={writing}>
					<Label label="Summary">
						<TextInput
							bind:value={summary}
							inputProps={{ placeholder: 'What this set of cases is for' }}
						/>
					</Label>
					<Label
						label="Path"
						tooltip="Where the dataset lives. Renaming it moves it: its cases and its runs follow."
					>
						<Path
							bind:this={pathInput}
							bind:path
							bind:error={pathError}
							bind:dirty={pathDirty}
							initialPath={mode === 'edit' ? (datasetPath ?? '') : ''}
							checkInitialPathExistence={false}
							warnOnRename={false}
							namePlaceholder="cases"
							kind="resource"
							workspaceOverride={workspace}
							autofocus={false}
							size="sm"
						/>
					</Label>
				</div>
			{/key}

			<!-- Offered while naming a new dataset too: a scorer is a runnable of its own, so it needs
			     the dataset's name but not its row, and the list is carried into the create. -->
			<div inert={writing}>
				<EvalScorers
					{workspace}
					datasetPath={mode === 'edit' ? datasetPath : path}
					dataset={mode === 'edit' ? dataset : undefined}
					{datasets}
					bind:pending={pendingScorers}
					onChanged={onScorersChanged}
					onWriting={(w) => (scorersWriting = w)}
				/>
			</div>
			<!-- Sized by its rows, not by what is left of the drawer: a dataset with one case should
			     not be followed by an empty half-screen of table. The drawer body scrolls when the
			     list outgrows it. -->
			<div class="flex flex-col gap-2">
				<div class="flex items-center gap-2">
					<span class="text-xs font-semibold text-emphasis">Cases</span>
					<span class="text-2xs text-tertiary">{workingCases.length}</span>
					<div class="grow"></div>
					<Button
						unifiedSize="sm"
						variant="default"
						startIcon={{ icon: Plus }}
						disabled={(mode === 'edit' && !datasetPath) || writing}
						onclick={addCase}
					>
						Add a case
					</Button>
				</div>
				<div>
					<EvalCasesGrid
						bind:cases={workingCases}
						onRemove={removeCase}
						onAdd={addCase}
						{focusCaseId}
						locked={writing}
					/>
				</div>
			</div>
		</div>
		{#snippet actions()}
			{#if mode === 'edit'}
				<Button
					unifiedSize="md"
					variant="default"
					destructive
					startIcon={{ icon: Trash }}
					loading={deleting}
					disabled={writing || !datasetPath}
					onclick={() => (removingDataset = true)}
				>
					Delete
				</Button>
				<Button
					unifiedSize="md"
					variant="accent"
					loading={saving}
					disabled={writing || !path || !!pathError || nothingToSave}
					onclick={saveDataset}
				>
					Save
				</Button>
			{:else}
				<Button
					unifiedSize="md"
					variant="accent"
					startIcon={{ icon: Plus }}
					loading={creating}
					disabled={creating || !path || !!pathError || noCases}
					title={noCasesTitle}
					onclick={createDataset}
				>
					Create dataset
				</Button>
			{/if}
		{/snippet}
	</DrawerContent>
</Drawer>
