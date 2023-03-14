<script lang="ts">
	import { goto } from '$app/navigation'
	import type { Schema } from '$lib/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import { AppService, type CompletedJob } from '$lib/gen'
	import { classNames, defaultIfEmptyString, emptySchema, sendUserToast } from '$lib/utils'
	import { Bug } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInputs, Runnable } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import { computeGlobalContext, eval_like } from './eval'
	import InputValue from './InputValue.svelte'
	import RefreshButton from './RefreshButton.svelte'

	// Component props
	export let id: string
	export let fields: AppInputs
	export let runnable: Runnable
	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let result: any = undefined
	export let forceSchemaDisplay: boolean = false
	export let flexWrap = false
	export let wrapperClass = ''
	export let wrapperStyle = ''
	export let initializing: boolean | undefined = undefined
	export let gotoUrl: string | undefined = undefined
	export let gotoNewTab: boolean | undefined = undefined
	export let render: boolean
	export let recomputable: boolean = false
	export let recomputeIds: string[] = []

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
		state
	} = getContext<AppViewerContext>('AppViewerContext')

	$: autoRefresh && handleAutorefresh()

	if (recomputable) {
		$runnableComponents[id] = async () => {
			await executeComponent(true)
		}
		$runnableComponents = $runnableComponents
	}

	function handleAutorefresh() {
		if (autoRefresh && $worldStore) {
			$runnableComponents[id] = async () => {
				await executeComponent(true)
			}
			executeComponent(true)
		}
	}

	let args: Record<string, any> | undefined = undefined
	let testIsLoading = false
	let runnableInputValues: Record<string, any> = {}
	let executeTimeout: NodeJS.Timeout | undefined = undefined

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
	$: if (JSON.stringify(currentStaticValues) != JSON.stringify(lazyStaticValues)) {
		lazyStaticValues = currentStaticValues
		refreshIfAutoRefresh()
	}

	function refreshIfAutoRefresh() {
		if (autoRefresh) {
			setDebouncedExecute()
		}
	}

	$: fields && (lazyStaticValues = computeStaticValues())
	$: (runnableInputValues || extraQueryParams || args) && testJobLoader && refreshIfAutoRefresh()

	// Test job internal state
	let testJob: CompletedJob | undefined = undefined
	let testJobLoader: TestJobLoader | undefined = undefined

	$: outputs = initOutput($worldStore, id, { result: undefined, loading: false })

	$: outputs?.loading?.set(testIsLoading)
	$: schemaStripped = stripSchema(fields, $stateId)

	function stripSchema(inputs: AppInputs, s: any): Schema {
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

			if (input.type === 'static' && schemaStripped !== undefined) {
				delete schemaStripped.properties[key]
			}

			if (input.type === 'connected' && schemaStripped !== undefined) {
				delete schemaStripped.properties[key]
			}
		})
		return schemaStripped as Schema
	}

	$: disabledArgs = Object.keys(fields ?? {}).reduce(
		(disabledArgsAccumulator: string[], inputName: string) => {
			if (fields[inputName].type === 'static') {
				disabledArgsAccumulator = [...disabledArgsAccumulator, inputName]
			}
			return disabledArgsAccumulator
		},
		[]
	)

	async function executeComponent(noToast = false) {
		if (runnable?.type === 'runnableByName' && runnable.inlineScript?.language === 'frontend') {
			outputs?.loading?.set(true)
			try {
				const r = await eval_like(
					runnable.inlineScript?.content,
					computeGlobalContext($worldStore, id, {}),
					false,
					$state,
					$mode == 'dnd'
				)
				setResult(r)
				$state = $state
			} catch (e) {
				sendUserToast('Error running frontend script: ' + e.message, true)
			}
			outputs?.loading?.set(false)
			return
		}
		if (noBackend) {
			if (!noToast) {
				sendUserToast('This app is not connected to a windmill backend, it is a static preview')
			}
			return
		}
		if (runnable?.type === 'runnableByName' && !runnable.inlineScript) {
			return
		}

		outputs?.loading?.set(true)

		let njob = await testJobLoader?.abstractRun(() => {
			const nonStaticRunnableInputs = {}
			const staticRunnableInputs = {}
			Object.keys(fields ?? {}).forEach((k) => {
				let field = fields[k]
				if (field?.type == 'static' && fields[k]) {
					staticRunnableInputs[k] = field.value
				} else if (field?.type == 'user') {
					nonStaticRunnableInputs[k] = args?.[k]
				} else {
					nonStaticRunnableInputs[k] = runnableInputValues[k]
				}
			})

			const requestBody = {
				args: nonStaticRunnableInputs,
				force_viewer_static_fields: !isEditor ? undefined : staticRunnableInputs
			}

			if (runnable?.type === 'runnableByName') {
				const { inlineScript } = runnable

				if (inlineScript) {
					requestBody['raw_code'] = {
						content: inlineScript.content,
						language: inlineScript.language,
						path: inlineScript.path
					}
				}
			} else if (runnable?.type === 'runnableByPath') {
				const { path, runType } = runnable
				requestBody['path'] = runType !== 'hubscript' ? `${runType}/${path}` : `script/${path}`
			}

			return AppService.executeComponent({
				workspace,
				path: defaultIfEmptyString(appPath, 'newapp'),
				requestBody
			})
		})
		if (njob) {
			$jobs = [{ job: njob, component: id }, ...$jobs]
		}
	}

	export async function runComponent() {
		await executeComponent()
	}

	let lastStartedAt: number = Date.now()

	function recordError(error: string) {
		if (testJob) {
			$errorByComponent[testJob.id] = {
				error: error,
				componentId: id
			}
		}
	}

	function setResult(res: any) {
		outputs.result?.set(res)

		result = res

		const previousJobId = Object.keys($errorByComponent).find(
			(key) => $errorByComponent[key].componentId === id
		)

		if (previousJobId && !result?.error) {
			delete $errorByComponent[previousJobId]
			$errorByComponent = $errorByComponent
		}
		if (gotoUrl && gotoUrl != '' && result?.error == undefined) {
			if (gotoNewTab) {
				window.open(gotoUrl, '_blank')
			} else {
				goto(gotoUrl)
			}
		}

		if (recomputeIds) {
			recomputeIds.map((id) => $runnableComponents?.[id]?.())
		}
	}
	$: result?.error && recordError(result.error)
</script>

{#each Object.entries(fields ?? {}) as [key, v]}
	{#if v.type != 'static' && v.type != 'user'}
		<InputValue {id} input={fields[key]} bind:value={runnableInputValues[key]} />
	{/if}
{/each}

<TestJobLoader
	workspaceOverride={workspace}
	on:done={(e) => {
		if (testJob && outputs) {
			const startedAt = new Date(testJob.started_at).getTime()
			if (startedAt > lastStartedAt) {
				lastStartedAt = startedAt
				setResult(e.detail.result)
			}
		}
	}}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
	bind:this={testJobLoader}
/>

{#if render}
	<div class="h-full flex relative flex-row flex-wrap {wrapperClass}" style={wrapperStyle}>
		{#if schemaStripped && Object.keys(schemaStripped?.properties ?? {}).length > 0 && (autoRefresh || forceSchemaDisplay)}
			<div class="px-2 h-fit min-h-0">
				<SchemaForm
					{flexWrap}
					schema={schemaStripped}
					bind:args
					{disabledArgs}
					shouldHideNoInputs
					noVariablePicker
				/>
			</div>
		{/if}

		{#if !runnable && autoRefresh}
			<Alert type="warning" size="xs" class="mt-2 px-1" title="Missing runnable">
				Please select a runnable
			</Alert>
		{:else if result?.error && $mode === 'preview'}
			<div
				title="Error"
				class={classNames(
					'text-red-500 px-1 text-2xs py-0.5 font-bold w-fit absolute border border-red-500 -bottom-2  shadow left-1/2 transform -translate-x-1/2 z-50 cursor-pointer',
					'bg-red-100/80'
				)}
			>
				<Popover notClickable placement="bottom" popupClass="!bg-white border w-96">
					<Bug size={14} />
					<span slot="text">
						<div class="bg-white">
							<Alert type="error" title="Error during execution">
								<div class="flex flex-col gap-2">
									An error occured, please contact the app author.
									<span class="font-semibold">Job id: {testJob?.id}</span>
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
		{#if !initializing && autoRefresh === true}
			<div class="flex absolute top-1 right-1 z-50">
				<RefreshButton componentId={id} />
			</div>
		{/if}
	</div>
{:else}
	<div class="w-full h-full" />
{/if}
