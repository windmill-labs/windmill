<script lang="ts">
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import ScriptEditorDrawer from '$lib/components/flows/content/ScriptEditorDrawer.svelte'
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import { AiEvalsService, ScriptService, type EvalDataset, type Scorer } from '$lib/gen'
	import { randomUUID } from '$lib/utils/uuid'
	import { sendUserToast } from '$lib/toast'
	import {
		Bot,
		ChevronDown,
		Code2,
		ExternalLink,
		Pencil,
		Plus,
		Settings,
		Trash2
	} from 'lucide-svelte'
	import { base } from '$lib/base'
	import AddScorer from './AddScorer.svelte'
	import { kindLabel, parseThreshold, scorerLabel, type ScorerKind } from './evalUtils'

	let {
		workspace,
		datasetPath,
		dataset,
		datasets,
		pending = $bindable(),
		onChanged,
		onWriting
	}: {
		workspace: string | undefined
		/** What to name new runnables after, which a dataset has before it exists. */
		datasetPath: string | undefined
		/** The workspace's datasets, for naming the one a reusable scorer already measures. */
		datasets: EvalDataset[]
		/** The saved dataset, absent until there is one: its absence says to collect, not to save. */
		dataset: EvalDataset | undefined
		/** The columns a dataset that does not exist yet is collecting. There is no row to write
		 * them to, so they are held here and sent with the dataset that is about to be created. */
		pending?: Scorer[]
		onChanged: () => void | Promise<void>
		/** A write of the columns is in flight: the drawer holding this must not save over it. */
		onWriting?: (writing: boolean) => void
	} = $props()

	let scorers = $derived(dataset ? (dataset.scorers ?? []) : (pending ?? []))
	// The server's cap, refused when the dataset is written. Adding creates the judge or script
	// first, so the 21st would leave a runnable behind; the control closes before that.
	const MAX_SCORERS_PER_DATASET = 20

	let scorerDrawer: Drawer | undefined = $state()
	let addScorerForm: AddScorer | undefined = $state()
	let scriptEditorDrawer: ScriptEditorDrawer | undefined = $state()
	let resourceEditorDrawer: ResourceEditorDrawer | undefined = $state()
	let scorerKind = $state<ScorerKind>('agent')
	let scorerMode = $state<'new' | 'existing'>('new')
	let scorerFormGeneration = $state(0)
	let removingScorer = $state<Scorer | undefined>(undefined)

	let settingsDrawer: Drawer | undefined = $state()
	let settingsScorer = $state<Scorer | undefined>(undefined)
	let settingsName = $state('')
	let settingsThreshold = $state('')
	let savingSettings = $state(false)

	async function saveScorers(next: Scorer[]) {
		// Nothing to save them to yet: the dataset being named carries them in, and until then this
		// list is the whole of what it knows about its columns.
		if (!dataset) {
			pending = next
			return
		}
		if (!workspace || !datasetPath) return
		onWriting?.(true)
		try {
			await AiEvalsService.updateEvalDataset({
				workspace,
				path: datasetPath,
				requestBody: { scorers: next }
			})
			await onChanged()
		} finally {
			onWriting?.(false)
		}
	}

	function openAdd(kind: ScorerKind, mode: 'new' | 'existing') {
		scorerKind = kind
		scorerMode = mode
		// Keyed so the form is seeded for what it was opened for, rather than carrying the path and
		// prompt of the one added before it.
		scorerFormGeneration += 1
		scorerDrawer?.openDrawer()
	}

	async function addScorer(scorer: Scorer) {
		try {
			// A pending column is identified by this id until the dataset is created and the server
			// mints its own: without it every pending scorer shares `undefined`, and an edit to one
			// hits all of them.
			await saveScorers([...scorers, { ...scorer, id: scorer.id ?? randomUUID() }])
			scorerDrawer?.closeDrawer()
		} catch (e) {
			sendUserToast(`Failed to add the scorer: ${e}`, true)
		}
	}

	/** Editing a column is editing the runnable it points at. */
	async function editScorer(scorer: Scorer) {
		if (!workspace) return
		if (scorer.kind === 'agent') {
			await resourceEditorDrawer?.initEdit(scorer.path)
			return
		}
		try {
			// The editor opens a script by hash, so the latest one is resolved here.
			const script = await ScriptService.getScriptByPath({ workspace, path: scorer.path })
			await scriptEditorDrawer?.openDrawer(script.hash, onChanged)
		} catch (e) {
			sendUserToast(`Failed to open ${scorer.path}: ${e}`, true)
		}
	}

	function openSettings(scorer: Scorer) {
		settingsScorer = scorer
		settingsName = scorer.name ?? ''
		settingsThreshold = scorer.pass_if == undefined ? '' : String(scorer.pass_if)
		settingsDrawer?.openDrawer()
	}

	let threshold = $derived(parseThreshold(settingsThreshold))

	/** The name is the column header only: it is a copy taken when the scorer was added, so the
	 *  runnable keeps whatever it is called on its own. */
	async function saveSettings() {
		const target = settingsScorer
		if (!target || threshold.error) return
		savingSettings = true
		try {
			await saveScorers(
				scorers.map((s) =>
					s.id === target.id
						? { ...s, name: settingsName.trim() || undefined, pass_if: threshold.value }
						: s
				)
			)
			settingsDrawer?.closeDrawer()
		} catch (e) {
			sendUserToast(`Failed to save the scorer: ${e}`, true)
		} finally {
			savingSettings = false
		}
	}
</script>

<div class="flex flex-col gap-2">
	<div class="flex items-center gap-2">
		<span class="text-xs font-semibold text-emphasis">Scorers</span>
		<span class="text-2xs text-tertiary">{scorers.length}</span>
		<div class="grow"></div>
		<DropdownV2
			items={[
				{ displayName: 'New AI judge', icon: Bot, action: () => openAdd('agent', 'new') },
				{
					displayName: 'Existing AI judge',
					icon: Bot,
					action: () => openAdd('agent', 'existing')
				},
				{ displayName: 'New code scorer', icon: Code2, action: () => openAdd('script', 'new') },
				{
					displayName: 'Existing code scorer',
					icon: Code2,
					action: () => openAdd('script', 'existing')
				}
			]}
			placement="bottom-end"
		>
			{#snippet buttonReplacement()}
				<Button
					nonCaptureEvent
					unifiedSize="md"
					variant="default"
					startIcon={{ icon: Plus }}
					endIcon={{ icon: ChevronDown }}
					disabled={!workspace || scorers.length >= MAX_SCORERS_PER_DATASET}
				>
					Add scorer
				</Button>
			{/snippet}
		</DropdownV2>
	</div>

	<div>
		{#if scorers.length === 0}
			<div class="border rounded-md p-3 text-2xs text-tertiary">
				A scorer reads one run and returns a number. Every run of this dataset is measured by all of
				them, which is what makes two runs comparable.
			</div>
		{:else}
			<!-- Laid out as `RadioCard` is — same padding, icon size, and label-over-description type
			     scale — so a scorer reads as a card of the same family. Not that component: nothing
			     here is being selected. -->
			<div class="flex flex-col gap-2">
				{#each scorers as scorer (scorer.id)}
					<div
						class="flex items-center gap-2 rounded-md border border-border-light bg-surface-tertiary p-3"
					>
						{#if scorer.kind === 'agent'}
							<Bot size={16} class="text-secondary shrink-0" />
						{:else}
							<Code2 size={16} class="text-secondary shrink-0" />
						{/if}
						<div class="flex flex-col min-w-0 grow">
							<span class="text-xs font-semibold text-emphasis truncate">
								{scorerLabel(scorer)}
							</span>
							<span class="text-xs font-normal text-secondary truncate mt-0.5">{scorer.path}</span>
						</div>
						{#if scorer.pass_if != undefined}
							<span class="text-2xs text-tertiary shrink-0" title="Pass threshold">
								≥ {scorer.pass_if}
							</span>
						{/if}
						<Button
							unifiedSize="sm"
							variant="subtle"
							startIcon={{ icon: Pencil }}
							iconOnly
							title={`Edit the ${scorer.kind === 'agent' ? 'agent' : 'script'} behind this column`}
							on:click={() => editScorer(scorer)}
						/>
						<Button
							unifiedSize="sm"
							variant="subtle"
							startIcon={{ icon: Settings }}
							iconOnly
							title="Name and pass threshold"
							on:click={() => openSettings(scorer)}
						/>
						<Button
							unifiedSize="sm"
							variant="subtle"
							startIcon={{ icon: Trash2 }}
							iconOnly
							title="Remove this scorer"
							on:click={() => (removingScorer = scorer)}
						/>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<Drawer bind:this={scorerDrawer} size="700px">
	<DrawerContent
		title={`${scorerMode === 'new' ? 'New' : 'Existing'} ${kindLabel(scorerKind).toLowerCase()}`}
		on:close={() => scorerDrawer?.closeDrawer()}
	>
		{#if workspace && datasetPath}
			{#key scorerFormGeneration}
				<AddScorer
					bind:this={addScorerForm}
					{workspace}
					{datasetPath}
					{datasets}
					kind={scorerKind}
					mode={scorerMode}
					onAdd={addScorer}
					onEditScript={(hash) =>
						scriptEditorDrawer
							?.openDrawer(hash, onChanged)
							.catch((e) => sendUserToast(`Failed to open the scorer: ${e}`, true))}
				/>
			{/key}
		{/if}
		{#snippet actions()}
			{@const state = addScorerForm?.submitState()}
			<Button
				unifiedSize="md"
				variant="accent"
				loading={state?.busy}
				disabled={!state || state.disabled}
				title={state?.title}
				onclick={() => addScorerForm?.submit()}
			>
				{state?.label ?? 'Add scorer'}
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<Drawer bind:this={settingsDrawer} size="600px">
	<DrawerContent title="Scorer settings" on:close={() => settingsDrawer?.closeDrawer()}>
		{#if settingsScorer}
			<div class="flex flex-col gap-6">
				<Label label={kindLabel(settingsScorer.kind)}>
					<a
						class="flex items-center gap-1.5 text-xs min-w-0 hover:underline w-fit"
						href={settingsScorer.kind === 'agent'
							? `${base}/resources?path=${settingsScorer.path}&workspace=${workspace}`
							: `${base}/scripts/get/${settingsScorer.path}?workspace=${workspace}`}
						target="_blank"
						rel="noreferrer"
						title={`Open ${settingsScorer.path}`}
					>
						{#if settingsScorer.kind === 'agent'}
							<Bot size={14} class="text-tertiary shrink-0" />
						{:else}
							<Code2 size={14} class="text-tertiary shrink-0" />
						{/if}
						<span class="truncate">{settingsScorer.path}</span>
						<ExternalLink size={12} class="text-tertiary shrink-0" />
					</a>
				</Label>
				<Label label="Name">
					<span class="text-xs text-secondary">
						What the column is called here. This dataset's own name for the scorer, so it does not
						rename the {settingsScorer.kind === 'agent' ? 'agent' : 'script'} it points at.
					</span>
					<TextInput
						bind:value={settingsName}
						inputProps={{ maxlength: 120, placeholder: settingsScorer.path.split('/').pop() }}
					/>
				</Label>
				<Label label="Pass threshold">
					<span class="text-xs text-secondary">
						A score at or above this counts as a pass, and the column reports a pass rate. Leave it
						empty to report the average score.
					</span>
					<TextInput
						bind:value={settingsThreshold}
						error={threshold.error}
						inputProps={{ placeholder: 'No threshold' }}
					/>
				</Label>
			</div>
		{/if}
		{#snippet actions()}
			<Button
				unifiedSize="md"
				variant="accent"
				loading={savingSettings}
				disabled={savingSettings || threshold.error}
				onclick={saveSettings}
			>
				Save
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<ScriptEditorDrawer bind:this={scriptEditorDrawer} />

<ResourceEditorDrawer
	bind:this={resourceEditorDrawer}
	{workspace}
	onRestored={onChanged}
	on:refresh={onChanged}
/>

<ConfirmationModal
	open={removingScorer != undefined}
	title="Remove this column"
	confirmationText="Remove"
	on:canceled={() => (removingScorer = undefined)}
	on:confirmed={async () => {
		const target = removingScorer
		removingScorer = undefined
		if (!target) return
		try {
			await saveScorers(scorers.filter((s) => s.id !== target.id))
		} catch (e) {
			sendUserToast(`Failed to remove the scorer: ${e}`, true)
		}
	}}
>
	<span class="text-sm">
		The column goes from every run of this dataset, the ones already recorded included. Adding it
		again starts a new column, which fills from the next run on.
	</span>
</ConfirmationModal>
