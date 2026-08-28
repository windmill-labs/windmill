<script lang="ts">
	import { FlaskConical, History, Save } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { resource } from 'runed'
	import Modal, { type ModalTrailSegment } from '$lib/components/common/modal/Modal.svelte'
	import { Badge, Button, Drawer, DrawerContent } from '$lib/components/common'
	import LocalDraftBanner from '$lib/components/LocalDraftBanner.svelte'
	import ResourceVersionHistory from '$lib/components/ResourceVersionHistory.svelte'
	import EvalsPane from '$lib/components/aiEvals/EvalsPane.svelte'
	import type { EvalsLocation } from '$lib/components/aiEvals/evalUtils'
	import { ResourceService, type AgentDraft } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import {
		agentEditorTarget,
		closeAgentEditor,
		showAgentEditorTool,
		showAgentEditorView
	} from '../agentEditorStore.svelte'
	import { agentConfigToInputTransforms } from '../agentResourceUtils'
	import { toolDisplayName } from '../agentToolUtils'
	import { publishLinkedAgentTools } from '../flowState'
	import { linkedToolsScope } from '../linkedAgentToolsStore.svelte'
	import AgentEditorHost from './AgentEditorHost.svelte'

	interface Props {
		enableAi?: boolean
	}

	let { enableAi = false }: Props = $props()

	let target = $derived(agentEditorTarget())
	let ws = $derived(target?.workspace ?? $workspaceStore)
	let host = $state<ReturnType<typeof AgentEditorHost> | undefined>(undefined)
	let versionDrawer: Drawer | undefined = $state(undefined)
	let saving = $state(false)

	// Bumped after every write, so a deploy that leaves the editor on the same agent still refetches
	// the version it just minted.
	let writes = $state(0)
	let versionResource = resource(
		() => ({ ws, path: target?.path, writes }),
		async ({ ws, path }) => {
			if (!ws || !path) return { ws, path }
			const history = await ResourceService.getResourceHistory({ workspace: ws, path })
			return { ws, path, version: history.versions?.[0]?.version }
		}
	)
	let version = $derived.by(() => {
		const loaded = versionResource.current
		return loaded !== undefined && loaded.ws === ws && loaded.path === target?.path
			? loaded.version
			: undefined
	})

	let draft = $derived(host?.draftHandle())
	let tools = $derived((draft?.state?.args?.tools ?? []) as { id: string; summary?: string }[])
	let openTool = $derived(target?.toolId ? tools.find((t) => t.id === target?.toolId) : undefined)
	let inEvals = $derived(target?.view === 'evals')

	// Where the evals pane is within itself, so its levels extend this dialog's trail rather than
	// opening a dialog of their own. Cleared on the way in: the pane reports a level once it is on
	// one, and never that it is back at its root.
	let evalsLocation = $state<EvalsLocation | undefined>(undefined)
	let evalsKey = $derived(`${ws}:${target?.path}:${target?.view}`)
	$effect(() => {
		evalsKey
		untrack(() => (evalsLocation = undefined))
	})

	const AGENT_DESCRIPTION = 'Changes here update the saved agent, and every flow that links to it.'
	const EVALS_DESCRIPTION =
		'Each run answers a dataset of cases with this agent and scores the answers, so runs can be compared.'

	let root = $derived<ModalTrailSegment>({
		label: target?.path ?? 'Agent',
		onclick: openTool || inEvals ? () => showAgentEditorView(undefined) : undefined
	})
	let trail = $derived<ModalTrailSegment[]>(
		openTool
			? [root, { label: toolDisplayName(openTool as any) }]
			: inEvals
				? [
						root,
						{ label: 'Evals', onclick: evalsLocation ? evalsLocation.back : undefined },
						...(evalsLocation ? [{ label: evalsLocation.label }] : [])
					]
				: [root]
	)
	let description = $derived(
		inEvals ? (evalsLocation ? undefined : EVALS_DESCRIPTION) : AGENT_DESCRIPTION
	)

	/** The unsaved edits, in the shape the server builds from a deployed config
	 *  (`ai_evals/run.rs` `config_to_draft`), so a draft run's hash can be recognised as equal to
	 *  the version it is later deployed as. */
	function editedConfig(): AgentDraft {
		const args = draft?.state?.args
		return {
			input_transforms: agentConfigToInputTransforms(args ?? {}) as Record<string, unknown>,
			tools: (args?.tools ?? []) as unknown[] as Record<string, unknown>[]
		}
	}

	async function onDeploy() {
		saving = true
		try {
			const ok = await host?.deploy()
			if (ok) {
				writes++
				const h = target?.host
				if (h && ws && target?.path) {
					// The host graph resolves a linked agent's tool nodes from the resource, so it has
					// to re-read now that the resource moved.
					await publishLinkedAgentTools(
						target.path,
						ws,
						linkedToolsScope(ws, h.flowPath),
						h.moduleId
					)
				}
			}
		} finally {
			saving = false
		}
	}
</script>

{#if target}
	{#key `${ws}:${target.path}`}
		<Modal
			open={true}
			kind="X"
			fillHeight
			enterConfirms={false}
			title={target.path}
			{trail}
			{description}
			class="w-[92vw] sm:w-[92vw] max-w-[1500px] sm:max-w-[1500px] h-[88vh]"
			on:canceled={closeAgentEditor}
		>
			{#snippet titleBadge()}
				{#if version != undefined}
					<Badge color="gray" class="shrink-0" title="The version runs are recorded against">
						v{version}
					</Badge>
				{/if}
			{/snippet}
			{#snippet settings()}
				<div class="flex flex-row items-center gap-2 shrink-0">
					{#if inEvals}
						<!-- Marks the level, not the agent, so it sits with the actions rather than in
						     `titleBadge`, which names the dialog. -->
						<Badge color="blue" small class="shrink-0 !py-0 leading-4">Beta</Badge>
					{:else}
						<Button
							unifiedSize="sm"
							variant="default"
							startIcon={{ icon: FlaskConical }}
							title="Run this agent against a dataset of cases"
							on:click={() => showAgentEditorView('evals')}
						>
							Evals
						</Button>
					{/if}
					<Button
						unifiedSize="sm"
						variant="default"
						startIcon={{ icon: History }}
						iconOnly
						title="Version history"
						on:click={() => versionDrawer?.openDrawer()}
					/>
					<Button
						unifiedSize="sm"
						variant="accent"
						startIcon={{ icon: Save }}
						loading={saving}
						on:click={onDeploy}
					>
						Deploy
					</Button>
				</div>
			{/snippet}
			<div class="h-full min-h-0 flex flex-col">
				<!-- Full-bleed, as a banner is everywhere else: the dialog's own horizontal padding is
				     cancelled so it spans the body. -->
				<div class="-mx-4 sm:-mx-6 shrink-0">
					<LocalDraftBanner
						show={draft?.sync.hasDraft ?? false}
						reserveSpace={draft?.sync.hasBaseline ?? false}
						getDeployed={() => draft?.deployed}
						getCurrent={() => draft?.state}
						onDiscard={() => draft?.sync.resetToDeployed(target?.path ?? '')}
						title="Deployed <> Unsaved agent changes"
					/>
				</div>
				<div class="flex-1 min-h-0">
					<!-- The host stays mounted under the evals level: it holds the draft the header's
					     banner and Deploy act on, and the config a draft run is offered on. -->
					<div class="h-full min-h-0 {inEvals ? 'hidden' : ''}">
						<AgentEditorHost
							bind:this={host}
							path={target.path}
							workspace={ws}
							{enableAi}
							toolId={target.toolId}
							onSelectTool={(id) => showAgentEditorTool(id)}
						/>
					</div>
					{#if inEvals}
						<EvalsPane
							agentPath={target.path}
							opWorkspace={ws}
							editedConfig={draft?.sync.hasDraft ? editedConfig : undefined}
							bind:location={evalsLocation}
						/>
					{/if}
				</div>
			</div>
		</Modal>
	{/key}

	<Drawer bind:this={versionDrawer} size="1200px">
		<DrawerContent title="Version history" on:close={() => versionDrawer?.closeDrawer()} noPadding>
			<ResourceVersionHistory
				path={target.path}
				workspace={ws}
				onRestore={() => {
					writes++
					versionDrawer?.closeDrawer()
					// The restored value is now the deployed one; reload so the editor is not left
					// showing the version it replaced.
					host?.reloadFromResource()
				}}
			/>
		</DrawerContent>
	</Drawer>
{/if}
