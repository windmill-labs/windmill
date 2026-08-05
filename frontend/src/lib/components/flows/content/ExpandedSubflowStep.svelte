<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import { FlowService, type FlowModule } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import FlowGraphViewerStep from '$lib/components/FlowGraphViewerStep.svelte'
	import { ChevronRight, ExternalLink, Pen } from 'lucide-svelte'
	import { sendUserToast } from '$lib/utils'
	import type { FlowEditorContext } from '../types'
	import {
		resolveExpandedSubflowStep,
		type ResolvedExpandedSubflowStep
	} from '../expandedSubflowStep'
	import { parseExpandedSubflowId } from '$lib/components/restartFromStepPath'
	import { base } from '$app/paths'
	import FlowPanelChrome from '../common/FlowPanelChrome.svelte'

	interface Props {
		/** Graph node id of the selected step, of the form `subflow:<step>[:<step>...]:<leaf>`. */
		selectedId: string
		/** Called once the subflow was deployed from the drawer, to refresh the inlined graph. */
		onSubflowUpdated?: () => void
	}

	let { selectedId, onSubflowUpdated }: Props = $props()

	const { flowStore, flowEditorDrawer, opWorkspace } =
		getContext<FlowEditorContext>('FlowEditorContext')
	let opWs = $derived(opWorkspace?.() ?? $workspaceStore)

	let leafId = $derived(parseExpandedSubflowId(selectedId)?.leaf ?? selectedId)

	// Modules of the subflows crossed so far: the panel stays mounted while the user clicks
	// through the expansion, so without this every click refetches the whole chain. Deploying
	// a subflow bumps the generation, which drops what the cache holds.
	let modulesCache = new Map<string, FlowModule[]>()
	let cachedGeneration = 0
	let generation = $state(0)
	// `undefined` while resolving. A resolution is only applied when it is still the one the
	// current selection asked for, so a slower deep chain can't overwrite a newer selection.
	let loaded = $state<{ step?: ResolvedExpandedSubflowStep; error?: Error } | undefined>(undefined)
	let latestRequest = 0

	$effect(() => {
		const [id, ws, gen] = [selectedId, opWs, generation]
		if (gen !== cachedGeneration) {
			modulesCache.clear()
			cachedGeneration = gen
		}
		const request = ++latestRequest
		loaded = undefined
		const rootModules = untrack(() => $state.snapshot(flowStore.val.value.modules) as FlowModule[])
		resolveExpandedSubflowStep(id, rootModules, async (path) => {
			const key = `${ws}:${path}`
			let modules = modulesCache.get(key)
			if (!modules) {
				modules = (await FlowService.getFlowByPath({ workspace: ws!, path })).value.modules
				// A response from before the last deploy must not repopulate the cache it cleared.
				if (gen === cachedGeneration) {
					modulesCache.set(key, modules)
				}
			}
			return modules
		})
			.then((step) => {
				if (request === latestRequest) loaded = { step }
			})
			.catch((error) => {
				if (request === latestRequest) loaded = { error }
			})
	})

	let resolved = $derived(loaded?.step)

	function editSubflow(path: string, stepId: string | undefined) {
		$flowEditorDrawer?.openDrawer(
			path,
			() => {
				sendUserToast('Subflow has been updated')
				generation++
				onSubflowUpdated?.()
			},
			stepId
		)
	}
</script>

<div class="flex flex-col h-full bg-surface">
	<div class="flex items-center justify-between gap-2 px-4 py-2 border-b shrink-0">
		<div class="flex items-center gap-1 min-w-0 text-xs text-secondary">
			<Badge color="indigo">{leafId}</Badge>
			{#if resolved}
				<span class="shrink-0">in</span>
				{#each resolved.pathChain as path, i (i)}
					{#if i > 0}
						<ChevronRight size={12} class="shrink-0" />
					{/if}
					<span class="truncate text-primary font-medium" title={path}>{path}</span>
				{/each}
			{/if}
		</div>
		<div class="flex items-center gap-1 shrink-0">
			{#if resolved}
				{@const { containingFlowPath, module } = resolved}
				{#if $flowEditorDrawer}
					<Button
						unifiedSize="sm"
						variant="subtle"
						startIcon={{ icon: Pen }}
						on:click={() => editSubflow(containingFlowPath, module ? leafId : undefined)}
					>
						Edit
					</Button>
				{/if}
				<Button
					unifiedSize="sm"
					variant="subtle"
					title="Open the subflow in a new tab"
					startIcon={{ icon: ExternalLink }}
					iconOnly
					href={`${base}/flows/edit/${containingFlowPath}?workspace=${encodeURIComponent(
						opWs ?? ''
					)}${module ? `&selected=${encodeURIComponent(leafId)}` : ''}`}
					target="_blank"
				/>
			{/if}
			<FlowPanelChrome />
		</div>
	</div>
	<div class="min-h-0 grow overflow-auto">
		{#if loaded == undefined}
			<div class="p-4">
				<Skeleton layout={[[2], 1, [8]]} />
			</div>
		{:else if loaded.error}
			<div class="p-4 text-xs text-secondary">
				Could not load the subflow this step belongs to: {loaded.error.message}
			</div>
		{:else if resolved?.module}
			<FlowGraphViewerStep stepDetail={resolved.module} workspace={opWs} />
		{:else}
			<div class="p-4 text-xs text-secondary">
				This step is part of an expanded subflow and is not editable from this flow.
			</div>
		{/if}
	</div>
</div>
