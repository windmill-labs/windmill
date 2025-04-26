<script lang="ts">
	import {
		type FlowStatusModule,
		type Job,
		JobService,
		type FlowStatus,
		type FlowModuleValue,
		type FlowModule
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'
	import FlowJobResult from './FlowJobResult.svelte'
	import FlowPreviewStatus from './preview/FlowPreviewStatus.svelte'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import { onDestroy } from 'svelte'
	import { Badge, Button, Skeleton, Tab } from './common'
	import DisplayResult from './DisplayResult.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import { type DurationStatus, type FlowStatusViewerContext, type GraphModuleState } from './graph'
	import ModuleStatus from './ModuleStatus.svelte'
	import { emptyString, isScriptPreview, msToSec, truncateRev } from '$lib/utils'
	import JobArgs from './JobArgs.svelte'
	import { ChevronDown, Hourglass, Loader2 } from 'lucide-svelte'
	import FlowStatusWaitingForEvents from './FlowStatusWaitingForEvents.svelte'
	import { deepEqual } from 'fast-equals'
	import FlowTimeline from './FlowTimeline.svelte'
	import { dfs } from './flows/dfs'
	import { get, writable, type Unsubscriber, type Writable } from 'svelte/store'
	import Alert from './common/alert/Alert.svelte'
	import FlowGraphViewerStep from './FlowGraphViewerStep.svelte'
	import FlowGraphV2 from './graph/FlowGraphV2.svelte'
	import { buildPrefix } from './graph/graphBuilder'

	const dispatch = createEventDispatcher()

	let {
		flowStateStore,
		retryStatus,
		suspendStatus,
		hideDownloadInGraph,
		hideTimeline,
		hideNodeDefinition,
		hideDownloadLogs
	} = getContext<FlowStatusViewerContext>('FlowStatusViewer')

	export let jobId: string
	export let initialJob: Job | undefined = undefined
	export let workspaceId: string | undefined = undefined
	export let flowJobIds:
		| {
				moduleId: string
				flowJobs: string[]
				flowJobsSuccess: (boolean | undefined)[]
				length: number
				branchall?: boolean
		  }
		| undefined = undefined

	//only useful when forloops are optimized and the job doesn't contain the mod id anymore
	export let innerModule: FlowModuleValue | undefined = undefined
	export let globalRefreshes: Record<string, (clear, root) => Promise<void>> = {}
	export let render = true

	export let isOwner = false

	export let selectedNode: string | undefined = undefined

	export let globalModuleStates: Writable<Record<string, GraphModuleState>>[]
	export let globalDurationStatuses: Writable<Record<string, DurationStatus>>[]

	export let childFlow: boolean = false
	export let isSubflow: boolean = false
	export let reducedPolling = false

	export let wideResults = false
	export let hideFlowResult = false
	export let workspace: string | undefined = $workspaceStore
	export let prefix: string | undefined = undefined
	export let subflowParentsGlobalModuleStates: Writable<Record<string, GraphModuleState>>[] = []
	export let subflowParentsDurationStatuses: Writable<Record<string, DurationStatus>>[] = []
	export let isForloopSelected = false
	export let parentRecursiveRefresh: Record<string, (clear, root) => Promise<void>> = {}
	export let job: Job | undefined = undefined
	export let rightColumnSelect: 'timeline' | 'node_status' | 'node_definition' | 'user_states' =
		'timeline'

	let recursiveRefresh: Record<string, (clear, root) => Promise<void>> = {}

	let jobResults: any[] =
		flowJobIds?.flowJobs?.map((x, id) => `iter #${id + 1} not loaded by frontend yet`) ?? []

	let retry_selected = ''
	let timeout: NodeJS.Timeout | undefined = undefined

	let localModuleStates: Writable<Record<string, GraphModuleState>> = writable({})
	let localDurationStatuses: Writable<Record<string, DurationStatus>> = writable({})
	let expandedSubflows: Record<string, FlowModule[]> = {}

	$: flowJobIds?.moduleId && onFlowModuleId()

	let selectedId: Writable<string | undefined> = writable(selectedNode)

	function onFlowModuleId() {
		if (globalRefreshes) {
			let modId = flowJobIds?.moduleId
			if (modId) {
				globalRefreshes[buildSubflowKey(modId, prefix)] = async (clear, root) => {
					await refresh(clear, root) // refresh(true, loopJob)
				}
			}
		}
	}

	function updateModuleStates(
		moduleState: Writable<Record<string, GraphModuleState>>,
		key: string,
		newValue: GraphModuleState,
		keepType: boolean | undefined
	) {
		const state = get(moduleState)
		if (
			newValue.selectedForloop != undefined &&
			state[key]?.selectedForloop != undefined &&
			newValue.selectedForloop != state[key].selectedForloop
		) {
			if (newValue.type == 'InProgress' && state[key]?.type != 'InProgress') {
				moduleState.update((state) => {
					state[key].type = 'InProgress'
					return state
				})
			}

			if (
				state[key]?.job_id != newValue.job_id ||
				!deepEqual(state[key]?.args, newValue.args) ||
				!deepEqual(state[key]?.result, newValue.result)
			) {
				moduleState.update((state) => {
					state[key].args = newValue.args
					state[key].result = newValue.result
					state[key].job_id = newValue.job_id
					return state
				})
			}
			return
		}

		if (state[key]?.selectedForLoopSetManually) {
			if (
				newValue.selectedForloop != undefined &&
				state[key]?.selectedForloop != newValue.selectedForloop
			) {
				return state
			} else {
				newValue.selectedForLoopSetManually = true
				newValue.selectedForloopIndex = state[key]?.selectedForloopIndex
				newValue.selectedForloop = state[key]?.selectedForloop
			}
		} else if (state[key]?.selectedForloopIndex != undefined) {
			newValue.selectedForloopIndex = state[key]?.selectedForloopIndex
			newValue.selectedForloop = state[key]?.selectedForloop
		}

		if (keepType && (state[key]?.type == 'Success' || state[key]?.type == 'Failure')) {
			newValue.type = state[key].type
		}
		if (!deepEqual(state[key], newValue)) {
			moduleState.update((state) => {
				state[key] = newValue
				return state
			})
		}
	}

	function buildSubflowKey(key: string, prefix: string | undefined) {
		return prefix ? 'subflow:' + prefix + key : key
	}

	async function refresh(clearLoop: boolean, rootJob: string | undefined) {
		let modId = flowJobIds?.moduleId

		if (clearLoop) {
			if (!rootJob) {
				let topLevelModuleStates = globalModuleStates?.[globalModuleStates?.length - 1]
				if (modId) {
					topLevelModuleStates?.update((x) => {
						if (modId) {
							delete x[modId]
						}
						return x
					})
				}

				if (subflowParentsGlobalModuleStates.length > 0) {
					subflowParentsGlobalModuleStates?.[subflowParentsGlobalModuleStates?.length - 1]?.update(
						(x) => {
							for (let mod of innerModules ?? []) {
								if (mod.id) {
									delete x[buildSubflowKey(mod.id, prefix)]
								}
							}

							return x
						}
					)
				} else {
					topLevelModuleStates?.update((x) => {
						for (let mod of innerModules ?? []) {
							if (mod.id) {
								delete x[mod.id]
							}
						}

						return x
					})
				}
			}
		} else {
			let state = modId ? getTopModuleStates()?.[modId] : undefined
			let loopjob = state?.selectedForloop
			let njob = flowJobIds && modId && loopjob ? storedListJobs?.[loopjob] : job
			if (njob) {
				dispatch('jobsLoaded', { job: njob, force: true })
			}
		}

		for (let [k, rec] of Object.entries(recursiveRefresh)) {
			if (rootJob != undefined && rootJob != k) {
				continue
			}
			await tick()
			await rec(clearLoop, undefined)
		}
	}

	function updateRecursiveRefresh(jobId: string) {
		if (jobId) {
			parentRecursiveRefresh[jobId] = async (clear, root) => {
				if (globalModuleStates.length > 0 || isSubflow) {
					await refresh(clear, root)
				}
			}
		}
	}

	function setModuleState(
		key: string,
		value: Partial<GraphModuleState>,
		force?: boolean,
		keepType?: boolean
	) {
		let newValue = { ...($localModuleStates[key] ?? {}), ...value }
		if (!deepEqual($localModuleStates[key], value) || force) {
			;[localModuleStates, ...globalModuleStates].forEach((s) => {
				updateModuleStates(s, key, newValue, keepType)
			})
			if (prefix) {
				subflowParentsGlobalModuleStates.forEach((s) =>
					updateModuleStates(s, buildSubflowKey(key, prefix), newValue, keepType)
				)
			}
		}
	}

	function setDurationStatusByJob(key: string, id: string, value: any) {
		if (!deepEqual($localDurationStatuses[key]?.byJob[id], value)) {
			$localDurationStatuses[key].byJob[id] = value
			globalDurationStatuses.forEach((s) => {
				s.update((x) => {
					x[key].byJob[id] = value

					return x
				})
			})
			if (prefix) {
				subflowParentsDurationStatuses.forEach((s) => {
					s.update((x) => {
						x[buildSubflowKey(key, prefix)].byJob[id] = value
						return x
					})
				})
			}
		}
	}

	function initializeByJob(modId: string) {
		if ($localDurationStatuses[modId] == undefined) {
			$localDurationStatuses[modId] = { byJob: {} }
		}
		globalDurationStatuses.forEach((x) =>
			x.update((x) => {
				if (x[modId] == undefined) {
					x[modId] = { byJob: {} }
				}
				return x
			})
		)
		if (prefix) {
			subflowParentsDurationStatuses.forEach((x) =>
				x.update((x) => {
					let key = buildSubflowKey(modId, prefix)
					if (x[key] == undefined) {
						x[key] = { byJob: {} }
					}
					return x
				})
			)
		}
	}

	let innerModules: FlowStatusModule[] = []

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
			$retryStatus[jobId ?? ''] = count
		} else if ($retryStatus[jobId ?? ''] != undefined) {
			delete $retryStatus[jobId ?? '']
			$retryStatus = $retryStatus
		}
		let jobStatus = job?.flow_status?.modules?.[job?.flow_status.step]

		if (jobStatus && jobStatus.count != undefined) {
			$suspendStatus[jobId ?? ''] = { nb: jobStatus.count, job: job! }
		} else if ($suspendStatus[jobId ?? ''] != undefined) {
			delete $suspendStatus[jobId ?? '']
			$suspendStatus = $suspendStatus
		}
	}

	function updateInnerModules() {
		if ($localModuleStates) {
			innerModules.forEach((mod, i) => {
				if (mod.type === 'WaitingForEvents' && innerModules?.[i - 1]?.type === 'Success') {
					setModuleState(mod.id ?? '', { type: mod.type, args: job?.args, tag: job?.tag })
				} else if (
					mod.type === 'WaitingForExecutor' &&
					$localModuleStates[mod.id ?? '']?.scheduled_for == undefined
				) {
					JobService.getJob({
						workspace: workspaceId ?? $workspaceStore ?? '',
						id: mod.job ?? '',
						noLogs: true
					})
						.then((job) => {
							const newState = {
								type: mod.type,
								scheduled_for: job?.['scheduled_for'],
								job_id: job?.id,
								parent_module: mod['parent_module'],
								args: job?.args,
								tag: job?.tag
							}

							setModuleState(mod.id ?? '', newState)
						})
						.catch((e) => {
							console.error(`Could not load inner module for job ${mod.job}`, e)
						})
				} else if (
					mod.flow_jobs &&
					(mod.type == 'Success' || mod.type == 'Failure') &&
					!['Success', 'Failure'].includes($localModuleStates?.[mod.id ?? '']?.type)
				) {
					setModuleState(
						mod.id ?? '',
						{
							type: mod.type
						},
						true
					)
				} else if (isForloopSelected) {
					setModuleState(mod.id ?? '', {}, true)
				}

				// if (isForloopSelected && mod?.flow_jobs) {
				// 	let states = getTopModuleStates()
				// 	if (states) {
				// 		states[mod.id ?? ''] = $localModuleStates[mod.id ?? '']
				// 	}
				// }

				if (mod.branch_chosen) {
					setModuleState(
						mod.id ?? '',
						{
							branchChosen:
								mod.branch_chosen.type == 'default' ? 0 : (mod.branch_chosen.branch ?? 0) + 1
						},
						true
					)
				}

				/**
				 * else if (mod.type === 'Failure' || mod.type === 'WaitingForPriorSteps') {
					if (job?.type === 'CompletedJob') {
						setModuleState('b', {
							type: 'Failure',
							args: job?.args,
							job_id: job?.id,
							result: job?.result
						})
					}
				}
				*/
			})
		}
	}

	$: isForloopSelected && globalModuleStates && debounceLoadJobInProgress()

	async function getNewJob(jobId: string, initialJob: Job | undefined) {
		if (
			jobId == initialJob?.id &&
			initialJob?.id != undefined &&
			initialJob?.type === 'CompletedJob'
		) {
			return initialJob
		} else {
			return await JobService.getJob({
				workspace: workspaceId ?? $workspaceStore ?? '',
				id: jobId ?? '',
				noLogs: true
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

	let errorCount = 0
	let notAnonynmous = false
	let started = false
	async function loadJobInProgress() {
		if (!started) {
			started = true
			dispatch('start')
		}
		if (jobId != '00000000-0000-0000-0000-000000000000') {
			try {
				const newJob = await getNewJob(jobId, initialJob)
				if (!deepEqual(job, newJob) || isForloopSelected) {
					job = newJob
					job?.flow_status && updateStatus(job?.flow_status)
					dispatch('jobsLoaded', { job, force: false })
				}
				errorCount = 0
				notAnonynmous = false
			} catch (e) {
				if (
					e?.body?.includes('As a non logged in user, you can only see jobs ran by anonymous users')
				) {
					notAnonynmous = true
				} else {
					errorCount += 1
					console.error(e)
				}
			}
		}
		if (job?.type !== 'CompletedJob' && errorCount < 4 && !destroyed) {
			debounceLoadJobInProgress()
		} else {
			dispatch('done', job)
		}
	}

	let destroyed = false

	updateRecursiveRefresh(jobId)

	async function updateJobId() {
		if (jobId !== job?.id) {
			$localModuleStates = {}
			flowTimeline?.reset()
			timeout && clearTimeout(timeout)
			innerModules = []
			if (flowJobIds) {
				let modId = flowJobIds?.moduleId ?? ''

				let common = {
					iteration_from: flowJobIds?.branchall ? 0 : Math.max(flowJobIds.flowJobs.length - 20, 0),
					iteration_total: $localDurationStatuses?.[modId]?.iteration_total ?? flowJobIds?.length
				}
				$localDurationStatuses[modId] = {
					...($localDurationStatuses[modId] ?? { byJob: {} }),
					...common
				}
				let prefixed = modId
				globalDurationStatuses.forEach((x) =>
					x.update((x) => {
						x[prefixed] = { ...(x[prefixed] ?? { byJob: {} }), ...common }
						return x
					})
				)
			} else {
				updateRecursiveRefresh(jobId)
				recursiveRefresh = {}
				$localDurationStatuses = {}
			}
			await loadJobInProgress()
		}
	}

	$: jobId && updateJobId()

	$: isListJob = flowJobIds != undefined && Array.isArray(flowJobIds?.flowJobs)

	function getTopModuleStates() {
		return get(globalModuleStates?.[globalModuleStates?.length - 1])
	}

	let forloop_selected = getTopModuleStates()?.[flowJobIds?.moduleId ?? '']?.selectedForloop

	let sub: Unsubscriber | undefined = undefined
	let timeoutForloopSelectedSub: NodeJS.Timeout | undefined = undefined
	let timeoutForloopSelected: NodeJS.Timeout | undefined = undefined
	$: flowJobIds?.moduleId && onModuleIdChange()

	function onModuleIdChange() {
		clearTimeout(timeoutForloopSelectedSub)
		timeoutForloopSelectedSub = setTimeout(() => {
			sub?.()
			sub = globalModuleStates?.[globalModuleStates?.length - 1].subscribe((x) => {
				const newForloopSelected = x[flowJobIds?.moduleId ?? '']?.selectedForloop
				if (newForloopSelected != forloop_selected) {
					clearTimeout(timeoutForloopSelected)
					timeoutForloopSelected = setTimeout(() => {
						forloop_selected = newForloopSelected
					}, 200)
				}
			})
		}, 200)
	}

	onDestroy(() => {
		destroyed = true
		timeout && clearTimeout(timeout)
		sub?.()
	})

	$: selected = isListJob ? 'sequence' : 'graph'

	function isSuccess(arg: any): boolean | undefined {
		if (arg == undefined) {
			return undefined
		} else {
			return arg == true
		}
	}

	function onJobsLoaded(mod: FlowStatusModule, job: Job, force?: boolean): void {
		if (mod.id && (mod.flow_jobs ?? []).length == 0) {
			if (!childFlow) {
				if ($flowStateStore?.[mod.id]) {
					$flowStateStore[mod.id] = {
						...$flowStateStore[mod.id],
						previewResult: job['result'],
						previewArgs: job.args,
						previewJobId: job.id,
						previewWorkspaceId: job.workspace_id,
						previewSuccess: job['success']
					}
				}
			}
			initializeByJob(mod.id)
			let started_at = job.started_at ? new Date(job.started_at).getTime() : undefined
			if (job.type == 'QueuedJob') {
				setModuleState(
					mod.id,
					{
						type: 'InProgress',
						job_id: job.id,
						logs: job.logs,
						args: job.args,
						tag: job.tag,
						started_at,
						parent_module: mod['parent_module']
					},
					force
				)
				setDurationStatusByJob(mod.id, job.id, {
					created_at: job.created_at ? new Date(job.created_at).getTime() : undefined,
					started_at
				})
			} else {
				const parent_module = mod['parent_module']

				// Delete existing failure node attached to the same parent module
				removeFailureNode(mod.id, parent_module)

				setModuleState(
					mod.id,
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
						iteration_total: mod.iterator?.itered?.length,
						retries: mod?.failed_retries?.length,
						skipped: mod.skipped
						// retries: $flowStateStore?.raw_flow
					},
					force
				)

				setDurationStatusByJob(mod.id, job.id, {
					created_at: job.created_at ? new Date(job.created_at).getTime() : undefined,
					started_at,
					duration_ms: job['duration_ms']
				})
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
			let globalState = globalModuleStates?.[globalModuleStates?.length - 1]
			let globalStateGet = globalState ? get(globalState) : undefined
			let state = globalStateGet?.[modId]

			if (clicked && state?.selectedForloop) {
				await globalRefreshes?.[modId]?.(true, state.selectedForloop)
			}
			let manualOnce = state?.selectedForLoopSetManually
			if (
				clicked ||
				(!manualOnce &&
					(state == undefined || !isForloop || j >= (state.selectedForloopIndex ?? -1)))
			) {
				let setManually = clicked || manualOnce

				let newState = {
					...(state ?? {}),
					selectedForloop: id,
					selectedForloopIndex: j,
					selectedForLoopSetManually: setManually
				}

				const selectedNotEqual =
					id != state?.selectedForloop ||
					j != state?.selectedForloopIndex ||
					setManually != state?.selectedForLoopSetManually
				if (selectedNotEqual) {
					globalState?.update((topLevelModuleStates) => {
						topLevelModuleStates[modId] = {
							type: 'WaitingForPriorSteps',
							args: {},
							...newState
						}
						return topLevelModuleStates
						// clicked && callGlobRefresh(modId, {index: j, job: id, selectedManually: setManually ?? false})
					})
				}
			}

			if (clicked) {
				await globalRefreshes?.[modId]?.(false, id)
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

			if ($flowStateStore && $flowStateStore?.[modId] == undefined) {
				$flowStateStore[modId] = {
					...(($flowStateStore[modId] as object) ?? {}),
					previewResult: jobLoaded.args
				}
			}
			if ($flowStateStore?.[modId]) {
				if (!childFlow) {
					if (
						!$flowStateStore[modId].previewResult ||
						!Array.isArray($flowStateStore[modId]?.previewResult)
					) {
						$flowStateStore[modId].previewResult = []
					}
					$flowStateStore[modId].previewArgs = jobLoaded.args
				}
				if (jobLoaded.type == 'QueuedJob') {
					jobResults[j] = 'Job in progress ...'
				} else if (jobLoaded.type == 'CompletedJob') {
					$flowStateStore[modId].previewResult[j] = jobLoaded.result
					jobResults[j] = jobLoaded.result
				}
			}

			let started_at = jobLoaded.started_at ? new Date(jobLoaded.started_at).getTime() : undefined

			let created_at = jobLoaded.created_at ? new Date(jobLoaded.created_at).getTime() : undefined

			let job_id = jobLoaded.id
			initializeByJob(modId)

			let v: Partial<GraphModuleState> = {
				started_at,
				flow_jobs: flowJobIds?.flowJobs,
				flow_jobs_success: flowJobIds?.flowJobsSuccess,
				iteration_total: flowJobIds?.length,
				duration_ms: undefined
			}

			let currentIndex = getTopModuleStates()?.[modId]?.selectedForloopIndex == j
			if (currentIndex) {
				v.logs = jobLoaded.logs
				v.args = jobLoaded.args
				v.job_id = jobLoaded.id
			}

			if (jobLoaded.type == 'QueuedJob') {
				if (started_at && $localModuleStates[modId]?.type != 'InProgress') {
					v.type = 'InProgress'
				}
			} else if (jobLoaded.type == 'CompletedJob') {
				v.flow_jobs_results = jobResults
				if (currentIndex) {
					v.result = jobLoaded.result
				}
			}
			setModuleState(modId, v, force, true)

			if (jobLoaded.type == 'QueuedJob') {
				setDurationStatusByJob(modId, job_id, {
					created_at,
					started_at
				})
			} else if (jobLoaded.type == 'CompletedJob') {
				setDurationStatusByJob(modId, job_id, {
					created_at,
					started_at,
					duration_ms: jobLoaded.duration_ms
				})
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
					onJobsLoaded({ id } as FlowStatusModule, jobLoaded)
				}
			}
		}
	}

	let flowTimeline: FlowTimeline

	function loadPreviousIters(lenToAdd: number) {
		let r = $localDurationStatuses[flowJobIds?.moduleId ?? '']
		if (r.iteration_from) {
			r.iteration_from -= lenToAdd
			$localDurationStatuses = $localDurationStatuses
			globalDurationStatuses.forEach((x) => x.update((x) => x))
		}
		jobResults = [
			...[...new Array(lenToAdd).keys()].map((x) => 'not computed or loaded yet'),
			...jobResults
		]
		// updateSlicedListJobIds()
	}

	let stepDetail: FlowModule | string | undefined = undefined

	let storedListJobs: Record<number, Job> = {}
	let wrapperHeight: number = 0

	function removeFailureNode(id: string, parent_module: any) {
		if (id?.startsWith('failure-') && parent_module) {
			;[...globalModuleStates, localModuleStates].forEach((stateMapStore) => {
				stateMapStore.update((stateMap) => {
					if (id) {
						Object.keys(stateMap).forEach((key) => {
							if (stateMap[key]?.parent_module == parent_module) {
								delete stateMap[key]
							}
						})
					}
					return stateMap
				})
			})
		}
	}

	function allModulesForTimeline(
		modules: FlowModule[],
		expandedSubflows: Record<string, FlowModule[]>
	): string[] {
		const ids = dfs(modules, (x) => x.id)

		function rec(ids: string[], prefix: string | undefined): string[] {
			return ids.concat(
				ids.flatMap((id) => {
					let fms = expandedSubflows[id]
					let oid = id.split(':').pop()
					if (!oid) {
						return []
					}
					let nprefix = buildPrefix(prefix, oid)
					return fms
						? rec(
								dfs(fms, (x) =>
									x.id.startsWith('subflow:') ? x.id : buildSubflowKey(x.id, nprefix)
								),
								nprefix
							)
						: []
				})
			)
		}

		return rec(ids, undefined)
	}

	let subflowsSize = 500
</script>

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
			{@const sliceFrom = $localDurationStatuses[flowJobIds?.moduleId ?? '']?.iteration_from ?? 0}
			{@const lenToAdd = Math.min(20, sliceFrom)}

			{#if (flowJobIds?.flowJobs.length ?? 0) > 20 && lenToAdd > 0}
				{@const allToAdd = (flowJobIds?.length ?? 0) - sliceFrom}
				<p class="text-tertiary italic text-xs">
					For performance reasons, only the last 20 items are shown by default <button
						class="text-primary underline ml-4"
						on:click={() => {
							loadPreviousIters(lenToAdd)
						}}
						>Load {lenToAdd} prior
					</button>
					{#if allToAdd > 0 && allToAdd > lenToAdd}
						{sliceFrom}
						<button
							class="text-primary underline ml-4"
							on:click={() => {
								loadPreviousIters(allToAdd)
							}}
							>Load {allToAdd} prior
						</button>
					{/if}
				</p>
			{/if}
			{#if render}
				<div class="w-full h-full border rounded-sm bg-surface p-1 overflow-auto">
					<DisplayResult
						workspaceId={job?.workspace_id}
						{jobId}
						result={jobResults}
						language={job?.language}
					/>
				</div>
			{/if}
		{:else if render}
			<div class={'border rounded-md shadow p-2'}>
				<FlowPreviewStatus {job} />
				{#if !job}
					<div>
						<Loader2 class="animate-spin" />
					</div>
				{:else if `result` in job}
					{#if !hideFlowResult}
						<div class="w-full h-full">
							<FlowJobResult
								workspaceId={job?.workspace_id}
								jobId={job?.id}
								tag={job?.tag}
								loading={job['running'] == true}
								result={job.result}
								logs={job.logs}
								durationStates={localDurationStatuses}
								downloadLogs={!hideDownloadLogs}
							/>
						</div>
					{/if}
				{:else if job.flow_status?.modules?.[job?.flow_status?.step]?.type === 'WaitingForEvents'}
					<FlowStatusWaitingForEvents {workspaceId} {job} {isOwner} />
				{:else if $suspendStatus && Object.keys($suspendStatus).length > 0}
					<div class="flex gap-2 flex-col">
						{#each Object.values($suspendStatus) as suspendCount (suspendCount.job.id)}
							<div>
								<div class="text-sm">
									Flow suspended, waiting for {suspendCount.nb} events
								</div>
								<FlowStatusWaitingForEvents job={suspendCount.job} {workspaceId} {isOwner} />
							</div>
						{/each}
					</div>
				{:else if job.logs}
					<div
						class="text-xs p-4 bg-surface-secondary overflow-auto max-h-80 border border-tertiary-inverse"
					>
						<pre class="w-full">{job.logs}</pre>
					</div>
				{:else if innerModules?.length > 0}
					<div class="flex flex-col gap-1">
						{#each innerModules as mod, i (mod.id)}
							{#if mod.type == 'InProgress'}
								{@const rawMod = job.raw_flow?.modules[i]}

								<div
									><span class="inline-flex gap-1"
										><Badge color="indigo">{mod.id}</Badge>
										<span class="font-medium text-primary">
											{#if !emptyString(rawMod?.summary)}
												{rawMod?.summary ?? ''}
											{:else if rawMod?.value.type == 'script'}
												{rawMod.value.path ?? ''}
											{:else if rawMod?.value.type}
												{rawMod?.value.type}
											{/if}
										</span>

										<Loader2 class="animate-spin" /></span
									></div
								>
							{/if}
						{/each}
					</div>
				{/if}
			</div>
		{/if}
		{#if render}
			{#if innerModules.length > 0 && !isListJob}
				<Tabs class="mx-auto {wideResults ? '' : 'max-w-7xl'}" bind:selected>
					<Tab value="graph"><span class="font-semibold text-md">Graph</span></Tab>
					<Tab value="sequence"><span class="font-semibold">Details</span></Tab>
				</Tabs>
			{:else}
				<div class="h-[30px]"></div>
			{/if}
		{/if}
		<div class="{selected != 'sequence' ? 'hidden' : ''} max-w-7xl mx-auto">
			{#if isListJob}
				{@const sliceFrom = $localDurationStatuses[flowJobIds?.moduleId ?? '']?.iteration_from ?? 0}
				<h3 class="text-md leading-6 font-bold text-tertiary border-b mb-4">
					Subflows ({flowJobIds?.flowJobs.length})
				</h3>
				<div class="overflow-auto max-h-1/2">
					{#each flowJobIds?.flowJobs ?? [] as loopJobId, j (loopJobId)}
						{#if render && j + subflowsSize + 1 == (flowJobIds?.flowJobs.length ?? 0)}
							<Button variant="border" color="light" on:click={() => (subflowsSize += 500)}
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
									if (!storedJob) {
										storedJob = await JobService.getJob({
											workspace: workspaceId ?? $workspaceStore ?? '',
											id: loopJobId,
											noLogs: true
										})
										storedListJobs[j] = storedJob
									}
									innerJobLoaded(storedJob, j, true, false)
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
						{#if j >= sliceFrom || forloop_selected == loopJobId}
							{@const forloopIsSelected =
								forloop_selected == loopJobId ||
								(innerModule?.type != 'forloopflow' && innerModule?.type != 'whileloopflow')}
							<!-- <LogId id={loopJobId} /> -->
							<div class="border p-6" class:hidden={forloop_selected != loopJobId}>
								<svelte:self
									{globalRefreshes}
									parentRecursiveRefresh={recursiveRefresh}
									{childFlow}
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
									render={forloop_selected == loopJobId && selected == 'sequence' && render}
									isForloopSelected={forloop_selected == loopJobId &&
										(innerModule?.type == 'forloopflow' || innerModule?.type == 'whileloopflow')}
									reducedPolling={reducedPolling ||
										(flowJobIds?.flowJobs.length && flowJobIds?.flowJobs.length > 20)}
									{workspaceId}
									jobId={loopJobId}
									on:jobsLoaded={(e) => {
										let { job, force } = e.detail
										storedListJobs[j] = job
										innerJobLoaded(job, j, false, force)
									}}
								/>
							</div>
						{/if}
					{/each}
				</div>
			{:else if innerModules.length > 0 && (job.raw_flow?.modules.length ?? 0) > 0}
				{@const hasPreprocessor = innerModules[0]?.id == 'preprocessor' ? 1 : 0}
				<ul class="w-full">
					<h3 class="text-md leading-6 font-bold text-primary border-b mb-4 py-2">
						Step-by-step
					</h3>

					{#each innerModules as mod, i}
						{#if render}
							<div class="line w-8 h-10"></div>
							<h3 class="text-tertiary mb-2 w-full">
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
										<svelte:self
											{globalRefreshes}
											parentRecursiveRefresh={recursiveRefresh}
											{childFlow}
											globalModuleStates={[localModuleStates, ...globalModuleStates]}
											globalDurationStatuses={[localDurationStatuses, ...globalDurationStatuses]}
											{prefix}
											{subflowParentsGlobalModuleStates}
											{subflowParentsDurationStatuses}
											render={failedRetry == retry_selected && render}
											{reducedPolling}
											{workspaceId}
											jobId={failedRetry}
										/>
									</div>
								{/each}
							{/if}
							{#if ['InProgress', 'Success', 'Failure'].includes(mod.type)}
								{#if job.raw_flow?.modules[i]?.value.type == 'flow'}
									<svelte:self
										{globalRefreshes}
										parentRecursiveRefresh={recursiveRefresh}
										globalModuleStates={[]}
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
										jobId={mod.job}
										{reducedPolling}
										isSubflow
										childFlow
										on:jobsLoaded={(e) => {
											let { force, job } = e.detail
											onJobsLoaded(mod, job, force)
										}}
									/>
								{:else if mod.flow_jobs?.length == 0 && mod.job == '00000000-0000-0000-0000-000000000000'}
									<div class="text-secondary">no subflow (empty loop?)</div>
								{:else}
									<svelte:self
										{globalRefreshes}
										parentRecursiveRefresh={recursiveRefresh}
										{childFlow}
										globalModuleStates={[localModuleStates, ...globalModuleStates]}
										globalDurationStatuses={[localDurationStatuses, ...globalDurationStatuses]}
										render={selected == 'sequence' && render}
										{workspaceId}
										{prefix}
										{subflowParentsGlobalModuleStates}
										{subflowParentsDurationStatuses}
										jobId={mod.job}
										{reducedPolling}
										innerModule={mod.flow_jobs ? job.raw_flow?.modules[i]?.value : undefined}
										flowJobIds={mod.flow_jobs
											? {
													moduleId: mod.id,
													flowJobs: mod.flow_jobs,
													flowJobsSuccess: mod.flow_jobs_success,
													length: mod.iterator?.itered?.length ?? mod.flow_jobs.length,
													branchall: job?.raw_flow?.modules?.[i]?.value?.type == 'branchall'
												}
											: undefined}
										on:jobsLoaded={(e) => {
											let { job, force } = e.detail
											onJobsLoaded(mod, job, force)
										}}
									/>
								{/if}
							{:else}
								<ModuleStatus
									type={mod.type}
									scheduled_for={$localModuleStates?.[mod.id ?? '']?.scheduled_for}
								/>
							{/if}
						</li>
					{/each}
				</ul>
			{:else}
				<div class="p-2 text-tertiary text-sm italic">Empty flow</div>
			{/if}
		</div>
	</div>
	{#if render}
		{#if job.raw_flow && !isListJob}
			<div class="{selected != 'graph' ? 'hidden' : ''} grow mt-4">
				<div class="grid grid-cols-3 border h-full" bind:clientHeight={wrapperHeight}>
					<div class="col-span-2 bg-surface-secondary">
						<div class="flex flex-col">
							{#each Object.values($retryStatus) as count}
								{#if count}
									<span class="text-sm">
										Retry in progress, # of failed attempts: {count}
									</span>
								{/if}
							{/each}
							{#each Object.values($suspendStatus) as count}
								{#if count.nb}
									<span class="text-sm">
										Flow suspended, waiting for {count.nb} events
									</span>
								{/if}
							{/each}
						</div>

						<FlowGraphV2
							{selectedId}
							triggerNode={true}
							download={!hideDownloadInGraph}
							minHeight={wrapperHeight}
							success={jobId != undefined && isSuccess(job?.['success'])}
							flowModuleStates={$localModuleStates}
							bind:expandedSubflows
							on:select={(e) => {
								if (rightColumnSelect != 'node_definition') {
									rightColumnSelect = 'node_status'
								}
								if (typeof e.detail == 'string') {
									if (e.detail == 'Input') {
										selectedNode = 'start'
										stepDetail = undefined
									} else if (e.detail == 'Result') {
										selectedNode = 'end'
										stepDetail = 'end'
									} else {
										const mod = dfs(job?.raw_flow?.modules ?? [], (m) => m).find(
											(m) => m?.id === e?.detail
										)
										stepDetail = mod
										selectedNode = e?.detail
									}
								} else {
									stepDetail = e.detail
									selectedNode = e.detail.id
								}
							}}
							on:selectedIteration={async (e) => {
								let detail = e.detail
								if (detail.manuallySet) {
									let rootJobId = detail.id
									await tick()

									let previousId = $localModuleStates[detail.moduleId]?.selectedForloop
									if (previousId) {
										await globalRefreshes?.[detail.moduleId]?.(true, previousId)
									}

									$localModuleStates[detail.moduleId] = {
										...$localModuleStates[detail.moduleId],
										selectedForloop: detail.id,
										selectedForloopIndex: detail.index,
										selectedForLoopSetManually: true
									}

									await tick()

									await globalRefreshes?.[detail.moduleId]?.(false, rootJobId)
								} else {
									$localModuleStates[detail.moduleId] = {
										...$localModuleStates[detail.moduleId],
										selectedForLoopSetManually: false
									}
								}
							}}
							earlyStop={job.raw_flow?.skip_expr !== undefined}
							cache={job.raw_flow?.cache_ttl !== undefined}
							modules={job.raw_flow?.modules ?? []}
							failureModule={job.raw_flow?.failure_module}
							preprocessorModule={job.raw_flow?.preprocessor_module}
							allowSimplifiedPoll={false}
							{workspace}
						/>
					</div>
					<div
						class="border-l border-tertiary-inverse pt-1 overflow-auto min-h-[700px] flex flex-col h-full"
					>
						<Tabs bind:selected={rightColumnSelect}>
							{#if !hideTimeline}
								<Tab value="timeline"><span class="font-semibold text-md">Timeline</span></Tab>
							{/if}
							<Tab value="node_status"><span class="font-semibold">Node status</span></Tab>
							{#if !hideNodeDefinition}
								<Tab value="node_definition"><span class="font-semibold">Node definition</span></Tab
								>
							{/if}
							{#if Object.keys(job?.flow_status?.user_states ?? {}).length > 0}
								<Tab value="user_states"><span class="font-semibold">User States</span></Tab>
							{/if}
						</Tabs>
						{#if rightColumnSelect == 'timeline'}
							<FlowTimeline
								selfWaitTime={job?.self_wait_time_ms}
								aggregateWaitTime={job?.aggregate_wait_time_ms}
								flowDone={job?.['success'] != undefined}
								bind:this={flowTimeline}
								flowModules={allModulesForTimeline(
									job?.raw_flow?.modules ?? [],
									expandedSubflows ?? {}
								)}
								durationStatuses={localDurationStatuses}
							/>
						{:else if rightColumnSelect == 'node_status'}
							<div class="pt-2 grow flex flex-col">
								{#if selectedNode}
									{@const node = $localModuleStates[selectedNode]}

									{#if selectedNode == 'end'}
										<FlowJobResult
											workspaceId={job?.workspace_id}
											jobId={job?.id}
											filename={job.id}
											loading={job['running']}
											tag={job?.tag}
											noBorder
											col
											result={job['result']}
											logs={job.logs ?? ''}
											durationStates={localDurationStatuses}
											downloadLogs={!hideDownloadLogs}
										/>
									{:else if selectedNode == 'start'}
										{#if job.args}
											<div class="p-2">
												<JobArgs
													id={job.id}
													workspace={job.workspace_id ?? $workspaceStore ?? 'no_w'}
													args={job.args}
												/>
											</div>
										{:else}
											<p class="p-2 text-secondary">No arguments</p>
										{/if}
									{:else if node}
										{#if node.flow_jobs_results}
											<span class="pl-1 text-tertiary"
												>Result of step as collection of all subflows</span
											>
											<div class="overflow-auto max-h-[200px] p-2">
												<DisplayResult
													workspaceId={job?.workspace_id}
													result={node.flow_jobs_results}
													nodeId={selectedNode}
													jobId={job?.id}
													language={job?.language}
												/>
											</div>
											<span class="pl-1 text-tertiary text-lg pt-4">Selected subflow</span>
										{/if}
										<div class="px-2 flex gap-2 min-w-0 w-full">
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
											{#if node.job_id}
												<div class="grow w-full flex flex-row-reverse">
													<a
														class="text-right text-xs"
														rel="noreferrer"
														target="_blank"
														href="{base}/run/{node.job_id ?? ''}?workspace={job?.workspace_id}"
													>
														{truncateRev(node.job_id ?? '', 10)}
													</a>
												</div>
											{/if}
										</div>
										{#if !node.isListJob}
											<div class="px-1 py-1">
												<JobArgs
													id={node.job_id}
													workspace={job.workspace_id ?? $workspaceStore ?? 'no_w'}
													args={node.args}
												/>
											</div>
										{/if}
										<FlowJobResult
											workspaceId={job?.workspace_id}
											jobId={node.job_id}
											noBorder
											loading={node.type != 'Success' && node.type != 'Failure'}
											waitingForExecutor={node.type == 'WaitingForExecutor'}
											refreshLog={node.type == 'InProgress'}
											col
											result={node.result}
											tag={node.tag}
											logs={node.logs}
											durationStates={localDurationStatuses}
											downloadLogs={!hideDownloadLogs}
										/>
									{:else}
										<p class="p-2 text-tertiary italic"
											>The execution of this node has no information attached to it. The job likely
											did not run yet</p
										>
									{/if}
								{:else}<p class="p-2 text-tertiary italic">Select a node to see its details here</p
									>{/if}
							</div>
						{:else if rightColumnSelect == 'node_definition'}
							<FlowGraphViewerStep {stepDetail} />
						{:else if rightColumnSelect == 'user_states'}
							<div class="p-2">
								<JobArgs argLabel="Key" args={job?.flow_status?.user_states ?? {}} />
							</div>
						{/if}
					</div>
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
