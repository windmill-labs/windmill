<script lang="ts">
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonMore from '$lib/components/common/toggleButton-v2/ToggleButtonMore.svelte'
	import { ResourceService, type AgentDraft, type EvalDataset, type EvalSubject } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import MissingWorkerTagAlert from '$lib/components/jobs/MissingWorkerTagAlert.svelte'
	import { Pencil, Play, Plus } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { datasetSummary } from './evalUtils'

	let {
		open = $bindable(),
		workspace,
		agentPath,
		datasets,
		defaultDataset,
		editedConfig = undefined,
		running = false,
		onRun,
		onEditDataset,
		onNewDataset
	}: {
		open: boolean
		workspace: string | undefined
		agentPath: string
		datasets: EvalDataset[]
		/** The dataset to open on: the one last worked in, or the one a run was read from. */
		defaultDataset: string | undefined
		/** Opened from an agent being edited: the edits, as the step holds them when Run is
		 *  pressed. Offered, and preselected, as a subject of their own. */
		editedConfig?: () => AgentDraft
		running?: boolean
		onRun: (subject: EvalSubject, dataset: string) => boolean | void | Promise<boolean | void>
		onEditDataset: (path: string) => void
		onNewDataset: () => void
	} = $props()

	/** What to run: `draft`, `deployed`, or an earlier version's number. One value for the whole
	 *  choice, so the toggle and the overflow menu cannot disagree about what was chosen. */
	let choice = $state<string>('deployed')
	/** Whether the edits are on offer: only when opened from an agent being edited. */
	let hasDraft = $derived(editedConfig !== undefined)
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
			versions = []
			sendUserToast(`Could not read ${agentPath}'s versions: ${e}`, true)
		}
	}

	/** Whether the dialog was already open on the previous run of the effect below, which reads it
	 *  to tell an open from the pane's dataset moving underneath. */
	let wasOpen = false

	$effect(() => {
		const isOpen = open
		const pane = defaultDataset
		untrack(() => {
			if (!isOpen) {
				wasOpen = false
				return
			}
			// Followed while the dialog stands, not only read at open: the dataset drawer opens over
			// this dialog rather than in place of it, so creating, renaming or deleting a dataset
			// there moves the pane's selection with the dialog still up. Nothing else moves it then.
			dataset = pane
			if (wasOpen) return
			wasOpen = true
			loadVersions()
			// The state of the agent there is most reason to measure.
			choice = hasDraft ? 'draft' : 'deployed'
		})
	})

	let latest = $derived(versions[0]?.version)
	// Everything but the newest, which is what `deployed` reads: offering it here would offer the
	// same run under two names.
	let olderVersions = $derived(versions.slice(1))
	let olderItems = $derived(
		olderVersions.map((v) => ({ label: `v${v.version}`, value: String(v.version) }))
	)
	/** Whether the choice came out of the overflow menu, which then shows it and drops its label. */
	let pickedOlder = $derived(olderItems.some((i) => i.value === choice))

	let datasetItems = $derived(
		[...datasets]
			.sort((a, b) => {
				// This agent's own first: a dataset is named under what it tests, so they cluster.
				const own = (d: EvalDataset) =>
					d.path.startsWith(`${agentPath}_`) || d.path.startsWith(`${agentPath}/`) ? 0 : 1
				return own(a) - own(b) || a.path.localeCompare(b.path)
			})
			.map((d) => ({
				label: d.summary || d.path,
				value: d.path,
				subtitle: d.summary ? d.path : undefined
			}))
	)

	function subjectOf(): EvalSubject {
		// The edits are read when Run is pressed, not when the dialog opened: the step stays live
		// behind it, and the run is of what is on screen at that moment.
		if (choice === 'draft') return { kind: 'agent_draft', path: agentPath, draft: editedConfig?.() }
		if (choice === 'deployed') return { kind: 'agent', path: agentPath }
		return { kind: 'agent_version', path: agentPath, version: Number(choice) }
	}

	/** What the closed field reads: the list stacks the summary over the path, which one line
	 *  cannot do, so it says both the other way round. */
	function datasetFieldText(text: string, path: unknown): string {
		const summary = datasetSummary(datasets, path)
		return summary ? `${summary} (${path})` : text
	}
</script>

<Modal title="Run evaluation" bind:open>
	<div class="flex flex-col gap-6 min-w-96">
		<Label
			label="Agent version"
			tooltip="Whichever you pick is read once, when the run starts, and every case runs that same configuration. Deploying an edit part-way through changes what the next run measures, never this one."
		>
			<ToggleButtonGroup bind:selected={choice} class="w-fit">
				{#snippet children({ item })}
					{#if hasDraft}
						<ToggleButton
							value="draft"
							label={latest ? `v${latest} + edits (current)` : 'Edits (current)'}
							tooltip="The edits in the step, as they are when Run is pressed."
							{item}
						/>
					{/if}
					<ToggleButton
						value="deployed"
						label={latest ? `v${latest} (latest deployed)` : 'Latest deployed'}
						tooltip="The agent as it is saved right now."
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

		<Label label="Dataset" tooltip="The set of cases the agent is measured on.">
			{#if datasets.length === 0}
				<div class="flex flex-col items-start gap-2">
					<span class="text-xs text-secondary">
						A run measures the agent on a set of cases, and there is no set yet.
					</span>
					<Button
						unifiedSize="sm"
						variant="default"
						startIcon={{ icon: Plus }}
						onclick={onNewDataset}
					>
						New dataset
					</Button>
				</div>
			{:else}
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
						{#snippet endSnippet({ item, close })}
							<Button
								variant="subtle"
								unifiedSize="sm"
								wrapperClasses="-mr-2 pl-1 -my-2"
								btnClasses="hover:bg-surface-tertiary"
								startIcon={{ icon: Pencil }}
								iconOnly
								title="Edit this dataset"
								on:click={() => {
									close()
									onEditDataset(item.value ?? '')
								}}
							/>
						{/snippet}
						{#snippet bottomSnippet({ close })}
							<div class="flex flex-col border-t">
								<Button
									variant="subtle"
									unifiedSize="sm"
									wrapperClasses="w-full"
									btnClasses="w-full !h-auto !justify-start !rounded-none flex items-center gap-2 px-3 py-2 text-xs !font-normal text-secondary hover:bg-surface-hover"
									onClick={() => {
										close()
										onNewDataset()
									}}
								>
									<Plus size={13} />
									New dataset
								</Button>
							</div>
						{/snippet}
					</Select>
					{#if dataset && hoveringDataset}
						<div class="absolute right-10 z-20">
							<Button
								variant="subtle"
								unifiedSize="sm"
								wrapperClasses="pl-1"
								btnClasses="hover:bg-surface-tertiary"
								startIcon={{ icon: Pencil }}
								iconOnly
								title="Edit this dataset"
								on:click={() => onEditDataset(dataset ?? '')}
							/>
						</div>
					{/if}
				</div>
			{/if}
		</Label>

		<!-- A run ends with a native job, whose tag belongs to the `native` worker group rather than
		     the default one: nothing serving it means the run queues rather than fails. -->
		<MissingWorkerTagAlert tag="nativets" subject="Eval runs" {workspace} />
	</div>
	{#snippet actions()}
		<Button
			unifiedSize="md"
			variant="accent"
			startIcon={{ icon: Play }}
			loading={running}
			disabled={running || !dataset}
			onclick={async () => {
				if (!dataset) return
				// Close only when the launch succeeded: a failed one leaves the dialog open so the
				// selection survives and the run can be retried.
				if (await onRun(subjectOf(), dataset)) open = false
			}}
		>
			Run
		</Button>
	{/snippet}
</Modal>
