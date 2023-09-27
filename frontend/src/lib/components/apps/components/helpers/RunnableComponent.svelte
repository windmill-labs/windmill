<script lang="ts">
	import type { Schema } from '$lib/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import LightweightSchemaForm from '$lib/components/LightweightSchemaForm.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { AppService } from '$lib/gen'
	import { classNames, defaultIfEmptyString, emptySchema, sendUserToast } from '$lib/utils'
	import { deepEqual } from 'fast-equals'
	import { Bug } from 'lucide-svelte'
	import { createEventDispatcher, getContext, onDestroy, onMount } from 'svelte'
	import type { AppInputs, Runnable } from '../../inputType'
	import type { Output } from '../../rx'
	import type {
		AppViewerContext,
		CancelablePromise,
		GroupContext,
		InlineScript,
		ListContext
	} from '../../types'
	import { computeGlobalContext, eval_like } from './eval'
	import InputValue from './InputValue.svelte'
	import RefreshButton from './RefreshButton.svelte'
	import { clearErrorByComponentId, selectId } from '../../editor/appUtils'
	import ResultJobLoader from '$lib/components/ResultJobLoader.svelte'
	import { userStore } from '$lib/stores'

	// Component props
	export let id: string
	export let fields: AppInputs
	export let runnable: Runnable
	export let transformer: (InlineScript & { language: 'frontend' }) | undefined
	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let result: any = undefined
	export let forceSchemaDisplay: boolean = false
	export let wrapperClass = ''
	export let wrapperStyle = ''
	export let initializing: boolean | undefined = undefined
	export let render: boolean
	export let outputs: {
		result: Output<any>
		loading: Output<boolean>
		jobId?: Output<any> | undefined
	}
	export let extraKey = ''
	export let recomputeOnInputChanged: boolean = true
	export let loading = false
	export let refreshOnStart: boolean = false
	export let recomputableByRefreshButton: boolean
	export let errorHandledByComponent: boolean = false
	export let hideRefreshButton: boolean = false
	export let hasChildrens: boolean

	const {
		worldStore,
		runnableComponents,
		workspace,
		appPath,
		isEditor,
		jobs,
		noBackend,
		errorByComponent,
		mode,
		stateId,
		state,
		componentControl,
		initialized,
		selectedComponent,
		app,
		connectingInput
	} = getContext<AppViewerContext>('AppViewerContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const rowContext = getContext<ListContext>('RowWrapperContext')
	const groupContext = getContext<GroupContext>('GroupContext')

	const dispatch = createEventDispatcher()

	let donePromise: (() => void) | undefined = undefined

	$runnableComponents = $runnableComponents

	export function setArgs(value: any) {
		args = value
	}
	
	let args: Record<string, any> | undefined = undefined
	let runnableInputValues: Record<string, any> = {}
	let executeTimeout: NodeJS.Timeout | undefined = undefined

	$: outputs.loading?.set(loading)

	function setDebouncedExecute() {
		executeTimeout && clearTimeout(executeTimeout)
		executeTimeout = setTimeout(() => {
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
	let currentStaticValues = lazyStaticValues

	$: fields && (currentStaticValues = computeStaticValues())
	$: if (!deepEqual(currentStaticValues, lazyStaticValues)) {
		lazyStaticValues = currentStaticValues
		refreshIfAutoRefresh('static changed')
	}

	$: (runnableInputValues || extraQueryParams || args) &&
		resultJobLoader &&
		refreshIfAutoRefresh('arg changed')

	$: refreshOn =
		runnable && runnable.type === 'runnableByName' ? runnable.inlineScript?.refreshOn ?? [] : []

	function refreshIfAutoRefresh(_src: string) {
		// console.debug(`Triggering refreshing ${id} because ${_src}`)
		const refreshEnabled =
			autoRefresh && ((recomputeOnInputChanged ?? true) || refreshOn?.length > 0)
		if (refreshEnabled && $initialized.initialized) {
			// console.debug(`Refreshing ${id} because ${_src} (enabled)`)
			setDebouncedExecute()
		}
	}

	// Test job internal state
	let resultJobLoader: ResultJobLoader | undefined = undefined

	let schemaStripped: Schema | undefined =
		autoRefresh || forceSchemaDisplay ? emptySchema() : undefined

	$: (autoRefresh || forceSchemaDisplay) &&
		Object.keys(fields ?? {}).length > 0 &&
		(schemaStripped = stripSchema(fields, $stateId))

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
		while ($jobs.find((j) => j.job === `${prefix}#${nextJobNumber}`)) {
			nextJobNumber++
		}
		return `${prefix}#${nextJobNumber}`
	}

	async function executeComponent(noToast = false, inlineScriptOverride?: InlineScript) {
		console.debug(`Executing ${id}`)
		if (iterContext && $iterContext.disabled) {
			console.debug(`Skipping execution of ${id} because it is part of a disabled list`)
			return
		}
		if (runnable?.type === 'runnableByName' && runnable.inlineScript?.language === 'frontend') {
			loading = true
			try {
				const r = await eval_like(
					runnable.inlineScript?.content,
					computeGlobalContext($worldStore, {
						iter: iterContext ? $iterContext : undefined,
						row: rowContext ? $rowContext : undefined,
						group: groupContext ? $groupContext : undefined
					}),
					false,
					$state,
					$mode == 'dnd',
					$componentControl,
					$worldStore,
					$runnableComponents
				)

				const job = generateNextFrontendJobId()
				await setResult(r, job)
				$state = $state
			} catch (e) {
				sendUserToast(`Error running frontend script ${id}: ` + e.message, true)
				const job = generateNextFrontendJobId()
				await setResult({ error: { message: e.body ?? e.message } }, job)
			}
			loading = false
			donePromise?.()
			return
		} else if (noBackend) {
			if (!noToast) {
				sendUserToast('This app is not connected to a windmill backend, it is a static preview')
			}
			donePromise?.()
			return
		}
		if (runnable?.type === 'runnableByName' && !runnable.inlineScript) {
			return
		}

		if (!resultJobLoader) {
			console.warn('No test job loader')
			return
		}

		try {
			await resultJobLoader?.abstractRun(async () => {
				const nonStaticRunnableInputs = {}
				const staticRunnableInputs = {}
				for (const k of Object.keys(fields ?? {})) {
					let field = fields[k]
					if (field?.type == 'static' && fields[k]) {
						staticRunnableInputs[k] = field.value
					} else if (field?.type == 'user') {
						nonStaticRunnableInputs[k] = args?.[k]
					} else if (field?.type == 'eval' || (field?.type == 'evalv2' && inputValues[k])) {
						nonStaticRunnableInputs[k] = await inputValues[k]?.computeExpr()
					} else {
						nonStaticRunnableInputs[k] = runnableInputValues[k]
					}
				}

				const requestBody = {
					args: nonStaticRunnableInputs,
					component: id,
					force_viewer_static_fields: !isEditor ? undefined : staticRunnableInputs
				}

				if (runnable?.type === 'runnableByName') {
					const { inlineScript } = inlineScriptOverride
						? { inlineScript: inlineScriptOverride }
						: runnable

					if (inlineScript) {
						requestBody['raw_code'] = {
							content: inlineScript.content,
							language: inlineScript.language,
							path: inlineScript.path,
							lock: inlineScript.lock,
							cache_ttl: inlineScript.cache_ttl
						}
					}
				} else if (runnable?.type === 'runnableByPath') {
					const { path, runType } = runnable
					requestBody['path'] = runType !== 'hubscript' ? `${runType}/${path}` : `script/${path}`
				}

				return AppService.executeComponent({
					workspace,
					path: defaultIfEmptyString(appPath, `u/${$userStore?.username ?? 'unknown'}/newapp`),
					requestBody
				})
			})
		} catch (e) {
			updateResult({ error: e.body ?? e.message })
			loading = false
		}
	}

	export async function runComponent() {
		try {
			if (cancellableRun) {
				await cancellableRun()
			} else {
				executeComponent()
			}
		} catch (e) {
			updateResult({ error: e.body ?? e.message })
		}
	}

	function recordError(error: string, jobId: string) {
		$errorByComponent[jobId] = {
			error: error,
			componentId: id
		}
	}

	async function setJobId(jobId: string) {
		outputs.jobId?.set(jobId)
	}

	function recordJob(
		jobId: string,
		result?: string,
		error?: string,
		transformer?: { result?: string; error?: string }
	) {
		const job = {
			...(result ? { result } : {}),
			...(error ? { error } : {}),
			...(transformer ? { transformer } : {}),
			job: jobId,
			component: id
		}

		if (error) {
			recordError(error, jobId)
		} else if (job?.transformer?.error) {
			recordError(job.transformer.error, jobId)
		}

		const njobs = [job, ...$jobs]
		// Only keep the last 100 jobs
		$jobs = $jobs?.length > 100 ? njobs.slice(0, 100) : njobs
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
				const transformerResult = await eval_like(
					transformer.content,
					computeGlobalContext($worldStore, {
						iter: iterContext ? $iterContext : undefined,
						row: rowContext ? $rowContext : undefined,
						result: res
					}),
					false,
					$state,
					$mode == 'dnd',
					$componentControl,
					$worldStore,
					$runnableComponents
				)
				raw.set(transformerResult)
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
		outputs.result?.set(res)
		result = res
	}

	async function setResult(res: any, jobId: string | undefined) {
		dispatch('done')
		const hasRes = res !== undefined && res !== null

		if (!jobId && !hasRes) {
			return
		}

		const errors = getResultErrors(res)

		if (errors) {
			const transformerResult = transformer
				? { error: 'Transformer could not be run because of previous errors' }
				: undefined

			recordJob(jobId!, undefined, errors, transformerResult)
			updateResult(res)
			dispatch('handleError', errors)
			donePromise?.()
			return
		}

		const transformerResult = await runTransformer(res)

		if (transformerResult?.error) {
			recordJob(jobId!, res, undefined, transformerResult)
			updateResult(transformerResult)
			dispatch('handleError', transformerResult.error)
			donePromise?.()
			return
		}

		updateResult(transformerResult ?? res)
		recordJob(jobId!, result, undefined, transformerResult)
		$errorByComponent = clearErrorByComponentId(id, $errorByComponent)

		dispatch('success')
		donePromise?.()
	}

	function handleInputClick(e: CustomEvent) {
		const event = e as unknown as PointerEvent
		!$connectingInput.opened && selectId(event, id, selectedComponent, $app)
	}

	let cancellableRun: ((inlineScript?: InlineScript) => CancelablePromise<void>) | undefined =
		undefined

	onMount(() => {
		console.log('create', id)

		cancellableRun = (inlineScript?: InlineScript) => {
			let rejectCb: (err: Error) => void
			let p: Partial<CancelablePromise<void>> = new Promise<void>((resolve, reject) => {
				rejectCb = reject
				donePromise = resolve

				executeComponent(true, inlineScript).catch(reject)
			})
			p.cancel = () => {
				resultJobLoader?.cancelJob()
				loading = false
				rejectCb(new Error('Canceled'))
			}

			return p as CancelablePromise<void>
		}

		$runnableComponents[id] = {
			autoRefresh: autoRefresh && recomputableByRefreshButton,
			refreshOnStart: refreshOnStart,
			cb: [...($runnableComponents[id]?.cb ?? []), cancellableRun]
		}

		if (!$initialized.initializedComponents.includes(id)) {
			$initialized.initializedComponents = [...$initialized.initializedComponents, id]
		}
	})

	onDestroy(() => {
		$initialized.initializedComponents = $initialized.initializedComponents.filter((c) => c !== id)
		$errorByComponent = clearErrorByComponentId(id, $errorByComponent)
		if ($runnableComponents[id]) {
			$runnableComponents[id] = {
				...$runnableComponents[id],
				cb: $runnableComponents[id].cb.filter((cb) => cb !== cancellableRun)
			}
			$runnableComponents = $runnableComponents
		}
	})

	let lastJobId: string | undefined = undefined

	let inputValues: Record<string, InputValue> = {}
</script>

{#each Object.entries(fields ?? {}) as [key, v] (key)}
	{#if v.type != 'static' && v.type != 'user'}
		<InputValue
			bind:this={inputValues[key]}
			key={key + extraKey}
			{id}
			input={fields[key]}
			bind:value={runnableInputValues[key]}
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
	on:started={(e) => {
		loading = true
		setJobId(e.detail)
		dispatch('started', e.detail)
	}}
	workspaceOverride={workspace}
	on:done={(e) => {
		lastJobId = e.detail.id
		setResult(e.detail.result, e.detail.id)
		loading = false
	}}
	on:doneError={(e) => {
		setResult({ error: e.detail.error }, e.detail.id)
		loading = false
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
		{#if render && (autoRefresh || forceSchemaDisplay) && schemaStripped && Object.keys(schemaStripped?.properties ?? {}).length > 0}
			<div class="px-2 h-fit min-h-0">
				<LightweightSchemaForm
					schema={schemaStripped}
					bind:args
					on:inputClicked={handleInputClick}
				/>
			</div>
		{/if}

		{#if !runnable && autoRefresh}
			<Alert type="warning" size="xs" class="mt-2 px-1" title="Missing runnable">
				Please select a runnable
			</Alert>
		{:else if result?.error && $mode === 'preview' && !errorHandledByComponent}
			<div
				title="Error"
				class={classNames(
					'text-red-500 px-1 text-2xs py-0.5 font-bold w-fit absolute border border-red-500 -bottom-2  shadow left-1/2 transform -translate-x-1/2 z-50 cursor-pointer',
					'bg-red-100/80'
				)}
			>
				<Popover notClickable placement="bottom" popupClass="!bg-surface border w-96">
					<Bug size={14} />
					<span slot="text">
						<div class="bg-surface">
							<Alert type="error" title="Error during execution">
								<div class="flex flex-col gap-2">
									An error occured, please contact the app author.

									{#if lastJobId && $errorByComponent[lastJobId]?.error}
										<div class="font-bold">{$errorByComponent[lastJobId]?.error}</div>
									{/if}
									<a
										href={`/run/${lastJobId}?workspace=${workspace}`}
										class="font-semibold text-red-800 underline"
										target="_blank"
									>
										Job id: {lastJobId}
									</a>
								</div>
							</Alert>
						</div>
					</span>
				</Popover>
			</div>
			<div class="block grow w-full max-h-full border border-red-300 bg-red-50 relative">
				<slot />
			</div>
		{:else}
			<div class="block grow w-full max-h-full">
				<slot />
			</div>
		{/if}
		{#if render && !initializing && autoRefresh === true && !hideRefreshButton}
			<div class="flex absolute top-1 right-1 z-50">
				<RefreshButton {loading} componentId={id} />
			</div>
		{/if}
	</div>
{/if}
