<script lang="ts">
	import { FlaskConical, History, Save } from 'lucide-svelte'
	import { onDestroy, untrack } from 'svelte'
	import { resource } from 'runed'
	import Modal, { type ModalTrailSegment } from '$lib/components/common/modal/Modal.svelte'
	import PagedContent from '$lib/components/common/modal/PagedContent.svelte'
	import { Badge, Button, Drawer, DrawerContent } from '$lib/components/common'
	import LocalDraftBanner from '$lib/components/LocalDraftBanner.svelte'
	import ResourceVersionHistory from '$lib/components/ResourceVersionHistory.svelte'
	import { clearPageDrawerAnchor } from '$lib/components/sessions/pageDrawerSession'
	import { RESOURCES_PATH } from '$lib/components/sessions/previewPaths'
	import EvalsPane from '$lib/components/aiEvals/EvalsPane.svelte'
	import type { EvalsLocation } from '$lib/components/aiEvals/evalUtils'
	import { ResourceService, type AgentDraft } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import {
		agentEditorTarget,
		agentWriteCount,
		type AgentEditorTarget,
		closeAgentEditor,
		markAgentWritten,
		showAgentEditorTool,
		showAgentEditorView
	} from '../agentEditorStore.svelte'
	import { agentConfigToInputTransforms } from '../agentResourceUtils'
	import { publishLinkedAgentTools } from '../flowState'
	import { linkedModulesForAgent, linkedToolsScope } from '../linkedAgentToolsStore.svelte'
	import AgentEditorHost from './AgentEditorHost.svelte'

	interface Props {
		enableAi?: boolean
		/** Which targets this mount is responsible for. The target is module-global while several
		 *  flow editors can be alive at once (a session retains every tab it has visited), so without
		 *  this each of them would build a whole editor — fetching, inferring tool schemas, and
		 *  running its own two-way sync against the one draft row. Required rather than defaulted: a
		 *  mount that guesses wrong renders nothing, and silence is a poor way to find that out. */
		owns: (target: AgentEditorTarget) => boolean
	}

	let { enableAi = false, owns }: Props = $props()

	// Every target names the surface that opened it, and only a flow step or a resource row can:
	// an agent used as a tool of the agent being edited stays part of it, with no way in this editor
	// to link it to a saved agent of its own. So `owns` decides alone — nothing here has to adopt a
	// target opened from inside another editor.
	let target = $derived.by(() => {
		const t = agentEditorTarget()
		return t && owns(t) ? t : undefined
	})
	// The last agent this dialog showed. The version pane keeps working after `onRestore` returns —
	// it reloads its list keyed on the path it was given — while the restore handler has already
	// closed the dialog and dropped the target. Reading the live one there throws, and the throw
	// lands in the global unhandled-rejection handler as a bare toast with no stack.
	let lastShownAgent = $state<string | undefined>(undefined)
	$effect(() => {
		const p = target?.path
		if (p) untrack(() => (lastShownAgent = p))
	})

	let ws = $derived(target?.workspace ?? $workspaceStore)
	let host = $state<ReturnType<typeof AgentEditorHost> | undefined>(undefined)
	let versionDrawer: Drawer | undefined = $state(undefined)
	let saving = $state(false)

	// Counted per agent, so a deploy that leaves the editor on the same agent still refetches the
	// version it just minted. Shared, so the step card behind the dialog refetches on the same signal.
	let writes = $derived(agentWriteCount(ws, target?.path))
	let versionResource = resource(
		() => ({ ws, path: target?.path, writes }),
		async ({ ws, path, writes }) => {
			if (!ws || !path) return { ws, path, writes }
			const history = await ResourceService.getResourceHistory({ workspace: ws, path })
			return { ws, path, writes, version: history.versions?.[0]?.version }
		}
	)
	// `writes` as well as the identity: runed neither aborts nor tags a superseded request, so a
	// response from before a deploy can land after the one for it and relabel the badge with the
	// version that deploy replaced.
	let version = $derived.by(() => {
		const loaded = versionResource.current
		return loaded !== undefined &&
			loaded.ws === ws &&
			loaded.path === target?.path &&
			loaded.writes === writes
			? loaded.version
			: undefined
	})

	let draft = $derived(host?.draftHandle())
	// The load refused this path, so there is no agent to deploy, evaluate or show a history of:
	// the dialog carries only the refusal the host renders.
	let refused = $derived(draft?.refusal != null)
	let inEvals = $derived(target?.view === 'evals' && !refused)
	let readOnly = $derived(draft ? !draft.canWrite : false)
	let draftOnly = $derived(draft?.noDeployed ?? false)

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

	let root = $derived<ModalTrailSegment>({
		label: target?.path ?? 'Agent',
		onclick: inEvals ? () => showAgentEditorView(undefined) : undefined
	})
	let trail = $derived<ModalTrailSegment[]>(
		inEvals
			? [
					root,
					{ label: 'Evals', onclick: evalsLocation ? evalsLocation.back : undefined },
					...(evalsLocation ? [{ label: evalsLocation.label }] : [])
				]
			: [root]
	)
	// The root's alone: below it the header's second line is the way back, and what a level is for
	// belongs to that level rather than to the dialog's own name.
	let description = $derived(inEvals || refused ? undefined : AGENT_DESCRIPTION)

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

	/** A resources-page row anchors itself in the hash on the way in, so closing has to clear it or
	 *  the URL keeps claiming the agent is open and a refresh reopens it. Anchored to that page, so
	 *  this is a no-op when the editor was opened from a flow. */
	function close() {
		closeAgentEditor()
		void clearPageDrawerAnchor(RESOURCES_PATH)
	}

	// The target is module-global and outlives this component: navigating away takes the mount that
	// owns it without passing through `close()`, and what it leaves behind reopens the dialog on the
	// way back and tells the flow editor a dialog is over its graph. Only its own, so a mount going
	// down never drops a target another one is showing.
	onDestroy(() => {
		const t = agentEditorTarget()
		if (t && owns(t)) closeAgentEditor()
	})

	/** What the flow behind a write has to be told about it, captured before the request rather than
	 *  read after: the dialog stays closable while one is in flight and can be pointed at another
	 *  agent by the time it lands, and the reactive `ws`/`target` would then name that one. */
	type WriteTarget = { ws: string | undefined; path: string; host: AgentEditorTarget['host'] }
	function currentWriteTarget(): WriteTarget | undefined {
		const t = target
		return t ? { ws, path: t.path, host: t.host } : undefined
	}
	/** Reconciling is best-effort: it refreshes what a write changed, and the write itself has
	 *  already landed. A rejection here must not surface as an unhandled one — the global handler
	 *  turns those into a bare toast with no stack, which says nothing about where it came from. */
	function reconcileQuietly(at: WriteTarget | undefined, path: string) {
		try {
			void Promise.resolve(reconcile(at, path)).catch((err) => {
				console.error('agent editor: could not refresh what links to', path, err)
			})
		} catch (err) {
			console.error('agent editor: could not refresh what links to', path, err)
		}
	}

	function reconcile(at: WriteTarget | undefined, path: string) {
		if (!at?.ws) return
		markAgentWritten(at.ws, path)
		if (!at.host) return
		// The host graph resolves a linked agent's tool nodes from the resource, so it has to re-read
		// what the write just changed. Every step of that flow linking this agent, not only the one
		// the editor was opened from: they all show tools the write may have moved.
		const scope = linkedToolsScope(at.ws, at.host.flowPath)
		const moduleIds = new Set(linkedModulesForAgent(scope, path))
		moduleIds.add(at.host.moduleId)
		return Promise.all(
			[...moduleIds].map((moduleId) => publishLinkedAgentTools(path, at.ws, scope, moduleId))
		)
	}

	let deployingFor = $state<WriteTarget | undefined>(undefined)

	async function onDeploy() {
		saving = true
		deployingFor = currentWriteTarget()
		try {
			await host?.deploy()
		} finally {
			saving = false
			deployingFor = undefined
		}
	}

	/** What a successful deploy has to reconcile. The path is the one it wrote, which `deploy` holds
	 *  to the one the editor opened: this editor does not rename. */
	async function onSaved(savedPath: string) {
		await reconcile(deployingFor ?? currentWriteTarget(), savedPath)
	}
</script>

{#if target}
	{#key `${ws}:${target.path}`}
		<!-- Bound rather than held open: the store is what decides whether this is mounted, so every
		     way out has to reach it. The close button and the backdrop only set `open`, and a
		     dialog left closed over a target still set could never be opened again.

		     `paginated` throughout: the body is one strip of pages (the agent form, then evals), so
		     the header keeps its height and only the pages move. -->
		<Modal
			bind:open={() => true, (open) => !open && close()}
			kind="X"
			fillHeight
			enterConfirms={false}
			paginated
			title={target.path}
			{trail}
			{description}
			class="w-[92vw] sm:w-[92vw] max-w-[1500px] sm:max-w-[1500px] h-[88vh]"
		>
			{#snippet titleBadge()}
				<!-- Against the agent's own name wherever it appears, as the linked-agent card in the
				     step panel has it. -->
				{#if version != undefined && !refused}
					<Badge color="gray" class="shrink-0" title="The version runs are recorded against">
						v{version}
					</Badge>
				{/if}
			{/snippet}
			{#snippet levelBadge()}
				<!-- Marks evals, not the agent, so it sits against that level's own name. Dropped a
				     level deeper, where it would read as marking the run rather than the feature it
				     belongs to. -->
				{#if inEvals && !evalsLocation}
					<Badge color="blue" small class="shrink-0 !py-0 leading-4">Beta</Badge>
				{/if}
			{/snippet}
			<!-- Evals carries none of the editor's actions: nothing there edits the agent, and the run
			     dialog states for itself whether a run is against the deployed version or the edits. -->
			{#snippet settings()}
				<div class="flex flex-row items-center gap-2 shrink-0">
					{#if !inEvals && !refused}
						{#if readOnly}
							<Badge
								color="gray"
								class="shrink-0"
								title="You do not have write access to this agent"
							>
								Read only
							</Badge>
						{/if}
						<!-- Evals run against the deployed agent, and a draft-only one has none: the
						     backend's `require_agent` would reject every run. -->
						{#if !draftOnly}
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
							disabled={readOnly}
							title={readOnly ? 'You do not have write access to this agent' : undefined}
							on:click={onDeploy}
						>
							Deploy
						</Button>
					{/if}
				</div>
			{/snippet}
			<div class="h-full min-h-0 flex flex-col">
				<!-- Full-bleed, as a banner is everywhere else: the dialog's own horizontal padding is
				     cancelled so it spans the body. -->
				<div class="-mx-4 sm:-mx-6 shrink-0 {inEvals ? 'hidden' : ''}">
					<LocalDraftBanner
						show={draft?.sync.hasDraft ?? false}
						reserveSpace={draft?.sync.hasBaseline ?? false}
						getDeployed={() => draft?.deployed}
						getCurrent={() => draft?.state}
						onDiscard={() => draft?.sync.resetToDeployed(target?.path ?? '')}
						title="Deployed <> Unsaved agent changes"
					/>
				</div>
				<!-- Two pages of one strip, so opening evals slides in from the right the way its own
				     levels do. No `onNavigate`: the arrow keys belong to whichever pane is on screen,
				     and evals answers them for its own levels. -->
				<PagedContent
					class="flex-1 min-h-0"
					current={inEvals ? 'evals' : 'agent'}
					pages={[
						{ key: 'agent', content: agentPage },
						{ key: 'evals', content: evalsPage }
					]}
				/>
			</div>
		</Modal>
	{/key}

	<!-- Mounted under the evals level too: it holds the draft the header's banner and Deploy act on,
	     and the config a draft run is offered on. -->
	{#snippet agentPage()}
		<AgentEditorHost
			bind:this={host}
			path={target.path}
			workspace={ws}
			{enableAi}
			toolId={target.toolId}
			onSelectTool={(id) => showAgentEditorTool(id)}
			{onSaved}
		/>
	{/snippet}

	{#snippet evalsPage()}
		<EvalsPane
			agentPath={target.path}
			opWorkspace={ws}
			editedConfig={draft?.sync.hasDraft ? editedConfig : undefined}
			bind:location={evalsLocation}
			active={inEvals}
		/>
	{/snippet}

	<Drawer bind:this={versionDrawer} size="1200px">
		<DrawerContent title="Version history" on:close={() => versionDrawer?.closeDrawer()} noPadding>
			<ResourceVersionHistory
				path={target?.path ?? lastShownAgent ?? ''}
				workspace={ws}
				canRestore={!readOnly}
				onRestore={() => {
					versionDrawer?.closeDrawer()
					// A restore writes the resource as a deploy does, so the flow behind has to be told
					// the same way. Captured before closing, which drops the target this reads.
					const at = currentWriteTarget()
					// Close the editor too, as the generic resource editor does on a restore: it holds
					// a baseline captured before the restore, and any local draft on top of it, so
					// deploying from it afterwards would write the pre-restore value straight back over
					// the version just restored.
					close()
					if (at) reconcileQuietly(at, at.path)
				}}
			/>
		</DrawerContent>
	</Drawer>
{/if}
