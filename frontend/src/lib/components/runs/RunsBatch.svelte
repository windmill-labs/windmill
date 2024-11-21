<script lang="ts">
	import {
		JobService,
		ScriptService,
		type Job,
		type RunScriptByHashData,
		type Script,
		type ScriptArgs
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { RefreshCw } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import RunForm from '../RunForm.svelte'
	import { Tab, TabContent, Tabs } from '../common'
	import { deepEqual } from 'fast-equals'

	type ScriptJob = Job & { job_kind: 'script'; script_hash: string }
	type RunnableScript = {
		prior: ScriptJob
		script: Script
		newArgs: ScriptArgs | undefined
	}
	type ScriptArgsMap = {
		[hash: 'common' | string]: {
			args: ScriptArgs | undefined
			schema: { [key: string]: unknown }
		}
	}
	type RichScriptArgs = {
		args: ScriptArgs | undefined
		schema: { [key: string]: unknown }
		hash: 'common' | string
	}

	function isScriptJob(job: Job): job is ScriptJob {
		return job.job_kind === 'script' && job.script_hash !== undefined
	}

	export let jobs: Job[] | undefined
	export let selectedIds: string[]

	let runnableScripts: RunnableScript[] = []
	let factorisedScriptArgs: RichScriptArgs[] = []
	let tab: 'common' | string = 'common'
	let isLoading = false

	let runScript = () => {}
	$: handleNewJobs(jobs, selectedIds)

	async function handleNewJobs(jobs: Job[] | undefined, selectedIds: string[]) {
		console.time('handleNewJobs')
		if (jobs === undefined || !Array.isArray(jobs)) {
			return
		}

		isLoading = true
		const selectedIdsSet = new Set(selectedIds)
		const selectedJobs = jobs.filter((job) => selectedIdsSet.has(job.id))

		const processedJobIds = new Set(runnableScripts.map((s) => s.prior.id))
		const newJobs = selectedJobs.filter((job) => !processedJobIds.has(job.id))

		console.time('jobsToRunnableScripts')
		const newRunnableScripts = await jobsToRunnableScripts(newJobs)

		console.timeEnd('jobsToRunnableScripts')

		runnableScripts = selectedJobs
			.map((job) => [...runnableScripts, ...newRunnableScripts].find((s) => s.prior.id === job.id))
			.filter((s) => s !== undefined)

		console.log('runnableScripts updated to', runnableScripts)
		factorisedScriptArgs = getScriptArgsMap(runnableScripts)
		console.log('scriptAegsMap updated to', factorisedScriptArgs)
		console.timeEnd('handleNewJobs')

		isLoading = false
	}

	async function jobsToRunnableScripts(jobs: Job[]): Promise<RunnableScript[]> {
		if (jobs.length === 0) {
			return []
		}
		const scriptJobs = jobs.filter(isScriptJob)
		const scriptHashes = [...new Set(scriptJobs.map((job) => job.script_hash))]

		const scriptArgsPromises = scriptJobs.map((job) =>
			JobService.getJobArgs({ id: job.id, workspace: $workspaceStore! })
		)
		const scriptPromises = scriptHashes.map((scriptHash) =>
			ScriptService.getScriptByHash({
				workspace: $workspaceStore!,
				hash: scriptHash
			})
		)
		console.time('promises')
		const results = await Promise.allSettled([...scriptArgsPromises, ...scriptPromises])
		console.timeEnd('promises')
		const scriptArgsResults = results.slice(0, scriptArgsPromises.length)
		const scriptResults = results.slice(scriptArgsPromises.length)

		const priorArgs = scriptArgsResults
			.filter((result) => result.status === 'fulfilled')
			.map((p) => p.value) as ScriptArgs[]
		const scriptsArr = scriptResults
			.filter((result) => result.status === 'fulfilled')
			.map((p) => p.value) as Script[]

		const scripts = scriptHashes.reduce((acc, scriptHash, i) => {
			acc[scriptHash] = scriptsArr[i]
			return acc
		}, {} as { [script_hash: string]: Script })

		return scriptJobs.map((job, i) => ({
			prior: { ...job, args: priorArgs[i] },
			script: scripts[job.script_hash],
			newArgs: structuredClone(priorArgs[i])
		}))
	}

	function getScriptArgsMap(runnables: RunnableScript[]): RichScriptArgs[] {
		const scriptArgsMap: ScriptArgsMap = {
			common: { schema: { properties: {}, type: 'object' }, args: {} }
		}
		const allArgNames: Set<string> = new Set()

		if (runnables.length === 0) return []

		if (runnables.some((r) => r.script.schema?.$schema !== runnables[0].script.schema?.$schema)) {
			console.warn('Warning: a script with a different version of JSON schema has been found.')
		} else {
			scriptArgsMap.common.schema.$schema = runnables[0].script.schema?.$schema
		}

		for (const runnable of runnables) {
			const schema = runnable.script.schema
			if (schema === undefined || schema.type !== 'object') continue

			Object.keys(schema.properties as Record<string, string>).forEach((argName) => {
				allArgNames.add(argName)
			})
		}

		const commonArgs: Record<string, any> = {}
		const commonArgsProperties: Record<string, any> = {}
		const commonRequired: string[] = []

		for (const argName of allArgNames) {
			let isCommon = true
			let commonArgProperties: { type: string } | undefined = undefined
			let argValue: ScriptArgs[string] | undefined = undefined
			let isRequired = false

			for (const runnable of runnables) {
				const schema = runnable.script.schema
				if (schema === undefined || schema.type !== 'object') continue

				const argProperties = (schema.properties as Record<string, any>)[argName]

				if (!argProperties) {
					isCommon = false
					break
				}

				if (commonArgProperties === undefined) {
					commonArgProperties = argProperties
					argValue = runnable.prior.args![argName]
				} else {
					if (!deepEqual(commonArgProperties, argProperties)) {
						isCommon = false
						break
					}
				}

				if (
					!isRequired &&
					Array.isArray(schema.required) &&
					schema.required.indexOf(argName) > -1
				) {
					isRequired = true
				}
			}

			if (isCommon) {
				commonArgs[argName] = argValue
				commonArgsProperties[argName] = commonArgProperties

				if (isRequired) {
					commonRequired.push(argName)
				}
			}
		}

		scriptArgsMap.common.args = commonArgs
		scriptArgsMap.common.schema.properties = commonArgsProperties

		if (commonRequired.length > 0) {
			scriptArgsMap.common.schema.required = commonRequired
		}

		for (const runnable of runnables) {
			if (runnable.script.schema === undefined) continue

			const specificArgs: Record<string, any> = {}
			const scriptSchema = runnable.script.schema as Record<string, any>

			for (const argName in runnable.prior.args) {
				if (argName in scriptSchema) {
					specificArgs[argName] = runnable.prior.args[argName]
				}
			}

			scriptArgsMap[runnable.script.hash] = {
				schema: scriptSchema,
				args: specificArgs
			}
		}

		return Object.entries(scriptArgsMap).map(([hash, props]) => ({ ...props, hash }))
	}

	async function runImmediatelyPriorArgs() {
		console.log('clicked on run', runnableScripts)

		const payloads: RunScriptByHashData[] = runnableScripts.map((newJob) => ({
			hash: newJob.prior.script_hash,
			requestBody: newJob.prior.args === undefined ? {} : newJob.prior.args,
			workspace: $workspaceStore!
		}))
		console.log('payloads', payloads)

		const promises = payloads.map((payload) => JobService.runScriptByHash(payload))
		const results = await Promise.allSettled(promises)
		console.log('results', results)
		return results
	}

	async function runImmediatelyNewArgs() {
		console.log('clicked on run', runnableScripts)

		const payloads: RunScriptByHashData[] = runnableScripts.map((newJob) => ({
			hash: newJob.prior.script_hash,
			requestBody: newJob.newArgs === undefined ? {} : newJob.newArgs,
			workspace: $workspaceStore!
		}))
		console.log('payloads', payloads)

		const promises = payloads.map((payload) => JobService.runScriptByHash(payload))
		const results = await Promise.allSettled(promises)
		console.log('results', results)
		return results
	}
</script>

<div class="tabs-container overflow-x-auto">
	<Button
		on:click|once={() => {
			runImmediatelyPriorArgs()
		}}
		color="blue"
		size="sm"
		startIcon={{ icon: RefreshCw }}
		wrapperClasses="m-2"
	>
		Run immediately selected scripts with prior args
	</Button>
	<Button
		on:click={() => {
			runnableScripts = runnableScripts.map((runnable) => ({
				...runnable,
				newArgs: structuredClone(runnable.prior.args)
			}))
		}}
		color="blue"
		size="sm"
		wrapperClasses="m-2"
	>
		Reset to prior args
	</Button>

	{#if !isLoading}
		<Tabs bind:selected={tab}>
			{#each factorisedScriptArgs as factorised (`tab-${factorised.hash}`)}
				<Tab value={factorised.hash}>{factorised.hash}</Tab>
			{/each}

			<svelte:fragment slot="content">
				{#each factorisedScriptArgs as factorised (`tabcontent-${factorised.hash}`)}
					<TabContent value={factorised.hash}>
						<div class="p-8 w-full max-w-3xl mx-auto">
							<RunForm
								scheduledForStr={undefined}
								invisible_to_owner={undefined}
								overrideTag={undefined}
								autofocus={true}
								detailed={false}
								runnable={factorised}
								runAction={runScript}
								bind:args={factorised.args}
								schedulable={false}
								noUrlChange={true}
							/>
						</div>
					</TabContent>
				{/each}
			</svelte:fragment>
		</Tabs>
	{/if}
</div>
