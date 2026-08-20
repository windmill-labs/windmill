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
	import { Plus } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import EvalCasesGrid from './EvalCasesGrid.svelte'
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
		onScorersChanged,
		onClosed
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
		onCasesChanged: () => void | Promise<void>
		/** The columns changed, which changes what every run of this dataset reports. */
		onScorersChanged: () => void | Promise<void>
		/** The drawer is done, whether it saved anything or not: what opened it decides whether
		 *  there is somewhere to go back to. */
		onClosed?: () => void
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
		drawer?.openDrawer()
	}

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
					// What was written for it while it was being named: the dataset arrives holding it
					// rather than being created empty and then edited to hold what was already chosen.
					scorers: pendingScorers,
					cases: workingCases.map(({ id: _id, ...rest }) => rest)
				}
			})
			// Everything the drawer holds is written with it, so creating is the whole of what this
			// was opened for: it closes on the dataset it just made, which the pane is now on.
			await onCreated(created)
			drawer?.closeDrawer()
		} catch (e) {
			sendUserToast(`Failed to create the dataset: ${e}`, true)
		} finally {
			creating = false
		}
	}

	/** Writes the cases as the drawer now holds them, in one request: what it added, what it
	 *  changed and what it dropped land together or not at all. The ids come back so that a save
	 *  which failed further on — the rename below — is retried as the same cases rather than
	 *  adding the new ones a second time. */
	async function saveCases() {
		if (!workspace || !datasetPath) return
		const sent = $state.snapshot(workingCases)
		const ids = await AiEvalsService.saveEvalCases({
			workspace,
			path: datasetPath,
			requestBody: {
				cases: sent.map((c) => ({
					// A local id is the drawer's own, for a row the dataset has never been told about.
					id: c.id != undefined && storedIds.has(c.id) ? c.id : undefined,
					name: c.name,
					input: c.input,
					expected: c.expected
				}))
			}
		})
		workingCases = workingCases.map((c, index) => ({ ...c, id: ids[index] ?? c.id }))
		storedIds = new Set(ids)
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
					scorers: dataset.scorers
				}
			})
			await onCasesChanged()
			await onRenamed(path)
			// Everything the drawer holds is written, so saving is the whole of what it was opened
			// for and it closes on that, as creating does. The next open seeds it from the dataset
			// as it now stands rather than from what this one was left holding.
			drawer?.closeDrawer()
		} catch (e) {
			sendUserToast(`Failed to save the dataset: ${e}`, true)
		} finally {
			saving = false
		}
	}

	/** Adding a case puts a row in the list the drawer is holding. It reaches the dataset when the
	 *  drawer is saved, like every other edit made here. */
	function addCase() {
		workingCases = [...workingCases, { ...emptyCase(), id: randomUUID() }]
	}

	function deleteCase(id: string) {
		workingCases = workingCases.filter((c) => c.id !== id)
	}

	/** A row the dataset has never been told about goes without being asked about: what the
	 *  confirmation warns of is the runs that executed the case, and nothing has executed one that
	 *  was added a moment ago. */
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
</script>

<Drawer bind:this={drawer} size="900px" on:close={() => onClosed?.()}>
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
							namePlaceholder="cases"
							kind="resource"
							workspaceOverride={workspace}
							autofocus={false}
							size="sm"
						/>
					</Label>
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
				{datasets}
				bind:pending={pendingScorers}
				onChanged={onScorersChanged}
			/>
			<!-- Written while naming a new dataset too: a case cannot be stored without one, so these
			     are held in the drawer and created with it. -->
			<div class="flex flex-col gap-2 grow min-h-0">
				<div class="flex items-center gap-2">
					<span class="text-xs font-semibold text-emphasis">Cases</span>
					<span class="text-2xs text-tertiary">{workingCases.length}</span>
					<div class="grow"></div>
					<Button
						size="xs"
						variant="default"
						startIcon={{ icon: Plus }}
						disabled={mode === 'edit' && !datasetPath}
						onclick={addCase}
					>
						Add a case
					</Button>
				</div>
				<!-- The whole set on screen, edited where it is read: curating a dataset is comparing
				     cases against each other, which a pane showing one at a time cannot be asked to do.
				     The same grid the data tables are edited in, so a set of rows is edited the one way
				     this app edits rows. -->
				<div class="grow min-h-0">
					<EvalCasesGrid bind:cases={workingCases} onRemove={removeCase} />
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
