<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import StepSettingsBar, {
		type RetryAttempts
	} from '$lib/components/flows/map/StepSettingsBar.svelte'
	import { RefreshCw } from 'lucide-svelte'
	import type { StepSettingView } from '$lib/components/flows/flowStepSettings'
	import NodeWrapper from './NodeWrapper.svelte'
	import { Minimize2 } from 'lucide-svelte'
	import type { SubflowBoundN } from '../../graphBuilder.svelte'
	import { getGraphContext } from '../../graphContext'
	import { getFlowRunStatusContext } from '../../flowRunStatus.svelte'

	interface Props {
		data: SubflowBoundN['data']
		id: string
	}

	let { data, id }: Props = $props()

	const { selectionManager } = getGraphContext()
	const flowRunStatus = getFlowRunStatusContext()

	// An expanded subflow step is drawn as these bounds, so its start carries the step's retry
	// badge, which the collapsed node would otherwise show.
	// The bounds carry no step definition, so only the retries the run recorded are shown.
	const retrySetting: StepSettingView = {
		key: 'retries',
		label: 'Retries',
		tooltip: 'Retries',
		icon: RefreshCw,
		configured: true,
		summary: { text: '', state: 'configured' }
	}
	let stepState = $derived(
		id === data.subflowId ? flowRunStatus?.getModuleState(data.subflowId) : undefined
	)
	let attempts: RetryAttempts | undefined = $derived.by(() => {
		const jobs = stepState?.attempt_jobs
		if (!jobs?.length) return undefined
		const shown = stepState?.selectedAttempt ? jobs.indexOf(stepState.selectedAttempt) : -1
		return {
			jobs,
			success: stepState?.attempt_jobs_success ?? [],
			selected: shown >= 0 ? shown : jobs.length - 1,
			moduleId: data.subflowId,
			onSelectedIteration: (detail) => data.eventHandlers.selectedIteration(detail)
		}
	})
</script>

<NodeWrapper>
	{#snippet children({ darkMode })}
		{#if attempts || stepState?.retries}
			<div class="absolute right-0 top-[calc(100%+3px)] z-10 flex flex-row text-sm">
				<StepSettingsBar settings={[retrySetting]} retries={stepState?.retries} {attempts} />
			</div>
		{/if}
		<VirtualItem
			label={data.label}
			preLabel={data.preLabel}
			selectable
			selected={selectionManager && selectionManager.isNodeSelected(id)}
			on:select={() => {
				setTimeout(() => data.eventHandlers?.select(data.id))
			}}
		/>
		<div class="z-50 absolute -top-4 right-11 rounded-md text-primary bg-surface">
			<button
				title="Unexpand subflow"
				class="rounded-md center-center text-primary hover:bg-surface-tertiary shadow-md p-1 duration-0"
				onclick={stopPropagation(
					preventDefault(() => {
						data.eventHandlers.minimizeSubflow(data.subflowId)
					})
				)}
			>
				<Minimize2 size={12} />
			</button>
		</div>
	{/snippet}
</NodeWrapper>
