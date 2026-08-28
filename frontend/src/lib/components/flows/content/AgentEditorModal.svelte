<script lang="ts">
	import { FlaskConical, History, Save } from 'lucide-svelte'
	import { resource } from 'runed'
	import Modal, { type ModalTrailSegment } from '$lib/components/common/modal/Modal.svelte'
	import { Badge, Button, Drawer, DrawerContent } from '$lib/components/common'
	import LocalDraftBanner from '$lib/components/LocalDraftBanner.svelte'
	import ResourceVersionHistory from '$lib/components/ResourceVersionHistory.svelte'
	import AgentEvalModal from '$lib/components/aiEvals/AgentEvalModal.svelte'
	import { ResourceService, type AgentDraft } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import {
		agentEditorTarget,
		closeAgentEditor,
		showAgentEditorTool
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
	let evalsOpen = $state(false)
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

	let trail = $derived<ModalTrailSegment[]>(
		openTool
			? [
					{ label: target?.path ?? 'Agent', onclick: () => showAgentEditorTool(undefined) },
					{ label: toolDisplayName(openTool as any) }
				]
			: [{ label: target?.path ?? 'Agent' }]
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
			description="Changes here update the saved agent, and every flow that links to it."
			class="w-[92vw] sm:w-[92vw] max-w-[1500px] sm:max-w-[1500px] h-[88vh]"
			on:canceled={closeAgentEditor}
		>
			<div class="h-full min-h-0 flex flex-col">
				{#if !openTool}
					<div class="flex flex-row items-center gap-2 px-4 py-2 border-b border-light shrink-0">
						{#if version != undefined}
							<Badge color="gray" class="shrink-0" title="The version runs are recorded against">
								v{version}
							</Badge>
						{/if}
						<div class="grow min-w-0">
							<LocalDraftBanner
								show={draft?.sync.hasDraft ?? false}
								reserveSpace={draft?.sync.hasBaseline ?? false}
								getDeployed={() => draft?.deployed}
								getCurrent={() => draft?.state}
								onDiscard={() => draft?.sync.resetToDeployed(target?.path ?? '')}
								title="Deployed <> Unsaved agent changes"
							/>
						</div>
						<Button
							unifiedSize="sm"
							variant="default"
							startIcon={{ icon: FlaskConical }}
							title="Run this agent against a dataset of cases"
							on:click={() => (evalsOpen = true)}
						>
							Evals
						</Button>
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
				{/if}
				<div class="flex-1 min-h-0">
					<AgentEditorHost
						bind:this={host}
						path={target.path}
						workspace={ws}
						{enableAi}
						toolId={target.toolId}
						onSelectTool={(id) => showAgentEditorTool(id)}
					/>
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

	<AgentEvalModal
		agentPath={target.path}
		opWorkspace={ws}
		editedConfig={draft?.sync.hasDraft ? editedConfig : undefined}
		bind:open={evalsOpen}
	/>
{/if}
