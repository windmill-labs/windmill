<script lang="ts">
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { ResourceService, type EvalDataset, type EvalSubject } from '$lib/gen'
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

	/** `deployed`, `draft`, or an agent version number as a string: the picker's values are the
	 *  three shapes a subject comes in, and the number is what an `agent_version` needs. */
	let version = $state('deployed')
	let dataset = $state<string | undefined>(undefined)

	/** The agent's versions, for pinning one. Loaded when the dialog opens rather than held: a
	 *  version list goes stale the moment the agent is saved again. */
	let versions = $state<{ version: number; created_at?: string }[]>([])
	let loadingVersions = $state(false)

	async function loadVersions() {
		if (!workspace) return
		loadingVersions = true
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
		} finally {
			loadingVersions = false
		}
	}

	$effect(() => {
		if (!open) return
		untrack(() => {
			// Seeded on every open: the dataset you were last in, and whichever state of the agent
			// there is most reason to measure — edits waiting are why you came.
			dataset = defaultDataset ?? datasets[0]?.path
			version = hasUndeployedChanges ? 'draft' : 'deployed'
			loadVersions()
		})
	})

	let latest = $derived(versions[0]?.version)
	let versionItems = $derived([
		...(hasUndeployedChanges
			? [{ label: 'Unsaved edits', value: 'draft', subtitle: 'the draft on top of what is saved' }]
			: []),
		{
			label: latest ? `v${latest} (latest)` : 'Latest',
			value: 'deployed',
			subtitle: 'resolved when the run executes, as in production'
		},
		...versions.slice(1).map((v) => ({
			label: `v${v.version}`,
			value: String(v.version),
			subtitle: 'inlined as it was'
		}))
	])

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
		if (version === 'draft') return { kind: 'agent_draft', path: agentPath }
		if (version === 'deployed') return { kind: 'agent', path: agentPath }
		return { kind: 'agent_version', path: agentPath, version: Number(version) }
	}
</script>

<Modal title="Run the agent" bind:open>
	<div class="flex flex-col gap-6 min-w-96">
		<Label
			label="Agent version"
			tooltip="Latest resolves the agent when the run executes, the way a flow step does. A past version is inlined as it was, which is the only way to run one. Unsaved edits run the draft."
		>
			<!-- The subtitles are the point of the list: what each choice actually executes is the
			     difference between them, and it is not in a version number. -->
			<Select items={versionItems} bind:value={version} loading={loadingVersions} class="text-xs" />
		</Label>

		<Label label="Dataset" tooltip="The set of cases the agent is measured on.">
			<Select
				items={datasetItems}
				bind:value={dataset}
				placeholder="Select a dataset"
				class="text-xs"
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
