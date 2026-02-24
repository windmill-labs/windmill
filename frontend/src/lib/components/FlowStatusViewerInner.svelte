<script lang="ts">
	import FlowStatusViewerInner from './FlowStatusViewerInner.svelte'

	import {
		type FlowStatusModule,
		type Job,
		JobService,
		type FlowStatus,
		type FlowModuleValue,
		type FlowModule,
		ResourceService,
		type CompletedJob
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import FlowJobResult from './FlowJobResult.svelte'
	import DisplayResult from './DisplayResult.svelte'

	import { getContext, setContext, tick, untrack } from 'svelte'
	import { onDestroy } from 'svelte'
	import { Badge, Button, Skeleton, Tab } from './common'
	import Tabs from './common/tabs/Tabs.svelte'
	import { type DurationStatus, type FlowStatusViewerContext, type GraphModuleState } from './graph'
	import ModuleStatus from './ModuleStatus.svelte'
	import { clone, isScriptPreview, msToSec, readFieldsRecursively, truncateRev } from '$lib/utils'
	import JobArgs from './JobArgs.svelte'
	import { ChevronDown, ExternalLink, Hourglass } from 'lucide-svelte'
	import { deepEqual } from 'fast-equals'
	import FlowTimeline from './FlowTimeline.svelte'
	import { dfs } from './flows/dfs'
	import { dfs as dfsPreviousResults } from '$lib/components/flows/previousResults'
	import Alert from './common/alert/Alert.svelte'
	import FlowGraphViewerStep from './FlowGraphViewerStep.svelte'
	import FlowGraphV2 from './graph/FlowGraphV2.svelte'
	import { buildPrefix } from './graph/graphBuilder.svelte'
	import { parseInputArgsAssets } from './assets/lib'
	import FlowLogViewerWrapper from './FlowLogViewerWrapper.svelte'
	import type { FlowGraphAssetContext } from './flows/types'
	import { createState } from '$lib/svelte5Utils.svelte'
	import JobLoader from './JobLoader.svelte'
	import {
		AI_TOOL_CALL_PREFIX,
		AI_TOOL_MESSAGE_PREFIX,
		AI_MCP_TOOL_CALL_PREFIX,
		AI_WEBSEARCH_PREFIX,
		getToolCallId
	} from './graph/renderers/nodes/AIToolNode.svelte'
	import JobAssetsViewer from './assets/JobAssetsViewer.svelte'
	import McpToolCallDetails from './McpToolCallDetails.svelte'
	import JobOtelTraces from './JobOtelTraces.svelte'
	import JobDetailHeader from './runs/JobDetailHeader.svelte'
	import LogViewer from './LogViewer.svelte'
	import { SelectionManager } from './graph/selectionUtils.svelte'
	import { useThrottle } from 'runed'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import { getActiveReplay } from './recording/flowRecording.svelte'

	let {
		flowState: flowStateStore,
		retryStatus,
		suspendStatus,
		hideDownloadInGraph,
		hideTimeline,
		hideNodeDefinition,
		hideDownloadLogs
	} = getContext<FlowStatusViewerContext>('FlowStatusViewer')

	interface Props {
		jobId: string
		initialJob?: Job | undefined
		workspaceId?: string | undefined
		flowJobIds?:
			| {
					moduleId: string
					flowJobs: string[]
					flowJobsSuccess: (boolean | undefined)[]
					flowJobsDuration:
						| {
								started_at?: (string | undefined)[]
								duration_ms?: (number | undefined)[]
						  }
						| undefined
					length: number
					branchall?: boolean
			  }
			| undefined
		//only useful when forloops are optimized and the job doesn't contain the mod id anymore
		innerModule?: FlowModuleValue | undefined
		render?: boolean
		selectedNode?: string | undefined
		globalModuleStates: Record<string, GraphModuleState>[]
		globalDurationStatuses?: Record<string, DurationStatus>[]
		isSelectedBranch?: boolean
		isSubflow?: boolean
		reducedPolling?: boolean
		wideResults?: boolean
		topModuleStates?: Record<string, GraphModuleState>
		workspace?: string | undefined
		prefix?: string | undefined
		subflowParentsGlobalModuleStates?: Record<string, GraphModuleState>[]
		subflowParentsDurationStatuses?: Record<string, DurationStatus>[]
		isForloopSelected?: boolean
		updateRecursiveRefreshFn?: (jobId: string, updateFn: (clear, root) => Promise<void>) => void
		refreshGlobal: (moduleId: string, clear: boolean, root: string) => Promise<void>
		updateGlobalRefresh: (moduleId: string, updateFn: (clear, root) => Promise<void>) => void
		job?: (Job & { result_stream?: string }) | undefined
		rightColumnSelect?: 'timeline' | 'node_status' | 'node_definition' | 'user_states' | 'tracing'
		localModuleStates?: Record<string, GraphModuleState>
		localDurationStatuses?: Record<string, DurationStatus>
		onResultStreamUpdate?: ({
			jobId,
			result_stream
		}: {
			jobId: string
			result_stream?: string
		}) => void
		customUi?: {
			tagLabel?: string | undefined
		}
		graphTabOpen: boolean
		isNodeSelected: boolean
		loadExtraLogs?: (logs: string) => void
		onStart?: () => void
		onJobsLoaded?: ({ job, force }: { job: Job; force: boolean }) => void
		onDone?: ({ job }: { job: CompletedJob }) => void
		toolCallStore?: {
			getStoredToolCallJob: (storeKey: string) => Job | undefined
			setStoredToolCallJob: (storeKey: string, job: Job) => void
			getLocalToolCallJobs: (prefix: string) => Record<number, Job>
			isToolCallToBeLoaded: (storeKey: string) => boolean
			addToolCallToLoad: (storeKey: string) => void
		}
		showLogsWithResult?: boolean
		showJobDetailHeader?: boolean
	}

	let {
		jobId,
		initialJob = undefined,
		workspaceId = undefined,
		flowJobIds = undefined,
		innerModule = undefined,
		render = true,
		selectedNode = $bindable(undefined),
		globalModuleStates,
		globalDurationStatuses = [],
		updateRecursiveRefreshFn = undefined,
		isSelectedBranch = true,
		isSubflow = false,
		reducedPolling = false,
		wideResults = false,
		workspace = $workspaceStore,
		prefix = undefined,
		topModuleStates = undefined,
		refreshGlobal,
		updateGlobalRefresh,
		subflowParentsGlobalModuleStates = [],
		subflowParentsDurationStatuses = [],
		isForloopSelected = false,
		job = $bindable(undefined),
		rightColumnSelect = $bindable('timeline'),
		localModuleStates = $bindable({}),
		localDurationStatuses = $bindable({}),
		customUi,
		onResultStreamUpdate = undefined,
		graphTabOpen,
		isNodeSelected,
		loadExtraLogs = undefined,
		onStart = undefined,
		onJobsLoaded = undefined,
		onDone = undefined,
		toolCallStore,
		showLogsWithResult = false,
		showJobDetailHeader = false
	}: Props = $props()

	let getTopModuleStates = $derived(topModuleStates ?? localModuleStates)

	// Cache replay check to avoid repeated function calls in the template
	let isReplay = $derived(!!getActiveReplay())

	let resultStreams: Record<string, string | undefined> = $state({})

	if (onResultStreamUpdate == undefined) {
		onResultStreamUpdate = ({
			jobId,
			result_stream
		}: {
			jobId: string
			result_stream?: string
		}) => {
			resultStreams[jobId] = result_stream
		}
	}

	let recursiveRefresh: Record<string, (clear, root) => Promise<void>> = $state({})
	let updateRecursiveRefreshInner = (
		childJobId: string,
		updateFn: (clear, root) => Promise<void>
	) => {
		if (childJobId) {
			recursiveRefresh[childJobId] = updateFn
		}
	}

	// Add support for the input args assets shown as an asset node
	const _flowGraphAssetsCtx = getContext<FlowGraphAssetContext | undefined>('FlowGraphAssetContext')
	let extendedFlowGraphAssetsCtx = $state(createState(clone(_flowGraphAssetsCtx)))
	setContext('FlowGraphAssetContext', extendedFlowGraphAssetsCtx)
	$effect(() => {
		readFieldsRecursively(_flowGraphAssetsCtx)
		job?.args
		untrack(() => {
			if (extendedFlowGraphAssetsCtx && _flowGraphAssetsCtx) {
				const inputAssets = parseInputArgsAssets(job?.args ?? {})
				const resourceMetadataCache = _flowGraphAssetsCtx.val.resourceMetadataCache
				for (const asset of inputAssets) {
					if (asset.kind === 'resource' && !(asset.path in resourceMetadataCache)) {
						resourceMetadataCache[asset.path] = undefined
						if (!isReplay) {
							ResourceService.getResource({
								workspace: workspace ?? $workspaceStore!,
								path: asset.path
							})
								.then((r) => (resourceMetadataCache[asset.path] = r))
								.catch((err) => {})
						}
					}
				}
				extendedFlowGraphAssetsCtx.val = clone(_flowGraphAssetsCtx?.val)
				extendedFlowGraphAssetsCtx.val.additionalAssetsMap['Input'] = inputAssets
			}
		})
	})

	let jobResults: any[] = $state(
		flowJobIds?.flowJobs?.map((x, id) => `iter #${id + 1} not loaded by frontend yet`) ?? []
	)

	let retry_selected = $state('')
	let timeout: number | undefined = undefined

	let expandedSubflows: Record<string, FlowModule[]> = $state({})

	let selectionManager = new SelectionManager()

	function onFlowModuleId() {
		let modId = flowJobIds?.moduleId
		if (modId) {
			let prefixedId = buildSubflowKey(modId, prefix)
			updateGlobalRefresh(prefixedId, async (clear, root) => {
				// console.debug('updateGlobalRefreshInner refresh', prefixedId, clear, root)
				await refresh(clear, root) // refresh(true, loopJob)
			})
		}
	}

	function updateModuleStates(
		moduleState: Record<string, GraphModuleState>,
		key: string,
		newValue: GraphModuleState,
		keepType: boolean | undefined
	) {
		if (
			newValue.selectedForloop != undefined &&
			moduleState[key]?.selectedForloop != undefined &&
			newValue.selectedForloop != moduleState[key].selectedForloop
		) {
			let newState = { ...moduleState[key] }
			if (
				newValue.type == 'InProgress' &&
				moduleState[key]?.type != 'InProgress' &&
				!(
					keepType &&
					(moduleState[key]?.type === 'Success' || moduleState[key]?.type === 'Failure')
				)
			) {
				newState.type = 'InProgress'
			} else if (['Success', 'Failure'].includes(newValue.type)) {
				newState.type = newValue.type
			}

			if (
				moduleState[key]?.job_id != newValue.job_id ||
				!deepEqual(moduleState[key]?.args, newValue.args) ||
				!deepEqual(moduleState[key]?.result, newValue.result)
			) {
				newState.args = newValue.args
				newState.result = newValue.result
				newState.job_id = newValue.job_id
			}
			moduleState[key] = newState
			return
		}

		if (moduleState[key]?.selectedForLoopSetManually) {
			if (
				newValue.selectedForloop != undefined &&
				moduleState[key]?.selectedForloop != newValue.selectedForloop
			) {
				return moduleState
			} else {
				newValue.selectedForLoopSetManually = true
				newValue.selectedForloopIndex = moduleState[key]?.selectedForloopIndex
				newValue.selectedForloop = moduleState[key]?.selectedForloop
			}
		} else if (moduleState[key]?.selectedForloopIndex != undefined) {
			newValue.selectedForloopIndex = moduleState[key]?.selectedForloopIndex
			newValue.selectedForloop = moduleState[key]?.selectedForloop
		}

		if (keepType && (moduleState[key]?.type == 'Success' || moduleState[key]?.type == 'Failure')) {
			newValue.type = moduleState[key].type
		}
		if (!deepEqual(moduleState[key], newValue)) {
			// console.debug('updateModuleStates 2', key, $state.snapshot(moduleState))
			moduleState[key] = newValue
		}
	}

	function buildSubflowKey(key: string, prefix: string | undefined) {
		return prefix ? 'subflow:' + prefix + key : key
	}

	async function refresh(clearLoop: boolean, rootJob: string | undefined) {
		console.debug('refresh', clearLoop, rootJob)
		let modId = flowJobIds?.moduleId
		let topModuleStates = getTopModuleStates

		if (clearLoop) {
			if (!rootJob) {
				if (modId && topModuleStates) {
					let prefixedId = buildSubflowKey(modId, prefix)
					delete topModuleStates[prefixedId] // TODO: this is not working
				}

				if (subflowParentsGlobalModuleStates.length > 0) {
					let subflowModuleStates =
						subflowParentsGlobalModuleStates?.[subflowParentsGlobalModuleStates?.length - 1]

					for (let mod of innerModules ?? []) {
						if (mod.id) {
							delete subflowModuleStates[buildSubflowKey(mod.id, prefix)]
						}
					}
				} else {
					for (let mod of innerModules ?? []) {
						if (mod.id && topModuleStates) {
							let prefixedId = buildSubflowKey(mod.id, prefix)
							delete topModuleStates[prefixedId]
						}
					}
				}
			}
		} else {
			let state = modId ? topModuleStates?.[buildSubflowKey(modId, prefix)] : undefined
			let loopjob = state?.selectedForloop
			let njob = flowJobIds && modId && loopjob ? storedListJobs?.[loopjob] : job

			if (njob) {
				onJobsLoaded?.({ job: njob, force: true })
			}
		}

		let callRec = async (rec: (clear, root) => Promise<void>) => {
			await tick()
			await rec(clearLoop, undefined)
		}
		if (rootJob) {
			let rec = recursiveRefresh[rootJob]
			if (rec) {
				await callRec(rec)
				// console.debug('refresh recursive 1', rec)
			} else {
				// console.debug('refresh recursive no rec', rootJob)
			}
		} else {
			for (let rec of Object.values(recursiveRefresh)) {
				await callRec(rec)
				// console.debug('refresh recursive 2', rec)
			}
		}
	}

	function updateRecursiveRefresh(jobId: string) {
		if (jobId) {
			updateRecursiveRefreshFn?.(jobId, async (clear, root) => {
				if (globalModuleStates.length > 0 || isSubflow) {
					await refresh(clear, root)
				}
			})
		}
	}

	function setModuleState(
		key: string,
		value: Partial<GraphModuleState>,
		force?: boolean,
		keepType?: boolean
	) {
		let newValue = { ...(localModuleStates[key] ?? {}), ...value }
		if (!deepEqual(localModuleStates[key], value) || force) {
			// console.debug('setModuleState', key, force, keepType, $state.snapshot(value))
			;[localModuleStates, ...globalModuleStates].forEach((s) => {
				updateModuleStates(s, key, newValue, keepType)
			})
			if (prefix) {
				let prefixedId = buildSubflowKey(key, prefix)
				subflowParentsGlobalModuleStates.forEach((s) => {
					updateModuleStates(s, prefixedId, newValue, keepType)
				})
			}
		}
	}

	function setDurationStatusByJob(
		key: string,
		id: string,
		value: DurationStatus['byJob'][string],
		overwrite: boolean = false
	) {
		if (!deepEqual(localDurationStatuses[key]?.byJob?.[id], value)) {
			if (localDurationStatuses[key]?.byJob == undefined) {
				localDurationStatuses[key] = { byJob: {} }
			}
			localDurationStatuses[key].byJob = {
				[id]: value,
				...(overwrite ? {} : (localDurationStatuses[key].byJob ?? {}))
			}

			globalDurationStatuses.forEach((s) => {
				s[key].byJob = { [id]: value, ...(overwrite ? {} : (s[key].byJob ?? {})) }
			})
			if (prefix) {
				subflowParentsDurationStatuses.forEach((s) => {
					s[buildSubflowKey(key, prefix)].byJob = {
						[id]: value,
						...(overwrite ? {} : (s[buildSubflowKey(key, prefix)].byJob ?? {}))
					}
				})
			}
		}
	}

	function initializeByJob(modId: string) {
		if (localDurationStatuses[modId] == undefined) {
			localDurationStatuses[modId] = { byJob: {} }
		}
		globalDurationStatuses.forEach((x) => {
			if (x[modId] == undefined) {
				x[modId] = { byJob: {} }
			}
		})
		if (prefix) {
			subflowParentsDurationStatuses.forEach((x) => {
				let key = buildSubflowKey(modId, prefix)
				if (x[key] == undefined) {
					x[key] = { byJob: {} }
				}
			})
		}
	}

	let innerModules = $state(undefined) as FlowStatusModule[] | undefined
	function updateStatus(status: FlowStatus) {
		innerModules =
			status?.modules?.concat(
				status.failure_module.type != 'WaitingForPriorSteps' ? status.failure_module : []
			) ?? []
		if (status.preprocessor_module) {
			innerModules.unshift(status.preprocessor_module)
		}
		updateInnerModules()

		let count = status.retry?.fail_count
		if (count) {
			retryStatus.val[jobId ?? ''] = count
		} else if (retryStatus.val[jobId ?? ''] != undefined) {
			delete retryStatus.val[jobId ?? '']
		}
		let jobStatus = job?.flow_status?.modules?.[job?.flow_status.step]

		if (jobStatus && jobStatus.count != undefined) {
			suspendStatus.val[jobId ?? ''] = { nb: jobStatus.count, job: job! }
		} else if (suspendStatus.val[jobId ?? ''] != undefined) {
			delete suspendStatus.val[jobId ?? '']
		}
	}

	function updateDurationStatuses(
		key: string,
		durationStatuses: Record<string, DurationStatus['byJob'][string]>
	) {
		if (localDurationStatuses[key] == undefined) {
			localDurationStatuses[key] = { byJob: {} }
		}
		localDurationStatuses[key].byJob = durationStatuses
		globalDurationStatuses.forEach((s) => {
			if (s[key] == undefined) {
				s[key] = { byJob: {} }
			}
			s[key].byJob = durationStatuses
		})
		if (prefix) {
			subflowParentsDurationStatuses.forEach((s) => {
				if (s[key] == undefined) {
					s[key] = { byJob: {} }
				}
				s[key].byJob = durationStatuses
			})
		}
	}

	let jobMissingStartedAt: Record<string, number | 'P'> = {}

	let setSelectedLoopSwitch = useThrottle(async (lastStarted: string, mod: FlowStatusModule) => {
		let position = mod.flow_jobs?.indexOf(lastStarted)
		if (!position) return
		for (const flow_job of mod.flow_jobs ?? []) {
			if (flow_job === lastStarted) {
				break
			} else if (flow_job !== lastStarted && suspendStatus.val[flow_job]) {
				delete suspendStatus.val[flow_job]
			}
		}
		setIteration(position, lastStarted, false, mod.id ?? '', true)
	}, 2000)

	function updateInnerModules() {
		if (localModuleStates) {
			innerModules?.forEach((mod, i) => {
				if (mod.type === 'WaitingForEvents' && innerModules?.[i - 1]?.type === 'Success') {
					setModuleState(mod.id ?? '', {
						type: mod.type,
						args: job?.args,
						tag: job?.tag,
						script_hash: job?.script_hash
					})
				} else if (
					mod.type === 'WaitingForExecutor' &&
					localModuleStates[mod.id ?? '']?.scheduled_for == undefined
				) {
					if (!isReplay) {
						JobService.getJob({
							workspace: workspaceId ?? $workspaceStore ?? '',
							id: mod.job ?? '',
							noLogs: true,
							noCode: true
						})
							.then((job) => {
								const newState = {
									type: mod.type,
									scheduled_for: job?.['scheduled_for'],
									job_id: job?.id,
									parent_module: mod['parent_module'],
									args: job?.args,
									tag: job?.tag,
									script_hash: job?.script_hash
								}

								setModuleState(mod.id ?? '', newState)
							})
							.catch((e) => {
								console.error(`Could not load inner module for job ${mod.job}`, e)
							})
					}
				} else if (
					(mod.flow_jobs || mod.branch_chosen) &&
					(mod.type == 'Success' || mod.type == 'Failure') &&
					!['Success', 'Failure'].includes(localModuleStates?.[mod.id ?? '']?.type)
				) {
					let branchChosen = mod.branch_chosen
						? {
								branchChosen:
									mod.branch_chosen.type == 'default' ? 0 : (mod.branch_chosen.branch ?? 0) + 1
							}
						: {}
					// console.debug('updateInnerModules', mod.id, mod.type, branchChosen)
					setModuleState(
						mod.id ?? '',
						{
							type: mod.type,
							...branchChosen
						},
						true
					)
				} else if (isForloopSelected) {
					setModuleState(mod.id ?? '', {}, true)
				}

				if (mod.flow_jobs_success || mod.flow_jobs_duration || mod.flow_jobs) {
					setModuleState(mod.id ?? '', {
						flow_jobs_success: mod.flow_jobs_success,
						flow_jobs_duration: mod.flow_jobs_duration,
						flow_jobs: mod.flow_jobs,
						iteration_total: mod.iterator?.itered_len ?? mod.flow_jobs?.length
					})
				}

				if (mod.flow_jobs_duration && mod.flow_jobs) {
					let key = buildSubflowKey(mod.id ?? '', prefix)
					let durationStatuses = Object.fromEntries(
						mod.flow_jobs.map((flowJobId, idx) => {
							let started_at_str = mod.flow_jobs_duration?.started_at?.[idx]
							let started_at = started_at_str ? new Date(started_at_str).getTime() : undefined
							let duration_ms = mod.flow_jobs_duration?.duration_ms?.[idx]
							if (started_at == undefined) {
								let missingStartedAt = jobMissingStartedAt[flowJobId]
								if (missingStartedAt != 'P') {
									started_at = missingStartedAt
								}
							} else {
								delete jobMissingStartedAt[flowJobId]
							}
							return [
								flowJobId,
								{
									created_at: started_at,
									started_at: started_at,
									duration_ms: duration_ms
								}
							]
						})
					)
					let missingStartedAtIds = Object.keys(durationStatuses)
						.filter(
							(id) => durationStatuses[id].created_at == undefined && jobMissingStartedAt[id] != 'P'
						)
						.slice(0, 100)

					updateDurationStatuses(key, durationStatuses)

					if (missingStartedAtIds.length > 0) {
						// Mark as pending to prevent duplicate fetches
						missingStartedAtIds.forEach((id) => {
							jobMissingStartedAt[id] = 'P'
						})
						if (!isReplay) {
							JobService.getStartedAtByIds({
								workspace: workspaceId ?? $workspaceStore ?? '',
								requestBody: missingStartedAtIds
							})
								.then((jobs) => {
									let lastStarted: string | undefined = undefined
									let anySet = false
									let nDurationStatuses = localDurationStatuses[key]?.byJob
									missingStartedAtIds.forEach((id, idx) => {
										const startedAt = jobs[idx]
										const time = startedAt ? new Date(startedAt).getTime() : undefined
										if (time) {
											jobMissingStartedAt[id] = time
										} else {
											delete jobMissingStartedAt[id]
										}
										if (nDurationStatuses && time) {
											if (!nDurationStatuses[id]?.duration_ms) {
												anySet = true
												lastStarted = id
												nDurationStatuses[id] = {
													created_at: time,
													started_at: time
												}
											}
										}
									})
									if (anySet) {
										updateDurationStatuses(key, nDurationStatuses)
										if (lastStarted) setSelectedLoopSwitch(lastStarted, mod)
									}
								})
								.catch((e) => {
									console.error(
										`Could not load inner module duration status for job ${mod.job}`,
										e
									)
								})
						}
					} else {
						setIteration(0, mod.flow_jobs?.[0] ?? '', false, mod.id ?? '', true)
					}
				}

				if (mod.agent_actions && mod.id) {
					setModuleState(mod.id, {
						agent_actions: mod.agent_actions
					})
					mod.agent_actions.forEach((action, idx) => {
						if (mod.id) {
							if (action.type == 'tool_call') {
								const toolCallId = getToolCallId(idx, mod.id, action.module_id)
								const success = mod.agent_actions_success?.[idx]
								setModuleState(toolCallId, {
									job_id: action.job_id,
									type: success != undefined ? (success ? 'Success' : 'Failure') : 'InProgress'
								})
							} else if (action.type == 'mcp_tool_call') {
								const mcpToolCallId = AI_MCP_TOOL_CALL_PREFIX + '-' + mod.id + '-' + idx
								const success = mod.agent_actions_success?.[idx]
								setModuleState(mcpToolCallId, {
									type: success != undefined ? (success ? 'Success' : 'Failure') : 'InProgress'
								})
							} else if (action.type == 'web_search') {
								const websearchId = AI_WEBSEARCH_PREFIX + '-' + mod.id + '-' + idx
								setModuleState(websearchId, {
									type: 'Success'
								})
							} else if (action.type == 'message') {
								const toolCallId = getToolCallId(idx, mod.id)
								setModuleState(toolCallId, {
									type: 'Success'
								})
							}
						}
					})
				}
			})
		}
	}

	let debounceJobId: string | undefined = undefined
	let lastRefreshed: Date | undefined = undefined
	function debounceLoadJobInProgress() {
		const pollingRate = reducedPolling ? 5000 : 1000
		if (
			lastRefreshed &&
			new Date().getTime() - lastRefreshed.getTime() < pollingRate &&
			debounceJobId == jobId
		) {
			timeout && clearTimeout(timeout)
		}
		timeout = setTimeout(() => {
			loadJobInProgress()
			lastRefreshed = new Date()
			debounceJobId = jobId
			timeout = undefined
		}, pollingRate)
	}

	let notAnonynmous = $state(false)
	let started = false
	let jobLoader: JobLoader | undefined = undefined

	function setJob(newJob: Job, force: boolean) {
		if (!deepEqual(job, newJob) || isForloopSelected || force || innerModules == undefined) {
			if (initialJob) {
				// keep raw_flow/raw_code from initial job if they exist
				job = {
					...newJob,
					raw_flow: initialJob.raw_flow ?? newJob.raw_flow,
					raw_code: initialJob.raw_code ?? newJob.raw_code
				}
			} else {
				job = newJob
			}
			job?.flow_status && updateStatus(job?.flow_status)
			onJobsLoaded?.({ job, force: false })
			notAnonynmous = false
			if (job?.type == 'CompletedJob' && !destroyed) {
				onDone?.({ job })
			}
		}
	}
	async function loadJobInProgress() {
		if (!started) {
			started = true
			onStart?.()
		}
		if (jobId != '00000000-0000-0000-0000-000000000000') {
			try {
				if (
					jobId == initialJob?.id &&
					initialJob?.id != undefined &&
					initialJob?.type === 'CompletedJob'
				) {
					setJob(initialJob, false)
				} else {
					jobLoader?.watchJob(jobId, {
						change(newJob) {
							setJob(newJob, true)
						},
						resultStreamUpdate({ id, result_stream }: { id: string; result_stream?: string }) {
							onResultStreamUpdate?.({ jobId: id, result_stream })
						},
						loadExtraLogs({ id, logs }: { id: string; logs: string }) {
							if (id == jobId && job) {
								job.logs = logs
							}
							if (loadExtraLogs) {
								loadExtraLogs(logs)
							}
						}
					})
				}
			} catch (e) {
				if (
					e?.body?.includes('As a non logged in user, you can only see jobs ran by anonymous users')
				) {
					notAnonynmous = true
				} else {
					console.error(e)
				}
			}
		}
	}

	let destroyed = false

	updateRecursiveRefresh(jobId)

	async function updateJobId() {
		if (jobId !== job?.id || innerModules == undefined) {
			localModuleStates = {}
			flowTimeline?.reset()
			timeout && clearTimeout(timeout)
			innerModules = undefined
			if (flowJobIds) {
				let modId = flowJobIds?.moduleId ?? ''

				if (localDurationStatuses[modId] == undefined) {
					localDurationStatuses[modId] = { byJob: {} }
				}
				let prefixed = buildSubflowKey(modId, prefix)
				globalDurationStatuses.forEach((x) => {
					if (x[prefixed] == undefined) {
						x[prefixed] = { byJob: {} }
					}
				})
			} else {
				recursiveRefresh = {}
				localDurationStatuses = {}
				updateRecursiveRefresh(jobId)
			}
			await loadJobInProgress()
		}
	}

	let forloop_selected = $derived(
		getTopModuleStates?.[buildSubflowKey(flowJobIds?.moduleId ?? '', prefix)]?.selectedForloop
	)

	onDestroy(() => {
		destroyed = true
		timeout && clearTimeout(timeout)
		// sub?.()
	})

	function isSuccess(arg: any): boolean | undefined {
		if (arg == undefined) {
			return undefined
		} else {
			return arg == true
		}
	}

	function onJobsLoadedInner(mod: FlowStatusModule, job: Job, force?: boolean): void {
		let id = mod.id
		if (id && ((mod.flow_jobs ?? []).length == 0 || force)) {
			// console.debug('onJobsLoadedInner', id, job.id, force)
			if (flowStateStore) {
				flowStateStore[buildSubflowKey(id, prefix)] = {
					...(flowStateStore?.[buildSubflowKey(id, prefix)] ?? {}),
					previewResult: job['result'],
					previewArgs: job.args,
					previewJobId: job.id,
					previewSuccess: job['success']
				}
			}

			initializeByJob(id)
			let started_at = job.started_at ? new Date(job.started_at).getTime() : undefined
			if (job.type == 'QueuedJob') {
				setModuleState(
					id,
					{
						type: 'InProgress',
						job_id: job.id,
						logs: job.logs,
						args: job.args,
						tag: job.tag,
						started_at,
						parent_module: mod['parent_module'],
						script_hash: job.script_hash
					},
					force
				)
				setDurationStatusByJob(
					id,
					job.id,
					{
						created_at: job.created_at ? new Date(job.created_at).getTime() : undefined,
						started_at
					},
					true
				)
			} else {
				const parent_module = mod['parent_module']

				// Delete existing failure node attached to the same parent module
				removeFailureNode(id, parent_module)

				setModuleState(
					id,
					{
						args: job.args,
						type: job['success'] ? 'Success' : 'Failure',
						logs: job.logs,
						result: job['result'],
						job_id: job.id,
						tag: job.tag,
						parent_module,
						duration_ms: job['duration_ms'],
						started_at: started_at,
						flow_jobs: mod.flow_jobs,
						flow_jobs_success: mod.flow_jobs_success,
						flow_jobs_duration: mod.flow_jobs_duration,
						iteration_total: mod.iterator?.itered_len,
						retries: mod?.failed_retries?.length,
						skipped: mod.skipped,
						agent_actions: mod.agent_actions,
						script_hash: job.script_hash
						// retries: flowStateStore?.raw_flow
					},
					force
				)

				setDurationStatusByJob(
					id,
					job.id,
					{
						created_at: job.created_at ? new Date(job.created_at).getTime() : undefined,
						started_at,
						duration_ms: job['duration_ms']
					},
					true
				)
			}
		}
	}

	async function setIteration(
		j: number,
		id: string,
		clicked: boolean,
		modId: string,
		isForloop: boolean
	) {
		if (modId) {
			let prefixedId = buildSubflowKey(modId, prefix)

			let topModuleStates = getTopModuleStates
			let state = topModuleStates?.[prefixedId]

			if (clicked && state?.selectedForloop) {
				await refreshGlobal?.(prefixedId, true, state.selectedForloop)
			}
			let manualOnce = state?.selectedForLoopSetManually
			if (
				clicked ||
				(!manualOnce &&
					(state == undefined || !isForloop || j >= (state.selectedForloopIndex ?? -1)))
			) {
				let setManually = clicked || manualOnce

				let newState: Partial<GraphModuleState> = {
					...(state ?? {}),
					selectedForloop: id,
					selectedForloopIndex: j,
					selectedForLoopSetManually: setManually
				}

				const selectedNotEqual =
					id != state?.selectedForloop ||
					j != state?.selectedForloopIndex ||
					setManually != state?.selectedForLoopSetManually
				// console.debug('setIteration', selectedNotEqual, state, topModuleStates)
				if (selectedNotEqual) {
					if (topModuleStates) {
						topModuleStates[prefixedId] = {
							type: 'WaitingForPriorSteps',
							args: {},
							...newState
						}
					}
				}
			}

			if (clicked) {
				await refreshGlobal?.(prefixedId, false, id)
			}
		}
	}

	function innerJobLoaded(jobLoaded: Job, j: number, clicked: boolean, force: boolean) {
		let modId = flowJobIds?.moduleId

		if (modId) {
			setIteration(
				j,
				jobLoaded.id,
				clicked,
				modId,
				innerModule?.type == 'forloopflow' || innerModule?.type == 'whileloopflow'
			)

			let prefixedId = buildSubflowKey(modId, prefix)

			// if (flowStateStore) {
			// 	flowStateStore[modId] = {
			// 		...((flowStateStore[modId] as object) ?? {}),
			// 		previewResult: jobLoaded.args
			// 	}
			// }
			if (flowStateStore && flowStateStore[prefixedId] == undefined) {
				flowStateStore[prefixedId] = {}
			}
			if (flowStateStore) {
				if (
					!flowStateStore?.[prefixedId]?.previewResult ||
					!Array.isArray(flowStateStore[prefixedId]?.previewResult)
				) {
					flowStateStore[prefixedId].previewResult = []
				}
				flowStateStore[prefixedId].previewArgs = jobLoaded.args
			}
			if (jobLoaded.type == 'QueuedJob') {
				jobResults[j] = 'Job in progress ...'
			} else if (jobLoaded.type == 'CompletedJob') {
				if (flowStateStore?.[prefixedId]) {
					flowStateStore[prefixedId].previewResult[j] = jobLoaded.result
				}
				jobResults[j] = jobLoaded.result
			}

			let started_at = jobLoaded.started_at ? new Date(jobLoaded.started_at).getTime() : undefined

			let created_at = jobLoaded.created_at ? new Date(jobLoaded.created_at).getTime() : undefined

			let job_id = jobLoaded.id
			initializeByJob(modId)

			let v: Partial<GraphModuleState> = {
				started_at,
				flow_jobs: flowJobIds?.flowJobs,
				flow_jobs_success: flowJobIds?.flowJobsSuccess,
				flow_jobs_duration: flowJobIds?.flowJobsDuration,
				iteration_total: flowJobIds?.length,
				duration_ms: undefined
			}

			let currentIndex = getTopModuleStates?.[prefixedId]?.selectedForloopIndex == j

			if (currentIndex) {
				v.logs = jobLoaded.logs
				v.args = jobLoaded.args
				v.job_id = jobLoaded.id
			}
			if (jobLoaded.type == 'QueuedJob') {
				if (started_at && localModuleStates[modId]?.type != 'InProgress') {
					v.type = 'InProgress'
				}
			} else if (jobLoaded.type == 'CompletedJob') {
				v.flow_jobs_results = jobResults
				if (currentIndex) {
					v.result = jobLoaded.result
				}
			}
			setModuleState(modId, v, force, true)
			if (innerModule?.type == 'branchall') {
				if (jobLoaded.type == 'QueuedJob') {
					setDurationStatusByJob(
						modId,
						job_id,
						{
							created_at,
							started_at
						},
						false
					)
				} else if (jobLoaded.type == 'CompletedJob') {
					setDurationStatusByJob(
						modId,
						job_id,
						{
							created_at,
							started_at,
							duration_ms: jobLoaded.duration_ms
						},
						false
					)
				}
			}

			if (jobLoaded.job_kind == 'script' || isScriptPreview(jobLoaded.job_kind)) {
				let id: string | undefined = undefined
				if (
					(innerModule?.type == 'forloopflow' || innerModule?.type == 'whileloopflow') &&
					innerModule.modules.length == 1
				) {
					id = innerModule?.modules?.[0]?.id
				}
				if (id) {
					onJobsLoadedInner({ id } as FlowStatusModule, jobLoaded)
				}
			}
		}
	}

	let flowTimeline: FlowTimeline | undefined = $state()

	let stepDetail: FlowModule | string | undefined = $state(undefined)

	let storedListJobs: Record<number, Job> = $state({})

	let selectedToolCall: string | undefined = $state(undefined)

	let wrapperHeight: number = $state(0)

	let retryStatusHeight: number = $state(0)

	function removeFailureNode(id: string, parent_module: any) {
		if (id?.startsWith('failure-') && parent_module) {
			;[...globalModuleStates, localModuleStates].forEach((stateMap) => {
				if (id) {
					Object.keys(stateMap).forEach((key) => {
						if (stateMap[key]?.parent_module == parent_module) {
							delete stateMap[key]
						}
					})
				}
			})
		}
	}

	export type FlowModuleForTimeline = {
		id: string
		type: FlowModuleValue['type']
	}

	function allModulesForTimeline(
		modules: FlowModule[],
		expandedSubflows: Record<string, FlowModule[]>
	): FlowModuleForTimeline[] {
		const ids = dfs(modules, (x) => ({ id: x.id, type: x.value.type }) as FlowModuleForTimeline, {
			skipToolNodes: true
		})

		function rec(
			ids: FlowModuleForTimeline[],
			prefix: string | undefined
		): FlowModuleForTimeline[] {
			return ids.concat(
				ids.flatMap(({ id }) => {
					let fms = expandedSubflows[id]
					let oid = id.split(':').pop()
					if (!oid) {
						return []
					}
					let nprefix = buildPrefix(prefix, oid)
					return fms
						? rec(
								dfs(
									fms,
									(x) => ({
										id: x.id.startsWith('subflow:') ? x.id : buildSubflowKey(x.id, nprefix),
										type: x.value.type
									}),
									{ skipToolNodes: true }
								),
								nprefix
							)
						: []
				})
			)
		}

		return rec(ids, undefined)
	}

	let subflowsSize = $state(500)

	// Splitpanes sizes for graph and details sections
	let graphPaneSize = $state(66)
	let detailsPaneSize = $state(34)

	function setParentModuleState(modId: string, state: Partial<GraphModuleState>) {
		;[localModuleStates, ...globalModuleStates].forEach((stateMap) => {
			if (stateMap[modId]) {
				stateMap[modId] = { ...stateMap[modId], ...state }
			}
		})
		if (prefix) {
			let prefixedId = buildSubflowKey(modId, prefix)
			subflowParentsGlobalModuleStates.forEach((stateMap) => {
				if (stateMap[prefixedId]) {
					stateMap[prefixedId] = { ...stateMap[prefixedId], ...state }
				}
			})
		}
	}
	async function onSelectedIteration(
		detail:
			| { id: string; index: number; manuallySet: true; moduleId: string }
			| { manuallySet: false; moduleId: string }
	) {
		let prefixedId = buildSubflowKey(detail.moduleId, prefix)
		if (detail.manuallySet) {
			let rootJobId = detail.id
			await tick()

			let previousId = getTopModuleStates?.[prefixedId]?.selectedForloop
			if (previousId) {
				await refreshGlobal?.(prefixedId, true, previousId)
			}

			setParentModuleState(detail.moduleId, {
				selectedForloop: detail.id,
				selectedForloopIndex: detail.index,
				selectedForLoopSetManually: true
			})

			await tick()

			await refreshGlobal?.(prefixedId, false, rootJobId)
		} else {
			setParentModuleState(detail.moduleId, {
				selectedForLoopSetManually: false
			})
		}
		if (selectedNode?.startsWith(AI_TOOL_CALL_PREFIX)) {
			const [, agentModuleId, toolCallIndex, _] = selectedNode.split('-')
			const parentLoopsPrefix = getParentLoopsPrefix(agentModuleId)
			toolCallStore?.addToolCallToLoad(parentLoopsPrefix + agentModuleId + '-' + toolCallIndex)
		}
	}

	$effect(() => {
		flowJobIds?.moduleId && untrack(() => onFlowModuleId())
	})
	$effect(() => {
		isForloopSelected && globalModuleStates && untrack(() => debounceLoadJobInProgress())
	})
	$effect(() => {
		jobId && untrack(() => updateJobId())
	})
	let isListJob = $derived(flowJobIds != undefined && Array.isArray(flowJobIds?.flowJobs))
	$effect(() => {
		if (isSelectedBranch) {
			let modId = flowJobIds?.moduleId
			if (modId) {
				let selectedForloop = getTopModuleStates?.[buildSubflowKey(modId, prefix)]?.selectedForloop
				untrack(() => {
					if (selectedForloop != forloop_selected) {
						forloop_selected = selectedForloop
					}
				})
			}
		}
	})
	let selected = $derived(isListJob ? 'sequence' : 'graph') as
		| 'sequence'
		| 'graph'
		| 'logs'
		| 'assets'

	let animateLogsTab = $state(false)

	let noLogs = $derived(graphTabOpen && !isNodeSelected)

	/**
	 * Returns a string like "forloopmodid1-{iter1}-forloopmodid2-{iter2}-forloopmodid3-{iter3}-"
	 * that can be used to prefix tool call store keys for nested tool calls.
	 */
	function getParentLoopsPrefix(modId: string) {
		if (job?.raw_flow) {
			const indices: string[] = []
			const parents = dfsPreviousResults(modId, { value: job?.raw_flow, summary: '' }, true)
			for (const parent of parents) {
				if (parent.value.type === 'forloopflow' || parent.value.type === 'whileloopflow') {
					const state = localModuleStates[parent.id]
					if (state?.selectedForloopIndex !== undefined) {
						indices.push(parent.id + '-' + state.selectedForloopIndex.toString())
					}
				}
			}
			indices.reverse()
			return indices.length > 0 ? indices.join('-') + '-' : ''
		}

		return ''
	}

	// Set all tabs content to the same height to prevent layout jumps
	let tabsHeight = $state({
		sequenceHeight: 0,
		logsHeight: 0,
		assetsHeight: 0,
		graphHeight: 0
	})

	let minTabHeight = $derived(
		Math.max(
			tabsHeight.sequenceHeight,
			tabsHeight.logsHeight,
			tabsHeight.assetsHeight,
			tabsHeight.graphHeight
		)
	)

	let totalEventsWaiting = $derived(
		Object.values(suspendStatus?.val ?? {}).reduce((a, b) => a + (b?.nb ?? 0), 0)
	)
</script>

<JobLoader workspaceOverride={workspaceId} {noLogs} noCode bind:this={jobLoader} />
{#if notAnonynmous}
	<Alert type="error" title="Required Auth">
		As a non logged in user, you can only see jobs ran by anonymous users like you
	</Alert>
{:else if job}
	<div class="flow-root w-full space-y-4 {wideResults ? '' : 'max-w-7xl px-4'} mx-auto">
		<!-- {#if innerModules.length > 0 && true}
			<h3 class="text-md leading-6 font-bold text-primay border-b pb-2">Flow result</h3>
		{:else}
			<div class="h-8" />
		{/if} -->
		{#if isListJob}
			{#if render}
				<div class="w-full border rounded-sm bg-surface p-1 overflow-auto max-h-[90vh]">
					<DisplayResult
						workspaceId={isReplay ? undefined : job?.workspace_id}
						jobId={isReplay ? undefined : jobId}
						result_stream={job?.result_stream}
						result={jobResults}
						language={job?.language}
					/>
				</div>
			{/if}
		{:else if render}
			<div class={'flex flex-col w-full'}>
				{#if showLogsWithResult && job}
					<!-- Side-by-side result and logs for simple jobs -->
					{#if job && showJobDetailHeader}
						<div class="mb-4">
							<JobDetailHeader {job} extraCompact />
						</div>
					{/if}
					<div class="grid grid-cols-2 grid-rows-[minmax(200px,400px)] gap-4 w-full">
						<!-- Result Column -->
						<div class="flex flex-col min-h-0">
							<h3 class="shrink-0 text-xs font-semibold text-emphasis mb-1">Result</h3>
							<div class="flex-1 min-h-0 overflow-auto rounded-md border bg-surface-tertiary p-4">
								{#if job !== undefined && (job.result_stream || (job.type == 'CompletedJob' && 'result' in job && job.result !== undefined))}
									<DisplayResult
										workspaceId={isReplay ? undefined : job?.workspace_id}
										result_stream={job.result_stream}
										jobId={isReplay ? undefined : job?.id}
										result={'result' in job ? job.result : undefined}
										language={job?.language}
										isTest={false}
									/>
								{:else if job}
									<div
										class="w-full h-full flex items-center justify-center text-secondary text-sm"
									>
										No output is available yet
									</div>
								{:else}
									<div class="w-full h-full flex items-center justify-center">
										<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
									</div>
								{/if}
							</div>
						</div>

						<!-- Logs Column -->
						<div class="flex flex-col min-h-0">
							<h3 class="shrink-0 text-xs font-semibold text-emphasis mb-1">Logs</h3>
							<div class="flex-1 min-h-0 overflow-auto rounded-md border bg-surface-tertiary">
								<LogViewer
									jobId={isReplay ? undefined : job.id}
									download={!isReplay}
									duration={job?.['duration_ms']}
									mem={job?.['mem_peak']}
									isLoading={job?.['running'] == false}
									content={job?.logs}
									tag={job?.tag}
								/>
							</div>
						</div>
					</div>
				{:else}
					<!-- Default single-column result -->
					<h3 class="text-xs font-semibold text-emphasis mb-1">Result</h3>
					<div class="flex-1 overflow-auto rounded-md border bg-surface-tertiary p-4 max-h-screen">
						{#if job !== undefined && (job.result_stream || (job.type == 'CompletedJob' && 'result' in job && job.result !== undefined))}
							<DisplayResult
								workspaceId={isReplay ? undefined : job?.workspace_id}
								result_stream={job.result_stream}
								jobId={isReplay ? undefined : job?.id}
								result={'result' in job ? job.result : undefined}
								language={job?.language}
								isTest={false}
							/>
						{:else if job}
							<div class="w-full h-full flex items-center justify-center text-secondary text-sm">
								No output is available yet
							</div>
						{:else}
							<div class="w-full h-full flex items-center justify-center">
								<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{/if}
		{#if render}
			{#if innerModules && innerModules.length > 0 && !isListJob}
				<Tabs class="mx-auto pt-2 {wideResults ? '' : 'max-w-7xl'}" bind:selected>
					<Tab value="graph" label="Graph" />
					<Tab
						value="logs"
						class={animateLogsTab
							? 'animate-pulse animate-duration-1000 bg-surface-inverse text-primary-inverse'
							: ''}
						label="Logs"
					/>

					<Tab value="sequence" label="Details" />
					<Tab value="assets" label="Assets" />
				</Tabs>
			{:else}
				<div class="h-[30px]"></div>
			{/if}
		{/if}
		<div
			class="{selected != 'sequence' ? 'hidden' : ''} max-w-7xl mx-auto"
			bind:clientHeight={tabsHeight.sequenceHeight}
			style="min-height: {minTabHeight}px"
		>
			{#if isListJob}
				<h3 class="text-md leading-6 font-bold text-primary border-b mb-4">
					Subflows ({flowJobIds?.flowJobs.length})
				</h3>
				<div class="overflow-auto max-h-1/2">
					{#each flowJobIds?.flowJobs ?? [] as loopJobId, j (loopJobId)}
						{#if render && j + subflowsSize + 1 == (flowJobIds?.flowJobs.length ?? 0)}
							<Button variant="default" on:click={() => (subflowsSize += 500)}
								>Load 500 more...</Button
							>
						{/if}
						{#if render && (j + subflowsSize + 1 > (flowJobIds?.flowJobs.length ?? 0) || forloop_selected == loopJobId)}
							<Button
								variant={forloop_selected === loopJobId ? 'contained' : 'border'}
								color={flowJobIds?.flowJobsSuccess?.[j] === false
									? 'red'
									: forloop_selected === loopJobId
										? 'dark'
										: 'light'}
								btnClasses="w-full flex justify-start"
								on:click={async () => {
									let storedJob = storedListJobs[j]
									if (!storedJob && !isReplay) {
										storedJob = await JobService.getJob({
											workspace: workspaceId ?? $workspaceStore ?? '',
											id: loopJobId,
											noLogs: true,
											noCode: true
										})
										storedListJobs[j] = storedJob
									}
									if (storedJob) {
										innerJobLoaded(storedJob, j, true, false)
									}
								}}
								endIcon={{
									icon: ChevronDown,
									classes: forloop_selected == loopJobId ? '!rotate-180' : ''
								}}
							>
								<span class="truncate font-mono">
									#{j + 1}: {loopJobId}
								</span>
							</Button>
						{/if}
						{#if forloop_selected == loopJobId || innerModule?.type == 'branchall'}
							{@const forloopIsSelected =
								forloop_selected == loopJobId ||
								(innerModule?.type != 'forloopflow' && innerModule?.type != 'whileloopflow')}
							{@const forLoopStoreKeyPrefix =
								innerModule?.type == 'forloopflow' || innerModule?.type == 'whileloopflow'
									? (flowJobIds?.moduleId ?? '') + '-' + j + '-'
									: ''}
							<!-- <LogId id={loopJobId} /> -->
							<div class="border p-6" class:hidden={forloop_selected != loopJobId}>
								<FlowStatusViewerInner
									topModuleStates={getTopModuleStates}
									{refreshGlobal}
									isSelectedBranch={isSelectedBranch && forloopIsSelected}
									updateRecursiveRefreshFn={updateRecursiveRefreshInner}
									job={storedListJobs[j]}
									initialJob={storedListJobs[j]}
									globalModuleStates={forloopIsSelected
										? [localModuleStates, ...globalModuleStates]
										: []}
									globalDurationStatuses={[localDurationStatuses, ...globalDurationStatuses]}
									{prefix}
									subflowParentsGlobalModuleStates={forloopIsSelected
										? subflowParentsGlobalModuleStates
										: []}
									{subflowParentsDurationStatuses}
									{updateGlobalRefresh}
									render={forloop_selected == loopJobId && selected == 'sequence' && render}
									isForloopSelected={forloop_selected == loopJobId &&
										(innerModule?.type == 'forloopflow' || innerModule?.type == 'whileloopflow')}
									reducedPolling={reducedPolling ||
										(!!flowJobIds?.flowJobs.length && flowJobIds?.flowJobs.length > 20)}
									{workspaceId}
									jobId={loopJobId}
									onJobsLoaded={({ job, force }) => {
										storedListJobs[j] = job
										innerJobLoaded(job, j, false, force)
									}}
									{onResultStreamUpdate}
									graphTabOpen={selected == 'graph' && graphTabOpen}
									isNodeSelected={forloop_selected == loopJobId}
									toolCallStore={{
										getStoredToolCallJob: (storeKey: string) =>
											toolCallStore?.getStoredToolCallJob(forLoopStoreKeyPrefix + storeKey),
										setStoredToolCallJob: (storeKey: string, job: Job) =>
											toolCallStore?.setStoredToolCallJob(forLoopStoreKeyPrefix + storeKey, job),
										getLocalToolCallJobs: (prefix: string) =>
											toolCallStore?.getLocalToolCallJobs(forLoopStoreKeyPrefix + prefix) ?? {},
										addToolCallToLoad: (storeKey: string) =>
											toolCallStore?.addToolCallToLoad(forLoopStoreKeyPrefix + storeKey),
										isToolCallToBeLoaded: (storeKey: string) =>
											toolCallStore?.isToolCallToBeLoaded(forLoopStoreKeyPrefix + storeKey) ?? false
									}}
									showLogsWithResult
									showJobDetailHeader
								/>
							</div>
						{/if}
					{/each}
				</div>
			{:else if innerModules && innerModules.length > 0 && ((job.raw_flow?.modules.length ?? 0) > 0 || innerModules[0]?.id == 'preprocessor')}
				{@const hasPreprocessor = innerModules[0]?.id == 'preprocessor' ? 1 : 0}
				{@const isPreprocessorOnly = hasPreprocessor && (job.raw_flow?.modules.length ?? 0) === 0}
				<!-- if the flow is preprocessor only, we should only display the first innerModule: the second one is a placeholder added when running empty flows -->
				{@const modules = isPreprocessorOnly ? [innerModules[0]] : (innerModules ?? [])}
				<ul class="w-full">
					<h3 class="text-md leading-6 font-bold text-primary border-b mb-4 py-2">
						Step-by-step
					</h3>

					{#each modules as mod, i}
						{#if render}
							<div class="line w-8 h-10"></div>
							<h3 class="text-primary mb-2 w-full">
								{#if mod.id === 'preprocessor'}
									Preprocessor module
								{:else if job?.raw_flow?.modules && i < job?.raw_flow?.modules.length + hasPreprocessor}
									Step
									<span class="font-medium text-primary">
										{i + 1 - hasPreprocessor}
									</span>
									out of
									<span class="font-medium text-primary">{job?.raw_flow?.modules.length}</span>
									{#if job.raw_flow?.modules[i]?.summary}
										: <span class="font-medium text-primary">
											{job.raw_flow?.modules[i]?.summary ?? ''}
										</span>
									{/if}
								{:else}
									Failure module
								{/if}
							</h3>
							<div class="line w-8 h-10"></div>
						{/if}
						<li class="w-full border p-6 space-y-2 bg-blue-50/50 dark:bg-frost-900/50">
							{#if render && Array.isArray(mod.failed_retries)}
								{#each mod.failed_retries as failedRetry, j}
									<Button
										variant={retry_selected === failedRetry ? 'contained' : 'border'}
										color="red"
										btnClasses="w-full flex justify-start"
										on:click={() => {
											if (retry_selected == failedRetry) {
												retry_selected = ''
											} else {
												retry_selected = failedRetry
											}
										}}
										endIcon={{
											icon: ChevronDown,
											classes: retry_selected == failedRetry ? '!rotate-180' : ''
										}}
									>
										<span class="truncate font-mono">
											# Attempt {j + 1}: {failedRetry}
										</span>
									</Button>

									<!-- <LogId id={loopJobId} /> -->
									<div class="border p-6" class:hidden={retry_selected != failedRetry}>
										<FlowStatusViewerInner
											topModuleStates={getTopModuleStates}
											{refreshGlobal}
											isSelectedBranch={isSelectedBranch && retry_selected == failedRetry}
											{updateGlobalRefresh}
											updateRecursiveRefreshFn={updateRecursiveRefreshInner}
											globalModuleStates={[localModuleStates, ...globalModuleStates]}
											globalDurationStatuses={[localDurationStatuses, ...globalDurationStatuses]}
											{prefix}
											{subflowParentsGlobalModuleStates}
											{subflowParentsDurationStatuses}
											render={failedRetry == retry_selected && render}
											{reducedPolling}
											{workspaceId}
											jobId={failedRetry}
											{onResultStreamUpdate}
											graphTabOpen={selected == 'graph' && graphTabOpen}
											isNodeSelected={retry_selected == failedRetry}
										/>
									</div>
								{/each}
							{/if}
							{#if ['InProgress', 'Success', 'Failure'].includes(mod.type)}
								{#if job.raw_flow?.modules[i]?.value.type == 'flow'}
									<FlowStatusViewerInner
										topModuleStates={getTopModuleStates}
										{isSelectedBranch}
										{refreshGlobal}
										updateRecursiveRefreshFn={updateRecursiveRefreshInner}
										globalModuleStates={[]}
										{updateGlobalRefresh}
										globalDurationStatuses={[]}
										prefix={buildPrefix(prefix, mod.id ?? '')}
										subflowParentsGlobalModuleStates={[
											localModuleStates,
											...globalModuleStates,
											...subflowParentsGlobalModuleStates
										]}
										subflowParentsDurationStatuses={[
											localDurationStatuses,
											...globalDurationStatuses,
											...subflowParentsDurationStatuses
										]}
										render={selected == 'sequence' && render}
										{workspaceId}
										jobId={mod.job ?? ''}
										{reducedPolling}
										isSubflow
										onJobsLoaded={({ job, force }) => {
											onJobsLoadedInner(mod, job, force)
										}}
										{onResultStreamUpdate}
										graphTabOpen={selected == 'graph' && graphTabOpen}
										isNodeSelected={false}
									/>
								{:else if mod.flow_jobs?.length == 0 && mod.job == '00000000-0000-0000-0000-000000000000'}
									<div class="text-secondary">no subflow (empty loop?)</div>
								{:else}
									<FlowStatusViewerInner
										topModuleStates={getTopModuleStates}
										{refreshGlobal}
										updateRecursiveRefreshFn={updateRecursiveRefreshInner}
										globalModuleStates={[localModuleStates, ...globalModuleStates]}
										globalDurationStatuses={[localDurationStatuses, ...globalDurationStatuses]}
										render={selected == 'sequence' && render}
										{workspaceId}
										{prefix}
										{updateGlobalRefresh}
										{subflowParentsGlobalModuleStates}
										{subflowParentsDurationStatuses}
										{isSelectedBranch}
										jobId={mod.job ?? ''}
										{reducedPolling}
										innerModule={mod.flow_jobs ? job.raw_flow?.modules[i]?.value : undefined}
										flowJobIds={mod.flow_jobs
											? {
													moduleId: mod.id ?? '',
													flowJobs: mod.flow_jobs,
													flowJobsSuccess: mod.flow_jobs_success ?? [],
													flowJobsDuration: mod.flow_jobs_duration,
													length: mod.iterator?.itered_len ?? mod.flow_jobs.length,
													branchall: job?.raw_flow?.modules?.[i]?.value?.type == 'branchall'
												}
											: undefined}
										onJobsLoaded={({ job, force }) => {
											onJobsLoadedInner(mod, job, force)
										}}
										loadExtraLogs={(logs) => {
											setModuleState(mod.id ?? '', {
												logs
											})
										}}
										{onResultStreamUpdate}
										graphTabOpen={selected == 'graph' && graphTabOpen}
										isNodeSelected={localModuleStates?.[selectedNode ?? '']?.job_id == mod.job}
										{toolCallStore}
										showLogsWithResult={['script', 'rawscript'].includes(
											job.raw_flow?.modules[i]?.value?.type ?? ''
										)}
										showJobDetailHeader={['script', 'rawscript'].includes(
											job.raw_flow?.modules[i]?.value?.type ?? ''
										)}
									/>
									{#if mod.agent_actions && mod.agent_actions.length > 0 && mod.id}
										{@const storeKeyPrefix = getParentLoopsPrefix(mod.id)}
										{#each mod.agent_actions as agentAction, j}
											{#if agentAction.type === 'tool_call'}
												{@const toolCallId = getToolCallId(j, mod.id, agentAction.module_id)}
												{@const localToolCallKey = mod.id + '-' + j}
												{@const storeKey = storeKeyPrefix + localToolCallKey}
												{@const storedToolCallJob = toolCallStore?.getStoredToolCallJob(storeKey)}
												{@const isSelected = localToolCallKey === selectedToolCall}
												<Button
													variant={isSelected ? 'contained' : 'border'}
													color={mod.agent_actions_success?.[j] === false
														? 'red'
														: isSelected
															? 'dark'
															: 'light'}
													btnClasses="w-full flex justify-start"
													on:click={async () => {
														if (isSelected) {
															selectedToolCall = undefined
														} else {
															selectedToolCall = localToolCallKey
														}
													}}
													endIcon={{
														icon: ChevronDown,
														classes: isSelected ? '!rotate-180' : ''
													}}
												>
													<span class="truncate font-mono">
														Tool call: {agentAction.function_name}
													</span>
												</Button>
												{#if isSelected || storedToolCallJob || toolCallStore?.isToolCallToBeLoaded(storeKey)}
													<FlowStatusViewerInner
														topModuleStates={getTopModuleStates}
														{refreshGlobal}
														updateRecursiveRefreshFn={updateRecursiveRefreshInner}
														globalModuleStates={[localModuleStates, ...globalModuleStates]}
														globalDurationStatuses={[
															localDurationStatuses,
															...globalDurationStatuses
														]}
														render={selected == 'sequence' && render && isSelected}
														{workspaceId}
														{prefix}
														{updateGlobalRefresh}
														{subflowParentsGlobalModuleStates}
														{subflowParentsDurationStatuses}
														{isSelectedBranch}
														jobId={agentAction.job_id}
														job={storedToolCallJob}
														initialJob={storedToolCallJob}
														{reducedPolling}
														onJobsLoaded={({ job, force }) => {
															toolCallStore?.setStoredToolCallJob(storeKey, job)
															onJobsLoadedInner({ id: toolCallId } as FlowStatusModule, job, force)
														}}
														loadExtraLogs={(logs) => {
															setModuleState(toolCallId, {
																logs
															})
														}}
														{onResultStreamUpdate}
														graphTabOpen={selected == 'graph' && graphTabOpen}
														isNodeSelected={localModuleStates?.[toolCallId]?.job_id ==
															agentAction.job_id}
													/>
												{/if}
											{/if}
										{/each}
									{/if}
								{/if}
							{:else}
								<ModuleStatus
									type={mod.type}
									scheduled_for={localModuleStates?.[mod.id ?? '']?.scheduled_for}
								/>
							{/if}
						</li>
					{/each}
				</ul>
			{:else}
				<div class="p-2 text-primary text-sm italic">Empty flow</div>
			{/if}
		</div>
		{#if selected == 'logs' && render}
			<div
				class="mx-auto overflow-auto"
				bind:clientHeight={tabsHeight.logsHeight}
				style="min-height: {minTabHeight}px"
			>
				<FlowLogViewerWrapper
					{job}
					{localModuleStates}
					{localDurationStatuses}
					{workspaceId}
					{render}
					{onSelectedIteration}
				/>
			</div>
		{:else if selected == 'assets' && render}
			<div
				class="p-2"
				bind:clientHeight={tabsHeight.assetsHeight}
				style="min-height: {minTabHeight}px"
			>
				<JobAssetsViewer {job} />
			</div>
		{/if}
	</div>
	{#if render}
		{#if job.raw_flow && !isListJob}
			<div
				class="{selected != 'graph' ? 'hidden' : ''} grow mt-4"
				bind:clientHeight={tabsHeight.graphHeight}
				style="min-height: {minTabHeight}px"
			>
				<div class="border h-full rounded-md" bind:clientHeight={wrapperHeight}>
					<Splitpanes>
						<Pane bind:size={graphPaneSize} minSize={30}>
							<div class="bg-surface-secondary h-full overflow-auto">
								<div class="flex flex-col" bind:clientHeight={retryStatusHeight}>
									{#each Object.values(retryStatus?.val ?? {}) as count}
										{#if count}
											<span class="text-sm">
												Retry in progress, # of failed attempts: {count}
											</span>
										{/if}
									{/each}
									{#if totalEventsWaiting}
										<span class="text-sm">
											Flow suspended, waiting for {totalEventsWaiting} events
										</span>
									{/if}
								</div>
								<FlowGraphV2
									{selectionManager}
									triggerNode={true}
									download={!hideDownloadInGraph}
									minHeight={wrapperHeight - retryStatusHeight}
									success={jobId != undefined && isSuccess(job?.['success'])}
									flowModuleStates={localModuleStates}
									bind:expandedSubflows
									onSelect={(e) => {
										if (rightColumnSelect != 'node_definition' && rightColumnSelect != 'tracing') {
											rightColumnSelect = 'node_status'
										}
										if (typeof e == 'string') {
											if (e == 'Input') {
												selectedNode = 'start'
												stepDetail = undefined
											} else if (e == 'Result') {
												selectedNode = 'end'
												stepDetail = 'end'
											} else {
												const id = e.startsWith(AI_TOOL_CALL_PREFIX) ? e.split('-').pop() : e
												const mod = dfs(job?.raw_flow?.modules ?? [], (m) => m).find(
													(m) => m?.id === id
												)
												stepDetail = mod
												selectedNode = e
												if (e.startsWith(AI_TOOL_CALL_PREFIX)) {
													const [_prefix, agentModuleId, j, _toolModuleId] = e.split('-')
													const parentLoopsPrefix = getParentLoopsPrefix(agentModuleId)
													const jIdx = Number(j)
													const storeKey = parentLoopsPrefix + agentModuleId + '-' + jIdx
													toolCallStore?.addToolCallToLoad(storeKey)
												}
											}
										} else {
											stepDetail = e
											selectedNode = e.id
										}
									}}
									{onSelectedIteration}
									earlyStop={job.raw_flow?.skip_expr !== undefined}
									cache={job.raw_flow?.cache_ttl !== undefined}
									modules={job.raw_flow?.modules ?? []}
									notes={job.raw_flow?.notes ?? []}
									failureModule={job.raw_flow?.failure_module}
									preprocessorModule={job.raw_flow?.preprocessor_module}
									allowSimplifiedPoll={false}
									{workspace}
								/>
							</div>
						</Pane>
						<Pane bind:size={detailsPaneSize} minSize={25} class="!overflow-visible">
							<div class="pt-1 overflow-auto min-h-[700px] flex flex-col h-full">
								<Tabs bind:selected={rightColumnSelect}>
									{#if !hideTimeline}
										<Tab value="timeline" label="Timeline" />
									{/if}
									<Tab value="node_status" label="Node status" />
									{#if !hideNodeDefinition}
										<Tab value="node_definition" label="Node definition" />
									{/if}
									{#if Object.keys(job?.flow_status?.user_states ?? {}).length > 0}
										<Tab value="user_states" label="User States" />
									{/if}
									<Tab value="tracing" label="Tracing" />
								</Tabs>
								{#if rightColumnSelect == 'timeline'}
									<FlowTimeline
										{localModuleStates}
										{onSelectedIteration}
										selfWaitTime={job?.self_wait_time_ms}
										aggregateWaitTime={job?.aggregate_wait_time_ms}
										flowDone={job?.['success'] != undefined}
										bind:this={flowTimeline}
										flowModules={allModulesForTimeline(
											job?.raw_flow?.modules ?? [],
											expandedSubflows ?? {}
										)}
										buildSubflowKey={(key) => buildSubflowKey(key, prefix)}
										durationStatuses={localDurationStatuses}
									/>
								{:else if rightColumnSelect == 'node_status'}
									<div class="p-4 grow flex flex-col gap-6">
										{#if selectedNode?.startsWith(AI_TOOL_MESSAGE_PREFIX)}
											<div class="pt-2 pb-4">
												<Alert
													type="info"
													title="Message output is available on the AI agent node"
												/>
											</div>
										{:else if selectedNode?.startsWith(AI_MCP_TOOL_CALL_PREFIX)}
											{@const [, agentModuleId, toolCallIndex] = selectedNode.split('-')}
											{@const agentNode = localModuleStates?.[agentModuleId]}
											{@const agentActions = agentNode?.agent_actions}
											{@const mcpActionIndex = parseInt(toolCallIndex)}
											{@const mcpAction =
												agentActions && mcpActionIndex >= 0 && mcpActionIndex < agentActions.length
													? agentActions[mcpActionIndex]
													: undefined}
											{#if mcpAction?.type === 'mcp_tool_call' && agentNode?.result?.messages}
												{@const message = agentNode.result.messages.find(
													(m: { agent_action?: { call_id?: string }; content?: any }) =>
														m.agent_action?.call_id === mcpAction.call_id
												)}
												<McpToolCallDetails
													functionName={mcpAction.function_name}
													args={mcpAction.arguments ?? {}}
													result={message?.content}
													type="Success"
													workspaceId={job?.workspace_id}
												/>
											{/if}
										{:else if selectedNode?.startsWith(AI_WEBSEARCH_PREFIX)}
											<Alert
												type="info"
												title="Web search output is available on the AI agent node"
											/>
										{:else if selectedNode}
											{@const node = localModuleStates[selectedNode]}
											{#if selectedNode == 'end'}
												<FlowJobResult
													tagLabel={customUi?.tagLabel}
													workspaceId={isReplay ? undefined : job?.workspace_id}
													jobId={isReplay ? undefined : job?.id}
													filename={job.id}
													loading={job['running']}
													tag={job?.tag}
													col
													result={job['result']}
													logs={job.logs ?? ''}
													downloadLogs={!hideDownloadLogs && !isReplay}
												/>
											{:else if selectedNode == 'start'}
												{#if job.args}
													<JobArgs
														id={isReplay ? undefined : job.id}
														workspace={isReplay ? undefined : (job.workspace_id ?? $workspaceStore ?? 'no_w')}
														args={job.args}
													/>
												{:else}
													<p class="text-secondary">No arguments</p>
												{/if}
											{:else if node}
												{@const module =
													stepDetail && typeof stepDetail !== 'string' ? stepDetail : undefined}
												{@const agentTools =
													module && module.value.type === 'aiagent'
														? module.value.tools
														: undefined}
												{@const parentLoopsPrefix = getParentLoopsPrefix(module?.id ?? '')}
												{#if node.flow_jobs_results}
													<div>
														<span class="pl-1 text-emphasis text-xs font-medium"
															>Result of step as collection of all subflows</span
														>
														<div class="overflow-auto max-h-[200px] p-2">
															<DisplayResult
																workspaceId={isReplay ? undefined : job?.workspace_id}
																result={node.flow_jobs_results}
																nodeId={selectedNode}
																jobId={isReplay ? undefined : job?.id}
																language={job?.language}
															/>
														</div>
													</div>
												{/if}
												<div class="flex flex-col gap-2">
													{#if node.flow_jobs_results}
														<span class="pl-1 text-xs font-medium text-emphasis pt-4"
															>Selected subflow</span
														>
													{/if}
													<div class="flex flex-col gap-6">
														<div class="flex flex-col gap-6">
															<div class="flex gap-2 min-w-0 w-full items-center">
																<ModuleStatus
																	type={node.type}
																	scheduled_for={node.scheduled_for}
																	skipped={node.skipped}
																/>
																{#if node.duration_ms}
																	<Badge>
																		<Hourglass class="mr-2" size={10} />
																		{msToSec(node.duration_ms)} s
																	</Badge>
																{/if}
																{#if node.job_id && !isReplay}
																	<div class="grow w-full flex flex-row-reverse">
																		<a
																			class="text-right text-xs"
																			rel="noreferrer"
																			target="_blank"
																			href="{base}/run/{node.job_id ??
																				''}?workspace={job?.workspace_id}"
																		>
																			{truncateRev(node.job_id ?? '', 10)}
																			<ExternalLink size={12} class="inline-block" />
																		</a>
																	</div>
																{/if}
															</div>
															{#if !node.isListJob}
																<div>
																	<div class="text-xs text-emphasis font-semibold mb-1">Inputs</div>
																	<JobArgs
																		id={isReplay ? undefined : node.job_id}
																		workspace={isReplay ? undefined : (job.workspace_id ?? $workspaceStore ?? 'no_w')}
																		args={node.args}
																	/>
																</div>
															{/if}
														</div>
														<FlowJobResult
															tagLabel={customUi?.tagLabel}
															workspaceId={isReplay ? undefined : job?.workspace_id}
															jobId={isReplay ? undefined : node.job_id}
															loading={node.type != 'Success' && node.type != 'Failure'}
															waitingForExecutor={node.type == 'WaitingForExecutor'}
															refreshLog={node.type == 'InProgress'}
															col
															result_stream={resultStreams[node.job_id ?? '']}
															result={node.result}
															tag={node.tag}
															logs={node.logs}
															downloadLogs={!hideDownloadLogs && !isReplay}
															aiAgentStatus={agentTools &&
															node?.job_id &&
															(node.type === 'Success' || node.type === 'Failure')
																? {
																		tools: agentTools,
																		agentJob: {
																			id: node.job_id,
																			result: node.result,
																			logs: node.logs,
																			args: node.args,
																			success: node.type === 'Success',
																			type: 'CompletedJob'
																		},
																		storedToolCallJobs: module
																			? toolCallStore?.getLocalToolCallJobs(parentLoopsPrefix)
																			: undefined,
																		onToolJobLoaded: (job, idx) => {
																			if (module) {
																				const storeKey = parentLoopsPrefix + module.id + '-' + idx
																				toolCallStore?.setStoredToolCallJob(storeKey, job)
																			}
																		}
																	}
																: undefined}
														/>
													</div>
												</div>
											{:else}
												<p class="p-2 text-primary italic"
													>The execution of this node has no information attached to it. The job
													likely did not run yet</p
												>
											{/if}
										{:else}<p class="p-2 text-primary italic"
												>Select a node to see its details here</p
											>{/if}
									</div>
								{:else if rightColumnSelect == 'node_definition'}
									{@const node = selectedNode ? localModuleStates[selectedNode] : undefined}
									<FlowGraphViewerStep {stepDetail} jobScriptHash={node?.script_hash} />
								{:else if rightColumnSelect == 'user_states'}
									<div class="p-2">
										<JobArgs argLabel="Key" args={job?.flow_status?.user_states ?? {}} />
									</div>
								{:else if rightColumnSelect == 'tracing'}
									{@const node = selectedNode ? localModuleStates[selectedNode] : undefined}
									{#if node?.job_id}
										<JobOtelTraces jobId={node.job_id} />
									{:else}
										<div class="p-4 text-secondary"
											>Select a node with a job to see HTTP request traces</div
										>
									{/if}
								{/if}
							</div>
						</Pane>
					</Splitpanes>
				</div>
			</div>
		{/if}
	{/if}
{:else}
	<Skeleton layout={[[15], 1, [70]]}></Skeleton>
{/if}

<style>
	.line {
		background: repeating-linear-gradient(to bottom, transparent 0 4px, #bbb 4px 8px) 50%/1px 100%
			no-repeat;
	}
</style>
