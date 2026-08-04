<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import { resource } from 'runed'
	import { FlowService, type FlowModule } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import FlowGraphViewerStep from '$lib/components/FlowGraphViewerStep.svelte'
	import { ChevronRight, ExternalLink, Pen } from 'lucide-svelte'
	import { sendUserToast } from '$lib/utils'
	import type { FlowEditorContext } from '../types'
	import { resolveExpandedSubflowStep } from '../expandedSubflowStep'
	import { parseExpandedSubflowId } from '$lib/components/restartFromStepPath'

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

	let step = resource([() => selectedId, () => opWs], async ([id, ws]) =>
		resolveExpandedSubflowStep(
			id,
			untrack(() => $state.snapshot(flowStore.val.value.modules) as FlowModule[]),
			async (path) => (await FlowService.getFlowByPath({ workspace: ws!, path })).value.modules
		)
	)

	// While a new selection resolves, `step.current` still holds the previous step: hide the
	// breadcrumb and the actions rather than pointing them at the step the user just left.
	let resolved = $derived(step.loading ? undefined : step.current)

	function editSubflow(path: string, stepId: string | undefined) {
		$flowEditorDrawer?.openDrawer(
			path,
			() => {
				sendUserToast('Subflow has been updated')
				step.refetch()
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
		{#if resolved}
			{@const { containingFlowPath, module } = resolved}
			<div class="flex items-center gap-1 shrink-0">
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
					href={`/flows/edit/${containingFlowPath}?workspace=${opWs}${
						module ? `&selected=${encodeURIComponent(leafId)}` : ''
					}`}
					target="_blank"
				/>
			</div>
		{/if}
	</div>
	<div class="min-h-0 grow overflow-auto">
		{#if step.loading}
			<div class="p-4">
				<Skeleton layout={[[2], 1, [8]]} />
			</div>
		{:else if step.error}
			<div class="p-4 text-xs text-secondary">
				Could not load the subflow this step belongs to: {step.error.message}
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
