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
	import { summaryToName } from '$lib/utils'
	import { Plus, Trash2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import EvalCaseEditor from './EvalCaseEditor.svelte'
	import EvalScorers from './EvalScorers.svelte'
	import { caseLabel, emptyCase, fromStoredCase, type CaseDraft } from './evalCaseUtils'
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
		/** The dataset's cases, held by the pane that lists them. */
		cases: EvalCase[]
		onCreated: (path: string) => void | Promise<void>
		onRenamed: (path: string) => void | Promise<void>
		onCasesChanged: () => void | Promise<void>
		/** The columns changed, which changes what every run of this dataset reports. */
		onScorersChanged: () => void | Promise<void>
	} = $props()

	let drawer: Drawer | undefined = $state()
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
	/** The columns chosen while naming a dataset that does not exist yet, sent with the create. */
	let pendingScorers = $state<Scorer[]>([])
	/** The drawer's own copy of the cases, which is what the editor edits. Nothing here is written
	 *  until Save: a case is a row of a set being curated, and a set half saved while someone is
	 *  still typing in it is not a state anyone asked for. Ids that are not in `storedIds` are the
	 *  drawer's own, given to cases it is holding for a dataset that has yet to be told about them. */
	let workingCases = $state<CaseDraft[]>([])
	let storedIds = $state<Set<string>>(new Set())

	let selectedCaseId = $state<string | undefined>(undefined)
	let caseDraft = $state<CaseDraft | undefined>(undefined)
	// Bumped whenever `caseDraft` is replaced wholesale: the editor seeds local state from the
	// draft, so it is keyed on this and remounts rather than carrying one case's edits into the
	// next.
	let draftGeneration = $state(0)
	let removingCase = $state<CaseDraft | undefined>(undefined)

	/** The next free `<agent>_datasetN`, which is what a dataset is called until it is named. */
	function nextDatasetIndex(): number {
		const taken = new Set(datasets.map((d) => d.path))
		let index = 1
		while (taken.has(`${agentPath}_dataset_${index}`)) index++
		return index
	}

	/**
	 * @param opts.caseId the case to open on, for an edit reached from a row of the table
	 * @param opts.addCase start on a new, empty case
	 * @param opts.capture a case captured from a run, opened for review before it is saved
	 */
	export function openDrawer(
		next: 'new' | 'edit',
		opts?: { caseId?: string; addCase?: boolean; capture?: CaseDraft }
	) {
		mode = next
		pathError = ''
		// Cleared per open: what was collected for a dataset that was never created belongs to that
		// attempt, not to the next one.
		pendingScorers = []
		workingCases = next === 'edit' ? cases.map((c) => fromStoredCase(c) as CaseDraft) : []
		storedIds = new Set(next === 'edit' ? cases.map((c) => c.id) : [])
		if (next === 'edit') {
			path = datasetPath ?? ''
			summary = dataset?.summary ?? ''
			// A dataset that has a path keeps it: the summary names one that does not have one yet.
			pathDirty = true
		} else {
			// Both seeded rather than left empty: an empty path makes the picker invent a random
			// name, a dataset named after the agent it tests sorts with the agent's own, and a
			// dataset with no summary is one the tables can only call by its path.
			// Left underived so the summary keeps driving the path: renaming it to what the set is
			// actually for renames the path with it, and the seeds are what that rule already
			// produces for "Dataset N".
			const index = nextDatasetIndex()
			path = `${agentPath}_dataset_${index}`
			summary = `Dataset ${index}`
			pathDirty = false
		}
		formGeneration += 1
		selectCase(opts?.caseId)
		if (opts?.capture) {
			selectedCaseId = undefined
			caseDraft = structuredClone($state.snapshot(opts.capture)) as CaseDraft
			draftGeneration += 1
		}
		drawer?.openDrawer()
		if (opts?.addCase) addCase()
	}

	function selectCase(id: string | undefined) {
		const found = id ? workingCases.find((c) => c.id === id) : undefined
		selectedCaseId = found?.id
		caseDraft = found
		draftGeneration += 1
	}

	/** Edits land in the list as they are typed, so switching cases keeps them and Save writes
	 *  them. In the list rather than on the server: nothing here has been asked for yet. */
	$effect(() => {
		const draft = caseDraft
		if (!draft?.id) return
		const id = draft.id
		untrack(() => {
			workingCases = workingCases.map((c) => (c.id === id ? draft : c))
		})
	})

	/** The summary names the dataset, as it does a script: what it is for is the thing you know
	 *  first, and a path derived from it beats one you have to invent. Until the path is typed in,
	 *  after which it is the reader's. */
	$effect(() => {
		const current = summary
		untrack(() => {
			if (pathDirty || !current) return
			// Named after the agent as well as after itself, so it sorts with the agent's own and reads
			// as belonging to it. The whole path stays editable.
			const agentName = agentPath.split('/').pop() ?? agentPath
			pathInput?.setName(`${agentName}_${summaryToName(current)}`)
		})
	})

	/** Creates the dataset the first case needs. Naming it is a decision worth making after you
	 *  know what is in it, so it is made for you here and renamed from this same drawer. */
	async function createDataset() {
		if (!workspace || !path || pathError) return
		creating = true
		try {
			const created = path
			await AiEvalsService.createEvalDataset({
				workspace,
				requestBody: {
					path: created,
					summary: summary || undefined,
					default_subject: { kind: 'agent', path: agentPath },
					// What was written for it while it was being named: the dataset arrives holding it
					// rather than being created empty and then edited to hold what was already chosen.
					scorers: pendingScorers,
					cases: workingCases.map(({ id: _id, ...rest }) => rest)
				}
			})
			// Stays open, on the dataset it just made: scorers and cases are what a dataset is, and
			// they can only be added to one that exists, so closing here would send you to find it
			// again to do the part you opened this for.
			await onCreated(created)
			mode = 'edit'
		} catch (e) {
			sendUserToast(`Failed to create the dataset: ${e}`, true)
		} finally {
			creating = false
		}
	}

	/** Writes the cases as the drawer now holds them: the ones it added, the ones it changed and
	 *  the ones it dropped, in that order so a rename below moves a set that is already right. */
	async function saveCases() {
		if (!workspace || !datasetPath) return
		const stored = new Map(cases.map((c) => [c.id, c]))
		const kept = new Set<string>()
		for (const working of workingCases) {
			const id = working.id
			if (!id || !storedIds.has(id)) {
				const { id: _local, ...rest } = working
				await AiEvalsService.addEvalCase({ workspace, path: datasetPath, requestBody: rest })
				continue
			}
			kept.add(id)
			const before = stored.get(id)
			const unchanged =
				before != undefined &&
				JSON.stringify(fromStoredCase(before)) === JSON.stringify($state.snapshot(working))
			if (unchanged) continue
			await AiEvalsService.updateEvalCase({
				workspace,
				path: datasetPath,
				requestBody: { ...$state.snapshot(working), id }
			})
		}
		for (const id of storedIds) {
			if (kept.has(id)) continue
			await AiEvalsService.deleteEvalCase({ workspace, path: datasetPath, requestBody: { id } })
		}
	}

	/** Renaming moves the dataset: its cases and experiments follow it through the foreign keys. */
	async function saveDataset() {
		if (!workspace || !datasetPath || !dataset || !path || pathError) return
		saving = true
		try {
			await saveCases()
			await AiEvalsService.updateEvalDataset({
				workspace,
				path: datasetPath,
				requestBody: {
					path,
					summary: summary || undefined,
					description: dataset.description,
					default_subject: dataset.default_subject,
					scorers: dataset.scorers
				}
			})
			await onCasesChanged()
			await onRenamed(path)
			// The list is what was just written, so the ids it holds are now the dataset's own.
			storedIds = new Set(workingCases.map((c) => c.id).filter((id): id is string => !!id))
			// Re-seeded on the path it now has: the picker reads a path that is not the one it
			// opened on as a path someone else has taken.
			formGeneration += 1
		} catch (e) {
			sendUserToast(`Failed to save the dataset: ${e}`, true)
		} finally {
			saving = false
		}
	}

	/** Adding a case puts a row in the list the drawer is holding. It reaches the dataset when the
	 *  drawer is saved, like every other edit made here. */
	function addCase() {
		const draft = { ...emptyCase(), id: randomUUID() }
		workingCases = [...workingCases, draft]
		selectedCaseId = draft.id
		caseDraft = draft
		draftGeneration += 1
	}

	/** Puts a case captured from a run into the list, which is what the button over the fields
	 *  offers: until then it is a draft the drawer is showing, not one of the set. */
	function keepCapturedCase() {
		if (!caseDraft || caseDraft.id) return
		const draft = { ...$state.snapshot(caseDraft), id: randomUUID() } as CaseDraft
		workingCases = [...workingCases, draft]
		selectedCaseId = draft.id
		caseDraft = draft
		draftGeneration += 1
	}

	function deleteCase(id: string) {
		workingCases = workingCases.filter((c) => c.id !== id)
		if (selectedCaseId === id) selectCase(undefined)
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
</script>

<Drawer bind:this={drawer} size="900px">
	<!-- Inside the drawer, not beside it. The drawer is portalled to the body and stacked above the
	     pane that opened it, so a modal rendered from here is otherwise trapped under it. -->
	<ConfirmationModal
		open={removingCase != undefined}
		title="Delete this case"
		confirmationText="Delete"
		on:canceled={() => (removingCase = undefined)}
		on:confirmed={async () => {
			const target = removingCase
			removingCase = undefined
			if (target?.id) await deleteCase(target.id)
		}}
	>
		<span class="text-sm">
			{caseLabel(removingCase ?? { input: {} })} goes from the dataset. The runs that executed it keep
			their results: a run that happened is not undone by curating the case away.
		</span>
	</ConfirmationModal>
	<DrawerContent
		title={mode === 'edit' ? 'Edit dataset' : 'New dataset'}
		on:close={() => drawer?.closeDrawer()}
	>
		<div class="flex flex-col gap-6 h-full min-h-0">
			<!-- On the page rather than under an icon: it says what the drawer is for, which is worth
			     reading once without being asked for. -->
			<span class="text-xs text-secondary">
				{mode === 'edit'
					? 'The cases this agent is measured on. Editing them leaves the runs that already executed them as they were.'
					: 'A set of cases to measure this agent on, and the scorers that read them.'}
			</span>
			<!-- Keyed so the path field is seeded for the dataset it was opened for, rather than
			     carrying the one before it. -->
			{#key formGeneration}
				<div class="flex flex-col gap-6">
					<Label label="Summary">
						<TextInput
							bind:value={summary}
							inputProps={{ placeholder: 'What this set of cases is for' }}
						/>
					</Label>
					<Path
						bind:this={pathInput}
						bind:path
						bind:error={pathError}
						bind:dirty={pathDirty}
						initialPath={mode === 'edit' ? (datasetPath ?? '') : ''}
						checkInitialPathExistence={false}
						namePlaceholder="cases"
						kind="resource"
						workspaceOverride={workspace}
						autofocus={false}
						size="sm"
					/>
					{#if mode === 'edit'}
						<span class="text-2xs text-tertiary">
							Renaming moves the dataset: its cases and its runs follow it.
						</span>
					{/if}
				</div>
			{/key}

			<!-- What the dataset measures with, before what it measures: a column applies to every
			     case, and a case is read against every column. Offered while naming a new one too:
			     a scorer is a runnable of its own, so it needs the dataset's name but not its row,
			     and the list is carried into the dataset that creating this makes. -->
			<EvalScorers
				{workspace}
				datasetPath={mode === 'edit' ? datasetPath : path}
				dataset={mode === 'edit' ? dataset : undefined}
				bind:pending={pendingScorers}
				onChanged={onScorersChanged}
			/>
			<!-- The list picks, the editor to its right fills in: a case is a handful of fields, and
			     a list that expanded one of them in place would move every case under the reader.
			     Written while naming a new dataset too: a case cannot be stored without one, so these
			     are held in the drawer and created with it. -->
			<div class="flex flex-col gap-2 grow min-h-0">
				<div class="flex items-center gap-2">
					<span class="text-xs font-semibold text-emphasis">Cases</span>
					<span class="text-2xs text-tertiary">{workingCases.length}</span>
					<div class="grow"></div>
					<Button
						size="xs2"
						variant="default"
						startIcon={{ icon: Plus }}
						disabled={mode === 'edit' && !datasetPath}
						onclick={addCase}
					>
						Add a case
					</Button>
				</div>
				<div class="flex gap-3 grow min-h-0">
					<div class="w-60 shrink-0 overflow-auto border rounded-md">
						{#if workingCases.length === 0}
							<div class="p-3 text-2xs text-tertiary">
								A case is a question this agent is asked, and what a good answer to it looks like.
							</div>
						{:else}
							<div class="divide-y">
								{#each workingCases as stored (stored.id)}
									<div
										class={`flex items-center gap-1 pr-1 ${stored.id === selectedCaseId ? 'bg-blue-50 dark:bg-blue-900/50' : 'hover:bg-surface-hover'}`}
									>
										<button
											type="button"
											class="grow min-w-0 text-left truncate px-2 py-1.5 text-xs text-emphasis"
											onclick={() => selectCase(stored.id)}
										>
											{caseLabel(stored)}
										</button>
										<Button
											size="xs2"
											variant="subtle"
											startIcon={{ icon: Trash2 }}
											iconOnly
											title="Delete this case"
											on:click={() => (removingCase = stored)}
										/>
									</div>
								{/each}
							</div>
						{/if}
					</div>
					<div class="grow min-w-0 overflow-auto">
						{#if caseDraft}
							{#if !caseDraft.id}
								<!-- A captured case is in the dataset only once it is put there. Above the fields
									     rather than under them: an expected answer runs to any length, and the way to
									     keep the case must not sit at the bottom of it. -->
								<div class="flex items-center gap-2 pb-3">
									<span class="text-2xs text-tertiary grow">
										Captured from a run, and not in the dataset yet.
									</span>
									<Button
										size="xs"
										variant="accent"
										startIcon={{ icon: Plus }}
										onclick={keepCapturedCase}
									>
										Add to dataset
									</Button>
								</div>
							{/if}
							{#key draftGeneration}
								<EvalCaseEditor bind:draft={caseDraft} />
							{/key}
						{:else}
							<span class="text-xs text-tertiary">
								{workingCases.length === 0
									? 'Add a case to fill it in here.'
									: 'Pick a case to edit it.'}
							</span>
						{/if}
					</div>
				</div>
			</div>
		</div>
		{#snippet actions()}
			{#if mode === 'edit'}
				<Button
					size="xs"
					variant="accent"
					loading={saving}
					disabled={saving || !path || !!pathError || nothingToSave}
					onclick={saveDataset}
				>
					Save
				</Button>
			{:else}
				<Button
					size="xs"
					variant="accent"
					startIcon={{ icon: Plus }}
					loading={creating}
					disabled={creating || !path || !!pathError}
					onclick={createDataset}
				>
					Create dataset
				</Button>
			{/if}
		{/snippet}
	</DrawerContent>
</Drawer>
