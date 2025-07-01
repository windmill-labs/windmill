import { type Job, JobService, type RestartedFrom, type OpenFlow } from '$lib/gen'
import { workspaceStore } from '$lib/stores'
import { sendUserToast } from '$lib/utils'
import { runFlowPreview } from './flows/utils'
import { dfs } from './flows/dfs'
import { sliceModules } from './flows/flowStateUtils.svelte'
import { aiChatManager } from './copilot/chat/AIChatManager.svelte'
import { stateSnapshot } from '$lib/svelte5Utils.svelte'
import type { FlowEditorContext } from './flows/types'
import { getStepHistoryLoaderContext } from './stepHistoryLoader.svelte'
import { get } from 'svelte/store'

export class FlowPreview {
	previewMode = $state<'upTo' | 'whole'>('whole')
	jobId = $state<string | undefined>(undefined)
	job = $state<Job | undefined>(undefined)
	isRunning = $state<boolean>(false)
	lastPreviewFlow = $state<string | undefined>(undefined)

	private flowEditorContext: FlowEditorContext
	private stepHistoryLoader: ReturnType<typeof getStepHistoryLoaderContext>
	private jobProgressReset: (() => void) | undefined = $state(undefined)
	private onRunPreviewCallback: (() => void) | undefined = $state(undefined)

	constructor(flowEditorContext: FlowEditorContext) {
		this.flowEditorContext = flowEditorContext
		this.stepHistoryLoader = getStepHistoryLoaderContext()

		// Watch for job completion
		$effect(() => {
			if (this.job?.type === 'CompletedJob') {
				this.isRunning = false
			}
		})
	}

	setJobProgressReset(resetFn: () => void) {
		this.jobProgressReset = resetFn
	}

	setOnRunPreviewCallback(callback: () => void) {
		this.onRunPreviewCallback = callback
	}

	private extractFlow(previewMode: 'upTo' | 'whole'): OpenFlow {
		const previewFlow = aiChatManager.flowAiChatHelpers?.getPreviewFlow()
		if (previewMode === 'whole') {
			return previewFlow ?? this.flowEditorContext.flowStore.val
		} else {
			const flow = previewFlow ?? stateSnapshot(this.flowEditorContext.flowStore).val
			const idOrders = dfs(flow.value.modules, (x) => x.id)
			const selectedId = get(this.flowEditorContext.selectedId)
			let upToIndex = idOrders.indexOf(selectedId)

			if (upToIndex != -1) {
				flow.value.modules = sliceModules(flow.value.modules, upToIndex, idOrders)
			}
			return flow
		}
	}

	async runPreview(args: Record<string, any>, restartedFrom: RestartedFrom | undefined) {
		if (this.stepHistoryLoader?.flowJobInitial) {
			this.stepHistoryLoader?.setFlowJobInitial(false)
		}
		this.onRunPreviewCallback?.()
		try {
			this.lastPreviewFlow = JSON.stringify(this.flowEditorContext.flowStore.val)
			this.jobProgressReset?.()
			const newFlow = this.extractFlow(this.previewMode)
			this.jobId = await runFlowPreview(
				args,
				newFlow,
				get(this.flowEditorContext.pathStore),
				restartedFrom
			)
			this.isRunning = true
		} catch (e) {
			sendUserToast('Could not run preview', true, undefined, e.toString())
			this.isRunning = false
			this.jobId = undefined
		}
	}

	async cancelTest() {
		try {
			this.jobId &&
				(await JobService.cancelQueuedJob({
					workspace: get(workspaceStore) ?? '',
					id: this.jobId,
					requestBody: {}
				}))
		} catch {}
	}

	test(previewArgs: Record<string, any>) {
		this.runPreview(previewArgs, undefined)
	}

	hasFlowChanged(): boolean {
		return !!(
			this.lastPreviewFlow &&
			JSON.stringify(this.flowEditorContext.flowStore.val) !== this.lastPreviewFlow
		)
	}
}
