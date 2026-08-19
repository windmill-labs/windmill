<script lang="ts">
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import { AiEvalsService, type EvalCase, type EvalDataset } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { summaryToName } from '$lib/utils'
	import { Plus, Trash2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import EvalCaseEditor from './EvalCaseEditor.svelte'
	import EvalScorers from './EvalScorers.svelte'
	import { caseLabel, emptyCase, fromStoredCase, type CaseDraft } from './evalCaseUtils'

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

	let selectedCaseId = $state<string | undefined>(undefined)
	let caseDraft = $state<CaseDraft | undefined>(undefined)
	// Bumped whenever `caseDraft` is replaced wholesale: the editor seeds local state from the
	// draft, so it is keyed on this and remounts rather than carrying one case's edits into the
	// next.
	let draftGeneration = $state(0)
	let removingCase = $state<EvalCase | undefined>(undefined)

	/** The next free `<agent>_datasetN`, which is what a dataset is called until it is named. */
	function nextDatasetPath(): string {
		const taken = new Set(datasets.map((d) => d.path))
		let index = 1
		while (taken.has(`${agentPath}_dataset${index}`)) index++
		return `${agentPath}_dataset${index}`
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
		if (next === 'edit') {
			path = datasetPath ?? ''
			summary = dataset?.summary ?? ''
			// A dataset that has a path keeps it: the summary names one that does not have one yet.
			pathDirty = true
		} else {
			// Seeded rather than left empty: an empty path makes the picker invent a random name, and
			// a dataset named after the agent it tests sorts with the agent's own.
			path = nextDatasetPath()
			summary = ''
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
		const stored = id ? cases.find((c) => c.id === id) : undefined
		selectedCaseId = stored?.id
		caseDraft = stored ? (fromStoredCase(stored) as CaseDraft) : undefined
		draftGeneration += 1
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
					default_subject: { kind: 'agent', path: agentPath }
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

	/** Renaming moves the dataset: its cases and experiments follow it through the foreign keys. */
	async function saveDataset() {
		if (!workspace || !datasetPath || !dataset || !path || pathError) return
		saving = true
		try {
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
			await onRenamed(path)
			// Re-seeded on the path it now has: the picker reads a path that is not the one it
			// opened on as a path someone else has taken.
			formGeneration += 1
		} catch (e) {
			sendUserToast(`Failed to save the dataset: ${e}`, true)
		} finally {
			saving = false
		}
	}

	/** Adding a case writes the row: a case is a row of the dataset, so asking for one puts it
	 *  there and the editor beside the list fills it in. */
	async function addCase() {
		if (!workspace || !datasetPath) return
		try {
			const id = await AiEvalsService.addEvalCase({
				workspace,
				path: datasetPath,
				requestBody: emptyCase()
			})
			await onCasesChanged()
			selectedCaseId = id
			caseDraft = { ...emptyCase(), id }
			draftGeneration += 1
		} catch (e) {
			sendUserToast(`Failed to add a case: ${e}`, true)
		}
	}

	async function saveCase() {
		if (!workspace || !datasetPath || !caseDraft) return
		try {
			if (caseDraft.id) {
				await AiEvalsService.updateEvalCase({
					workspace,
					path: datasetPath,
					requestBody: { ...caseDraft, id: caseDraft.id }
				})
			} else {
				// The id the write returns is adopted, so saving twice edits the case rather than
				// adding a second one.
				const id = await AiEvalsService.addEvalCase({
					workspace,
					path: datasetPath,
					requestBody: caseDraft
				})
				caseDraft = { ...caseDraft, id }
				selectedCaseId = id
			}
			await onCasesChanged()
		} catch (e) {
			sendUserToast(`Failed to save the case: ${e}`, true)
		}
	}

	async function deleteCase(id: string) {
		if (!workspace || !datasetPath) return
		try {
			await AiEvalsService.deleteEvalCase({
				workspace,
				path: datasetPath,
				requestBody: { id }
			})
			if (selectedCaseId === id) selectCase(undefined)
			await onCasesChanged()
		} catch (e) {
			sendUserToast(`Failed to delete the case: ${e}`, true)
		}
	}

	let metadataUnchanged = $derived(
		path === (datasetPath ?? '') && (summary || '') === (dataset?.summary ?? '')
	)
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
			if (target) await deleteCase(target.id)
		}}
	>
		<span class="text-sm">
			{caseLabel(removingCase ?? { input: {} })} goes from the dataset. The runs that executed it keep
			their results: a run that happened is not undone by curating the case away.
		</span>
	</ConfirmationModal>
	<DrawerContent
		title={mode === 'edit' ? 'Edit dataset' : 'New dataset'}
		tooltip={mode === 'edit'
			? 'The cases this agent is measured on. Editing them leaves the runs that already executed them as they were.'
			: 'A set of cases to measure this agent on. Its cases are added once it exists.'}
		on:close={() => drawer?.closeDrawer()}
	>
		<div class="flex flex-col gap-6 h-full min-h-0">
			<!-- Keyed so the path field is seeded for the dataset it was opened for, rather than
			     carrying the one before it. -->
			{#key formGeneration}
				<div class="flex flex-col gap-6">
					<Label label="Summary">
						<TextInput
							bind:value={summary}
							size="sm"
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

			{#if mode === 'edit'}
				<!-- What the dataset measures with, before what it measures: a column applies to every
				     case, and a case is read against every column. -->
				<EvalScorers {workspace} {datasetPath} {dataset} onChanged={onScorersChanged} />
				<!-- The list picks, the editor to its right fills in: a case is a handful of fields, and
				     a list that expanded one of them in place would move every case under the reader. -->
				<div class="flex flex-col gap-2 grow min-h-0">
					<div class="flex items-center gap-2">
						<span class="text-xs font-semibold text-emphasis">Cases</span>
						<span class="text-2xs text-tertiary">{cases.length}</span>
						<div class="grow"></div>
						<Button
							size="xs2"
							variant="default"
							startIcon={{ icon: Plus }}
							disabled={!datasetPath}
							onclick={addCase}
						>
							Add a case
						</Button>
					</div>
					<div class="flex gap-3 grow min-h-0">
						<div class="w-60 shrink-0 overflow-auto border rounded-md">
							{#if cases.length === 0}
								<div class="p-3 text-2xs text-tertiary">
									A case is a question this agent is asked, and what a good answer to it looks like.
								</div>
							{:else}
								<div class="divide-y">
									{#each cases as stored (stored.id)}
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
											onclick={saveCase}
										>
											Add to dataset
										</Button>
									</div>
								{/if}
								{#key draftGeneration}
									<EvalCaseEditor
										bind:draft={caseDraft}
										canSave={!!datasetPath}
										onSave={saveCase}
									/>
								{/key}
							{:else}
								<span class="text-xs text-tertiary">
									{cases.length === 0
										? 'Add a case to fill it in here.'
										: 'Pick a case to edit it.'}
								</span>
							{/if}
						</div>
					</div>
				</div>
			{/if}
		</div>
		{#snippet actions()}
			{#if mode === 'edit'}
				<Button
					size="xs"
					variant="accent"
					loading={saving}
					disabled={saving || !path || !!pathError || metadataUnchanged}
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
