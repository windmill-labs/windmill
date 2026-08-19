<script lang="ts">
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonMore from '$lib/components/common/toggleButton-v2/ToggleButtonMore.svelte'
	import { AiEvalsService, ResourceService, type EvalDataset, type EvalSubject } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Pencil, Play, Plus } from 'lucide-svelte'
	import { untrack } from 'svelte'

	let {
		open = $bindable(false),
		workspace,
		agentPath,
		datasets,
		/** The dataset to open on: the one last worked in, or the one a run was read from. */
		defaultDataset,
		/** Whether the agent has edits waiting, which is the one option the history cannot show. */
		hasUndeployedChanges,
		running = false,
		onRun,
		onEditDataset,
		onNewDataset
	}: {
		open?: boolean
		workspace: string | undefined
		agentPath: string
		datasets: EvalDataset[]
		defaultDataset: string | undefined
		hasUndeployedChanges: boolean
		running?: boolean
		onRun: (subject: EvalSubject, dataset: string) => void | Promise<void>
		onEditDataset: (path: string) => void
		onNewDataset: () => void
	} = $props()

	/** What to run: `draft`, `deployed`, or an earlier version's number. One value for the whole
	 *  choice, so the toggle and the overflow menu cannot disagree about what was chosen. */
	let choice = $state<string>('deployed')
	/** Whether anyone has chosen since this opened, so a late answer about the draft can preselect
	 *  it without overruling a choice already made. */
	let touched = $state(false)
	/** Read here rather than taken from the caller, whose own copy is polled: an agent edited a
	 *  moment ago has a draft that poll has not seen yet, and the option would be missing exactly
	 *  when it is the reason the dialog was opened. */
	let hasDraft = $state(false)
	let dataset = $state<string | undefined>(undefined)
	let hoveringDataset = $state(false)

	/** The agent's versions, for pinning one. Loaded when the dialog opens rather than held: a
	 *  version list goes stale the moment the agent is saved again. */
	let versions = $state<{ version: number; created_at?: string }[]>([])

	async function loadVersions() {
		if (!workspace) return
		try {
			const history = await ResourceService.getResourceHistory({ workspace, path: agentPath })
			versions = (history.versions ?? []).map((v) => ({
				version: v.version,
				created_at: v.created_at
			}))
		} catch (e) {
			// The history is an offer, not the dialog: a run of what is deployed needs none of it.
			versions = []
			sendUserToast(`Could not read ${agentPath}'s versions: ${e}`, true)
		}
	}

	/** Whether the agent has edits waiting. Asked again on open so the answer is this moment's. */
	async function loadDraftState() {
		if (!workspace) return
		try {
			const state = await AiEvalsService.evalSubjectState({ workspace, path: agentPath })
			hasDraft = state.has_undeployed_changes
			// A draft that turned up after the dialog opened is still the reason to have opened it.
			if (hasDraft && !touched) choice = 'draft'
			if (!hasDraft && choice === 'draft') choice = 'deployed'
		} catch {
			// Leaves the toggle on what the caller knew, which is the last thing it saw.
		}
	}

	$effect(() => {
		if (!open) return
		untrack(() => {
			// Seeded on every open: the dataset you were last in, and whichever state of the agent
			// there is most reason to measure — edits waiting are why you came.
			dataset = defaultDataset ?? datasets[0]?.path
			hasDraft = hasUndeployedChanges
			choice = hasUndeployedChanges ? 'draft' : 'deployed'
			touched = false
			loadVersions()
			loadDraftState()
		})
	})

	let latest = $derived(versions[0]?.version)
	// Everything but the newest: the newest is what `deployed` resolves to, and offering it twice
	// would be offering a run that is live-resolved and one that is pinned under the same name.
	let olderVersions = $derived(versions.slice(1))
	let olderItems = $derived(
		olderVersions.map((v) => ({ label: `v${v.version}`, value: String(v.version) }))
	)
	/** Whether the choice came out of the overflow menu, which then shows it and drops its own
	 *  label: the version you picked is worth more room in the group than the word "More". */
	let pickedOlder = $derived(olderItems.some((i) => i.value === choice))

	let datasetItems = $derived(
		[...datasets]
			.sort((a, b) => {
				// This agent's own first: a dataset is named under what it tests, so they cluster.
				const own = (d: EvalDataset) =>
					d.path.startsWith(`${agentPath}_`) || d.path.startsWith(`${agentPath}/`) ? 0 : 1
				return own(a) - own(b) || a.path.localeCompare(b.path)
			})
			// Named by what it is for, with the path under it: a path is how you tell two of them
			// apart, and a summary is how you know which one you meant.
			.map((d) => ({
				label: d.summary || d.path,
				value: d.path,
				subtitle: d.summary ? d.path : undefined
			}))
	)

	function subjectOf(): EvalSubject {
		if (choice === 'draft') return { kind: 'agent_draft', path: agentPath }
		if (choice === 'deployed') return { kind: 'agent', path: agentPath }
		return { kind: 'agent_version', path: agentPath, version: Number(choice) }
	}

	/** What the closed field reads. The list stacks the summary over the path, which a one-line
	 *  field cannot do, so it says both the other way round: a summary names the one you meant and
	 *  the path is how you tell two of them apart. */
	function datasetFieldText(text: string, path: unknown): string {
		const summary = datasets.find((d) => d.path === path)?.summary
		return summary ? `${summary} (${path})` : text
	}
</script>

<Modal title="Run evaluation" bind:open>
	<div class="flex flex-col gap-6 min-w-96">
		<Label
			label="Agent version"
			tooltip="The saved agent is resolved when the run executes, as it is in production. Its draft and any earlier version are inlined as they are, which is the only way to run either."
		>
			<!-- The two worth naming are the two you are choosing between while editing; every
			     earlier version is one click further, since running one is a deliberate act. -->
			<ToggleButtonGroup bind:selected={choice} onSelected={() => (touched = true)} class="w-fit">
				{#snippet children({ item })}
					{#if hasDraft}
						<ToggleButton
							value="draft"
							label={latest ? `v${latest} draft` : 'Draft'}
							tooltip="The edits waiting on the agent, as they are before saving."
							{item}
						/>
					{/if}
					<ToggleButton
						value="deployed"
						label={latest ? `v${latest} (latest)` : 'Latest'}
						tooltip="The agent as saved, resolved when the run executes."
						{item}
					/>
					{#if olderItems.length > 0}
						<!-- Keyed on the versions it was built from: the menu reads its items once, and
						     they arrive after this dialog opens. -->
						{#key olderItems.map((i) => i.value).join(',')}
							<ToggleButtonMore
								btnText={pickedOlder ? '' : 'More'}
								togglableItems={olderItems}
								bind:selected={choice}
								{item}
							/>
						{/key}
					{/if}
				{/snippet}
			</ToggleButtonGroup>
		</Label>

		<!-- The pencil rides the field itself, as a resource picker's does: the dataset you are about
		     to measure against is exactly when you notice a case is missing from it. -->
		<Label label="Dataset" tooltip="The set of cases the agent is measured on.">
			<div
				class="relative flex flex-row items-center w-full"
				role="group"
				onmouseenter={() => (hoveringDataset = true)}
				onmouseleave={() => (hoveringDataset = false)}
			>
				<Select
					items={datasetItems}
					bind:value={dataset}
					placeholder="Select a dataset"
					clearable
					class="text-xs w-full"
					transformInputSelectedText={datasetFieldText}
				>
					<!-- The way into a dataset from the row that names it, as a resource picker does: what
				     you are about to measure is exactly when you notice a case is missing. -->
					{#snippet endSnippet({ item, close })}
						<Button
							variant="subtle"
							size="xs2"
							wrapperClasses="-mr-2 pl-1 -my-2"
							btnClasses="hover:bg-surface-tertiary"
							startIcon={{ icon: Pencil }}
							iconOnly
							title="Edit this dataset"
							on:click={() => {
								close()
								open = false
								onEditDataset(item.value ?? '')
							}}
						/>
					{/snippet}
					{#snippet bottomSnippet({ close })}
						<div class="flex flex-col border-t">
							<button
								type="button"
								class="flex items-center gap-2 px-3 py-2 text-xs text-secondary hover:bg-surface-hover"
								onclick={() => {
									close()
									open = false
									onNewDataset()
								}}
							>
								<Plus size={13} />
								New dataset
							</button>
						</div>
					{/snippet}
				</Select>
				{#if dataset && hoveringDataset}
					<div class="absolute right-10 z-20">
						<Button
							variant="subtle"
							size="xs2"
							wrapperClasses="pl-1"
							btnClasses="hover:bg-surface-tertiary"
							startIcon={{ icon: Pencil }}
							iconOnly
							title="Edit this dataset"
							on:click={() => {
								open = false
								onEditDataset(dataset ?? '')
							}}
						/>
					</div>
				{/if}
			</div>
		</Label>
	</div>
	{#snippet actions()}
		<Button
			size="xs"
			variant="accent"
			startIcon={{ icon: Play }}
			loading={running}
			disabled={running || !dataset}
			onclick={async () => {
				if (!dataset) return
				await onRun(subjectOf(), dataset)
				open = false
			}}
		>
			Run
		</Button>
	{/snippet}
</Modal>
