<script lang="ts">
	import type { Schema } from '$lib/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { type ExecuteComponentData } from '$lib/gen'
	import { classNames, defaultIfEmptyString, emptySchema, sendUserToast } from '$lib/utils'
	import { deepEqual } from 'fast-equals'
	import { Bug } from 'lucide-svelte'
	import { createEventDispatcher, getContext, onDestroy, onMount, untrack } from 'svelte'
	import type { AppInputs, Runnable } from '../../inputType'
	import type { Output } from '../../rx'
	import type {
		AppEditorContext,
		AppViewerContext,
		CancelablePromise,
		GroupContext,
		InlineScript,
		ListContext
	} from '../../types'
	import { computeGlobalContext, eval_like } from './eval'
	import InputValue from './InputValue.svelte'
	import { collectOneOfFields, selectId } from '../../editor/appUtils'
	import ResultJobLoader from '$lib/components/ResultJobLoader.svelte'
	import { userStore } from '$lib/stores'
	import { get } from 'svelte/store'
	import RefreshButton from '$lib/components/apps/components/helpers/RefreshButton.svelte'
	import { ctxRegex } from '../../utils'
	import { computeWorkspaceS3FileInputPolicy } from '../../editor/appUtilsS3'
	import { executeRunnable } from './executeRunnable'
	import SchemaForm from '$lib/components/SchemaForm.svelte'

	interface Props {
		// Component props
		id: string
		fields: AppInputs
		runnable: Runnable
		transformer: (InlineScript & { language: 'frontend' }) | undefined
		extraQueryParams?: Record<string, any>
		autoRefresh?: boolean
		result?: any
		forceSchemaDisplay?: boolean
		wrapperClass?: string
		wrapperStyle?: string
		render: boolean
		outputs: {
			result: Output<any>
			loading: Output<boolean>
			jobId?: Output<any> | undefined
		}
		extraKey?: string
		initializing?: boolean
		recomputeOnInputChanged?: boolean
		loading?: boolean
		refreshOnStart?: boolean
		recomputableByRefreshButton: boolean
		errorHandledByComponent?: boolean
		hideRefreshButton?: boolean
		hasChildrens: boolean
		allowConcurentRequests?: boolean
		noInitialize?: boolean
		overrideCallback?: (() => CancelablePromise<void>) | undefined
		overrideAutoRefresh?: boolean
		replaceCallback?: boolean
		children?: import('svelte').Snippet
	}

	let {
		id,
		fields,
		runnable,
		transformer,
		extraQueryParams = {},
		autoRefresh = true,
		result = $bindable(undefined),
		forceSchemaDisplay = false,
		wrapperClass = '',
		wrapperStyle = '',
		render,
		outputs,
		extraKey = '',
		initializing = false,
		recomputeOnInputChanged = true,
		loading = $bindable(false),
		refreshOnStart = false,
		recomputableByRefreshButton,
		errorHandledByComponent = false,
		hideRefreshButton = false,
		hasChildrens,
		allowConcurentRequests = false,
		noInitialize = false,
		overrideCallback = undefined,
		overrideAutoRefresh = false,
		replaceCallback = false,
		children
	}: Props = $props()

	const {
		worldStore,
		runnableComponents,
		workspace,
		appPath,
		isEditor,
		jobs,
		jobsById,
		noBackend,
		errorByComponent,
		mode,
		stateId,
		state: stateStore,
		componentControl,
		initialized,
		selectedComponent,
		app,
		connectingInput,
		bgRuns,
		recomputeAllContext
	} = getContext<AppViewerContext>('AppViewerContext')
	const editorContext = getContext<AppEditorContext>('AppEditorContext')

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const rowContext = getContext<ListContext>('RowWrapperContext')
	const groupContext = getContext<GroupContext>('GroupContext')

	const dispatch = createEventDispatcher()

	$runnableComponents = $runnableComponents

	export function setArgs(value: any) {
		args = value
	}

	let args: Record<string, any> | undefined = $state(undefined)
	let runnableInputValues: Record<string, any> = $state({})
	let executeTimeout: NodeJS.Timeout | undefined = undefined

	function setDebouncedExecute() {
		executeTimeout && clearTimeout(executeTimeout)
		executeTimeout = setTimeout(() => {
			console.debug('debounce execute')
			executeComponent(true)
		}, 200)
	}

	function computeStaticValues() {
		return Object.entries(fields ?? {})
			.filter(([k, v]) => v.type == 'static')
			.map(([name, field]) => {
				return [name, field['value']]
			})
	}

	let lazyStaticValues = computeStaticValues()
	let currentStaticValues = $state(lazyStaticValues)

	let isBg = id.startsWith('bg_')

	function refreshOnStaticChange() {
		if (!deepEqual(currentStaticValues, lazyStaticValues)) {
			lazyStaticValues = currentStaticValues
			refreshIfAutoRefresh('static changed')
		}
	}

	// $: sendUserToast('args' + JSON.stringify(runnableInputValues) + Boolean(extraQueryParams) || args)
	// $: console.log(runnableInputValues)
	let firstRefresh = true

	function refreshIfAutoRefresh(src: 'arg changed' | 'static changed') {
		// console.log(
		// 	'refreshIfAutoRefresh',
		// 	src,
		// 	id,
		// 	iterContext ? $iterContext : undefined,
		// 	$rowContext ? $rowContext : undefined,
		// 	firstRefresh
		// )
		if (firstRefresh) {
			firstRefresh = false
			if (
				src == 'arg changed' &&
				args == undefined &&
				result != undefined &&
				Object.keys(runnableInputValues ?? {}).length == 0 &&
				Object.keys(extraQueryParams ?? {}).length == 0
			) {
				console.log('skipping refresh because first refresh')
				return
			}
		}

		// console.debug(`Triggering refreshing ${id} because ${_src}`)
		const refreshEnabled =
			autoRefresh && ((recomputeOnInputChanged ?? true) || refreshOn?.length > 0)
		if (refreshEnabled && $initialized.initialized) {
			// console.debug(`Refreshing ${id} because ${_src} (enabled)`)
			setDebouncedExecute()
		}
	}

	let schemaForm: SchemaForm | undefined = $state()

	export function invalidate(key: string, error: string) {
		schemaForm?.invalidate(key, error)
	}

	export function validate(key: string) {
		schemaForm?.validate(key)
	}

	export function validateAll() {
		schemaForm?.validateAll()
	}

	// Test job internal state
	let resultJobLoader: ResultJobLoader | undefined = $state(undefined)

	let schemaStripped: Schema | undefined = $state(
		autoRefresh || forceSchemaDisplay ? emptySchema() : undefined
	)

	function stripSchema(inputs: AppInputs, s: any): Schema {
		if (inputs === undefined) {
			return emptySchema()
		}
		let schema =
			runnable?.type == 'runnableByName' ? runnable.inlineScript?.schema : runnable?.schema
		try {
			schemaStripped = JSON.parse(JSON.stringify(schema))
		} catch (e) {
			console.warn('Error loading schema')
			return emptySchema()
		}
		// schema.order = Object.keys(fields)
		// console.log(schema.order)

		// Remove hidden static inputs
		Object.keys(inputs ?? {}).forEach((key: string) => {
			const input = inputs[key]

			if (
				['static', 'eval', 'connected', 'evalv2'].includes(input.type) &&
				schemaStripped !== undefined &&
				schemaStripped.properties
			) {
				delete schemaStripped.properties[key]
			}
		})
		return schemaStripped as Schema
	}

	function generateNextFrontendJobId() {
		const prefix = 'Frontend: '
		let nextJobNumber = 1
		while ($jobsById[`${prefix}#${nextJobNumber}`]) {
			nextJobNumber++
		}
		return `${prefix}#${nextJobNumber}`
	}

	function addJob(jobId: string) {
		$jobsById[jobId] = {
			component: id,
			job: jobId,
			created_at: Date.now(),
			started_at: jobId.startsWith('Frontend') ? Date.now() : undefined
		}
		jobs.update((jobs) => {
			const njobs = [...jobs, jobId]
			// Only keep the last 100 jobs
			if (njobs?.length > 100) {
				while (njobs?.length > 100) {
					delete $jobsById[njobs.shift()!]
				}
			}
			return njobs
		})
	}

	async function executeComponent(
		noToast = false,
		inlineScriptOverride?: InlineScript,
		setRunnableJobEditorPanel?: boolean,
		dynamicArgsOverride?: Record<string, any>,
		callbacks?: Callbacks
	): Promise<string | undefined> {
		let jobId: string | undefined
		console.debug(`Executing ${id}`)
		if (iterContext && $iterContext.disabled) {
			callbacks?.done({})
			console.debug(`Skipping execution of ${id} because it is part of a disabled list`)
			return
		}

		if (runnable?.type === 'runnableByName' && runnable.inlineScript?.language === 'frontend') {
			loading = true

			let job: string | undefined
			if (isEditor) {
				job = generateNextFrontendJobId()
				addJob(job)
			}
			console.log('Frontend job started', id)

			let r: any
			try {
				r = await eval_like(
					runnable.inlineScript?.content,
					computeGlobalContext($worldStore, id, {
						iter: iterContext ? $iterContext : undefined,
						row: rowContext ? $rowContext : undefined,
						group: groupContext ? get(groupContext.context) : undefined
					}),
					$stateStore,
					isEditor,
					$componentControl,
					$worldStore,
					$runnableComponents,
					true,
					groupContext?.id,
					get(recomputeAllContext)?.onRefresh
				)

				await setResult(r, job)
				$stateStore = $stateStore
			} catch (e) {
				let additionalInfo = ''
				if (
					e.message.includes('Maximum call stack size exceeded') ||
					e.message.includes('too much recursion')
				) {
					additionalInfo =
						'This is likely due to a call to globalRecompute() in the frontend script. Please check your script for circular recomputes and disable the "Run on start and app refresh" toggle.'
				}
				sendUserToast(`Error running frontend script ${id}: ` + e.message + additionalInfo, true)
				r = { error: { message: (e.body ?? e.message) + additionalInfo } }
				await setResult(r, job)
			}
			loading = false
			callbacks?.done(r)
			if (setRunnableJobEditorPanel && editorContext) {
				editorContext.runnableJobEditorPanel.update((p) => {
					return {
						...p,
						frontendJobs: { ...p.frontendJobs, [id]: r }
					}
				})
			}
			return
		} else if (noBackend) {
			if (!noToast) {
				sendUserToast('This app is not connected to a windmill backend, it is a static preview')
			}
			callbacks?.done({})
			return
		}
		if (runnable?.type === 'runnableByName' && !runnable.inlineScript) {
			callbacks?.done({})
			return
		}

		if (!resultJobLoader) {
			console.warn('No test job loader')
			callbacks?.done({})
			return
		}

		try {
			jobId = await resultJobLoader?.abstractRun(async () => {
				const uuid = await executeRunnable(
					runnable,
					workspace,
					$app.version,
					$userStore?.username,
					$appPath,
					id,
					await buildRequestBody(dynamicArgsOverride),
					inlineScriptOverride
				)
				if (isEditor) {
					addJob(uuid)
				}
				return uuid
			}, callbacks)

			if (setRunnableJobEditorPanel && editorContext) {
				editorContext.runnableJobEditorPanel.update((p) => {
					return {
						...p,
						jobs: { ...p.jobs, [id]: jobId as string }
					}
				})
			}
			return jobId
		} catch (e) {
			let error = e.body ?? e.message
			updateResult({ error })
			$errorByComponent[id] = { error }

			callbacks?.done({ error })
			sendUserToast(error, true)
			loading = false
		}
	}
	type Callbacks = { done: (x: any) => void; cancel: () => void; error: (e: any) => void }

	export async function buildRequestBody(dynamicArgsOverride: Record<string, any> | undefined) {
		const nonStaticRunnableInputs = dynamicArgsOverride ?? {}
		const staticRunnableInputs = {}
		const allowUserResources: string[] = []
		for (const k of Object.keys(fields ?? {})) {
			let field = fields[k]
			if (field?.type == 'static' && fields[k]) {
				if (isEditor) {
					staticRunnableInputs[k] = field.value
				}
			} else if (field?.type == 'user') {
				nonStaticRunnableInputs[k] = args?.[k]
				if (isEditor && field.allowUserResources) {
					allowUserResources.push(k)
				}
			} else if (field?.type == 'eval' || (field?.type == 'evalv2' && inputValues[k])) {
				const ctxMatch = field?.expr?.match(ctxRegex)
				if (ctxMatch) {
					nonStaticRunnableInputs[k] = '$ctx:' + ctxMatch[1]
				} else {
					nonStaticRunnableInputs[k] = await inputValues[k]?.computeExpr()
				}
				if (isEditor && field?.type == 'evalv2' && field.allowUserResources) {
					allowUserResources.push(k)
				}
			} else {
				if (isEditor && field?.type == 'connected' && field.allowUserResources) {
					allowUserResources.push(k)
				}
				nonStaticRunnableInputs[k] = runnableInputValues[k]
			}
		}

		const oneOfRunnableInputs = isEditor ? collectOneOfFields(fields, $app) : {}

		const requestBody: ExecuteComponentData['requestBody'] = {
			args: nonStaticRunnableInputs,
			component: id,
			force_viewer_static_fields: !isEditor ? undefined : staticRunnableInputs,
			force_viewer_one_of_fields: !isEditor ? undefined : oneOfRunnableInputs,
			force_viewer_allow_user_resources: !isEditor ? undefined : allowUserResources
		}
		return requestBody
	}

	export async function runComponent(
		noToast = true,
		inlineScriptOverride?: InlineScript,
		setRunnableJobEditorPanel?: boolean,
		dynamicArgsOverride?: Record<string, any>,
		callbacks?: Callbacks
	): Promise<string | undefined> {
		try {
			if (cancellableRun && !dynamicArgsOverride) {
				await cancellableRun()
			} else {
				console.log('Run component', id)
				return await executeComponent(
					noToast,
					inlineScriptOverride,
					setRunnableJobEditorPanel,
					dynamicArgsOverride,
					callbacks
				)
			}
		} catch (e) {
			let error = e?.body ?? e?.message
			updateResult({ error })
			$errorByComponent[id] = { error }
		}
	}

	async function setJobId(jobId: string) {
		outputs.jobId?.set(jobId)
	}

	function recordJob(
		jobId?: string,
		result?: any,
		jobError?: string,
		transformer?: { result?: string; error?: string }
	) {
		const error = jobError ?? JSON.stringify(transformer?.error, null, 4)

		if (isEditor && jobId) {
			const oldJob = $jobsById[jobId]

			const job = {
				...oldJob,
				...(result ? { result } : {}),
				...(transformer ? { transformer } : {}),
				error,
				duration_ms: oldJob?.started_at ? Date.now() - oldJob?.started_at : 1
			}

			$jobsById[jobId] = job
		}

		if (error) {
			$errorByComponent[id] = { id: jobId, error }
		} else {
			delete $errorByComponent[id]
			$errorByComponent = $errorByComponent
		}
	}

	function getResultErrors(result: any | any[]): string | undefined {
		const errorAsArray = Array.isArray(result) ? result.flat() : [result]
		const hasErrors = errorAsArray.some((r) => r?.error)

		if (!hasErrors) {
			return undefined
		}

		return errorAsArray
			.map((r) => r?.error?.message)
			.filter(Boolean)
			.join('\n')
	}

	async function runTransformer(res) {
		if (transformer) {
			try {
				let raw = $worldStore.newOutput(id, 'raw', res)
				raw.set(res)
				const transformerResult = await eval_like(
					transformer.content,
					computeGlobalContext($worldStore, id, {
						iter: iterContext ? $iterContext : undefined,
						row: rowContext ? $rowContext : undefined,
						group: groupContext ? get(groupContext.context) : undefined,
						result: res
					}),
					$stateStore,
					isEditor,
					$componentControl,
					$worldStore,
					$runnableComponents,
					true,
					groupContext?.id,
					get(recomputeAllContext)?.onRefresh
				)
				return transformerResult
			} catch (err) {
				return {
					error: {
						name: 'TransformerError',
						message: 'An error occured in the transformer',
						stack: err.message
					}
				}
			}
		}
	}

	function updateResult(res) {
		outputs.result?.set($state.snapshot(res))
		result = res
	}

	async function setResult(res: any, jobId: string | undefined) {
		dispatch('resultSet', res)
		const errors = getResultErrors(res)

		if (errors) {
			const transformerResult = transformer
				? { error: 'Transformer could not be run because of previous errors' }
				: undefined

			recordJob(jobId, errors, errors, transformerResult)
			updateResult(res)
			dispatch('handleError', errors)
			// callbacks?.done(res)
			return
		}

		const transformerResult = await runTransformer(res)

		if (transformerResult && editorContext && get(editorContext.runnableJobEditorPanel)?.focused) {
			editorContext.runnableJobEditorPanel.update((p) => {
				return {
					...p,
					frontendJobs: { ...p.frontendJobs, [id + '_transformer']: transformerResult }
				}
			})
		}

		if (transformerResult?.error) {
			recordJob(jobId, res, undefined, transformerResult)
			updateResult(transformerResult)
			dispatch('handleError', transformerResult.error)
			// callbacks?.done(res)
			return
		}

		updateResult(transformerResult ?? res)
		recordJob(jobId, result, undefined, transformerResult)
		delete $errorByComponent[id]

		dispatch('success', result)
		// callbacks?.done(res)
	}

	function handleInputClick(e: CustomEvent) {
		const event = e as unknown as PointerEvent
		!$connectingInput.opened && selectId(event, id, selectedComponent, $app)
	}

	let cancellableRun: ((inlineScript?: InlineScript) => CancelablePromise<void>) | undefined =
		undefined

	onMount(() => {
		if (overrideCallback) {
			cancellableRun = overrideCallback
		} else {
			cancellableRun = (inlineScript?: InlineScript, setRunnableJobEditorPanel?: boolean) => {
				let rejectCb: (err: Error) => void
				let p: Partial<CancelablePromise<any>> = new Promise<any>((resolve, reject) => {
					dispatch('recompute')
					rejectCb = reject
					executeComponent(true, inlineScript, setRunnableJobEditorPanel, undefined, {
						done: (x) => {
							resolve(x)
						},
						cancel: () => {
							reject()
						},
						error: (e) => {
							console.error(e)
							reject(e)
						}
					}).catch(reject)
				})
				p.cancel = () => {
					resultJobLoader?.cancelJob()
					loading = false
					rejectCb(new Error('Canceled'))
				}

				return p as CancelablePromise<void>
			}
		}

		const nautoRefresh = (autoRefresh && recomputableByRefreshButton) || overrideAutoRefresh
		if (replaceCallback) {
			$runnableComponents[id] = {
				autoRefresh: nautoRefresh,
				refreshOnStart: refreshOnStart,
				cb: [cancellableRun]
			}
		} else {
			$runnableComponents[id] = {
				autoRefresh: nautoRefresh,
				refreshOnStart: refreshOnStart,
				cb: [...($runnableComponents[id]?.cb ?? []), cancellableRun]
			}
		}

		if (!noInitialize && !$initialized.initializedComponents.includes(id)) {
			$initialized.initializedComponents = [...$initialized.initializedComponents, id]
		}
		// console.log(initializing, $initialized.initialized, refreshOnStart)
		if (initializing && $initialized.initialized && (refreshOnStart || nautoRefresh)) {
			setDebouncedExecute()
		}
	})

	onDestroy(() => {
		$initialized.initializedComponents = $initialized.initializedComponents.filter((c) => c !== id)
		delete $errorByComponent[id]

		if ($runnableComponents[id]) {
			$runnableComponents[id] = {
				...$runnableComponents[id],
				cb: $runnableComponents[id].cb.filter((cb) => cb !== cancellableRun)
			}
			$runnableComponents = $runnableComponents
		}
	})

	let lastJobId: string | undefined = $state(undefined)

	let inputValues: Record<string, InputValue> = $state({})

	function updateBgRuns(loading: boolean) {
		if (loading) {
			bgRuns.update((runs) => [...runs, id])
		} else {
			bgRuns.update((runs) => runs.filter((r) => r !== id))
		}
	}

	function getError(obj: any) {
		try {
			if (obj?.error) {
				return obj.error
			}
			return undefined
		} catch (e) {
			console.error('Error accessing error from result', e)
			return undefined
		}
	}

	function computeS3ForceViewerPolicies() {
		if (!isEditor) {
			return undefined
		}
		const policy = computeWorkspaceS3FileInputPolicy()
		return policy
	}
	$effect(() => {
		outputs.loading?.set(loading)
	})
	$effect(() => {
		isBg && untrack(() => updateBgRuns(loading))
	})
	$effect(() => {
		fields && untrack(() => (currentStaticValues = computeStaticValues()))
	})
	$effect(() => {
		currentStaticValues && untrack(() => refreshOnStaticChange())
	})
	$effect(() => {
		if (runnableInputValues && typeof runnableInputValues === 'object') {
			for (const key in runnableInputValues) {
				runnableInputValues[key]
			}
		}
		if (extraQueryParams && typeof extraQueryParams === 'object') {
			for (const key in extraQueryParams) {
				extraQueryParams[key]
			}
		}
		if (args && typeof args === 'object') {
			for (const key in args) {
				args[key]
			}
		}
		;(runnableInputValues || extraQueryParams || args) &&
			resultJobLoader &&
			untrack(() => refreshIfAutoRefresh('arg changed'))
	})
	let ignoreFirst = true
	$effect(() => {
		runnableInputValues && !ignoreFirst && dispatch('argsChanged')
		ignoreFirst = false
	})
	let refreshOn = $derived(
		runnable && runnable.type === 'runnableByName' ? (runnable.inlineScript?.refreshOn ?? []) : []
	)
	$effect(() => {
		;(autoRefresh || forceSchemaDisplay) &&
			Object.keys(fields ?? {}).length > 0 &&
			untrack(() => (schemaStripped = stripSchema(fields, $stateId)))
	})
</script>

{#each Object.entries(fields ?? {}) as [key, v] (key)}
	{#if v.type != 'static' && v.type != 'user'}
		<InputValue
			bind:this={inputValues[key]}
			key={key + extraKey}
			{id}
			input={fields[key]}
			bind:value={runnableInputValues[key]}
			onDemandOnly={v.onDemandOnly}
		/>
	{/if}
{/each}

{#if runnable?.type == 'runnableByName' && runnable.inlineScript?.language == 'frontend'}
	{#each runnable.inlineScript.refreshOn ?? [] as { id: tid, key } (`${tid}-${key}`)}
		{@const fkey = `${tid}-${key}${extraKey}`}
		<InputValue
			{id}
			key={fkey}
			input={{ type: 'connected', connection: { componentId: tid, path: key }, fieldType: 'any' }}
			bind:value={runnableInputValues[fkey]}
		/>
	{/each}
{/if}

<ResultJobLoader
	{allowConcurentRequests}
	{isEditor}
	on:started={(e) => {
		console.log('started', e.detail)
		loading = true
		setJobId(e.detail)
		dispatch('started', e.detail)
	}}
	workspaceOverride={workspace}
	on:done={(e) => {
		lastJobId = e.detail.id
		setResult(e.detail.result, e.detail.id)
		loading = false
		dispatch('done', { id: e.detail.id, result: e.detail.result })
	}}
	on:cancel={(e) => {
		let jobId = e.detail
		console.debug('cancel', jobId)
		let job = $jobsById[jobId]
		if (job && job.created_at && !job.duration_ms) {
			$jobsById[jobId] = {
				...job,
				started_at: job.started_at ?? Date.now(),
				duration_ms: Date.now() - (job.started_at ?? job.created_at)
			}
		}
		dispatch('cancel', { id: e.detail })
	}}
	on:running={(e) => {
		let jobId = e.detail
		let job = $jobsById[jobId]
		if (job && !job.started_at) {
			$jobsById[jobId] = { ...job, started_at: Date.now() }
		}
	}}
	on:doneError={(e) => {
		setResult({ error: e.detail.error }, e.detail.id)
		loading = false
		dispatch('doneError', { id: e.detail.id, result: e.detail.result })
	}}
	bind:this={resultJobLoader}
/>

{#if render || hasChildrens}
	<div
		class="h-full flex relative flex-row flex-wrap {wrapperClass} {render
			? 'visible'
			: 'invisible h-0 overflow-hidden'}"
		style={wrapperStyle}
	>
		<!-- {Object.keys(schemaStripped?.properties ?? {}).length > 0} -->
		{#if render && (autoRefresh || forceSchemaDisplay) && schemaStripped && Object.keys(schemaStripped?.properties ?? {}).length > 0}
			<div class="px-2 h-fit min-h-0">
				<SchemaForm
					noVariablePicker
					onlyMaskPassword
					schema={schemaStripped}
					appPath={defaultIfEmptyString($appPath, `u/${$userStore?.username ?? 'unknown'}/newapp`)}
					{computeS3ForceViewerPolicies}
					{workspace}
					bind:this={schemaForm}
					bind:args
					on:inputClicked={handleInputClick}
				/>
			</div>
		{/if}

		{#if !runnable && autoRefresh}
			<Alert type="warning" size="xs" class="mt-2 px-1" title="Missing runnable">
				Please select a runnable
			</Alert>
		{:else if getError(result) && $mode === 'preview' && !errorHandledByComponent}
			<div
				title="Error"
				class={classNames(
					'text-red-500 px-1 text-2xs py-0.5 font-bold w-fit absolute border border-red-500 -bottom-2  shadow left-1/2 transform -translate-x-1/2 z-50 cursor-pointer'
				)}
			>
				<Popover notClickable placement="bottom" popupClass="!bg-surface border w-96">
					<Bug size={14} />
					{#snippet text()}
						<span>
							<div class="bg-surface">
								<Alert type="error" title="Error during execution">
									<div class="flex flex-col gap-2 overflow-auto">
										An error occured, please contact the app author.

										{#if $errorByComponent?.[id]?.error}
											<div class="font-bold">{$errorByComponent[id].error}</div>
										{/if}
										{#if lastJobId}
											<a
												href={`/run/${lastJobId}?workspace=${workspace}`}
												class="font-semibold text-red-800 underline"
												target="_blank"
											>
												Job id: {lastJobId}
											</a>
										{/if}
									</div>
								</Alert>
							</div>
						</span>
					{/snippet}
				</Popover>
			</div>
			<div class="block grow w-full max-h-full border border-red-30 relative">
				{@render children?.()}
			</div>
		{:else}
			<div class="block grow w-full max-h-full">
				{@render children?.()}
			</div>
		{/if}

		{#if render && autoRefresh === true && !hideRefreshButton}
			<div class="flex absolute top-1 right-1 z-50 app-component-refresh-btn">
				<RefreshButton {loading} {id} />
			</div>
		{/if}
	</div>
{/if}
