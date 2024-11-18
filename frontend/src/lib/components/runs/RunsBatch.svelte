<script lang="ts">
	import { JobService, type Job, type RunScriptByPathData, type ScriptArgs } from '$lib/gen'
	import { RefreshCw } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'

	// export let ids: string[]
	export let jobs: Job[] | undefined
	export let selectedIds: string[]

	type NewJob = {
		prior: Job & { args: ScriptArgs }
		newArgs: ScriptArgs
	}

	type RunnableNewJob = NewJob & {
		prior: {
			script_path: string
			workspace_id: string
		}
	}

	// local state.
	let newJobs: NewJob[] = []

	if (jobs) {
		const fewJobs = jobs
			.filter((job) => selectedIds.indexOf(job.id) > -1)
			.filter((job): job is Job & { workspace_id: string } => job.workspace_id !== undefined)

		const promises = fewJobs.map((job) =>
			JobService.getJobArgs({ id: job.id, workspace: job.workspace_id })
		)
		Promise.allSettled(promises).then((results) => {
			const priorArgs = results
				.filter((result) => result.status === 'fulfilled')
				.map((p) => p.value) as ScriptArgs[]

			newJobs = fewJobs.map((job, i) => ({
				prior: { ...job, args: priorArgs[i] },
				newArgs: structuredClone(priorArgs[i])
			}))

			console.log({ newJobs })
		})
	}

	function isRunnableNewJob(newJob: NewJob): newJob is RunnableNewJob {
		const { prior } = newJob

		return (
			prior !== undefined && prior.script_path !== undefined && prior.workspace_id !== undefined
		)
	}

	async function runBatchImmediately() {
		console.log('clicked on run', newJobs)

		const payloads: RunScriptByPathData[] = newJobs.reduce((payloadsArr, newJob) => {
			if (!isRunnableNewJob(newJob)) {
				console.error('this job is not runnable:', newJob) // TODO: rm line
				return payloadsArr
			}

			payloadsArr.push({
				path: newJob.prior.script_path,
				requestBody: newJob.newArgs,
				workspace: newJob.prior.workspace_id
			})
			return payloadsArr
		}, [] as RunScriptByPathData[])
		console.log('payloads', payloads)

		const promises = payloads.map((payload) => JobService.runScriptByPath(payload))

		const results = await Promise.allSettled(promises)
		console.log('results', results)
		return results
	}
</script>

<div class="p-4 flex flex-col gap-2 items-start h-full">
	<Button
		on:click|once={() => {
			runBatchImmediately()
		}}
		color="blue"
		size="sm"
		startIcon={{ icon: RefreshCw }}>Run immediately selected scripts with same args</Button
	>

	<!-- <div class="w-2/3 p-4">
		<h3 class="text-lg font-semibold mb-4">Job Configurations</h3>

		<Button
			on:click={() => {
				newJobs = newJobs.map((job) => ({
					...job,
					newArgs: structuredClone(job.prior.args)
				}))
			}}
			color="light"
			size="sm">Reset job args to prior execution</Button
		>
		<Button
			on:click={() => {
				newJobs = newJobs.sort((a, b) => {
					if (a.prior.script_hash === undefined) return 1
					if (b.prior.script_hash === undefined) return -1
					return a.prior.script_hash.localeCompare(b.prior.script_hash)
				})

				console.log(newJobs.map((job) => job.prior.script_hash))
			}}
			color="light"
			size="sm"
			>Group jobs by script
		</Button>

		{#each newJobs as job}
			<div class="mb-6">
				<h4 class="text-md font-semibold mb-2">{job.prior.script_path} Config</h4>
				<div class="space-y-2">
					{#if job.newArgs !== undefined}
						{#each Object.keys(job.newArgs) as key}
							<div class="flex items-center space-x-4">
								<label class="w-20" for={key}>{key}</label>
								<input type="text" class="p-2 border rounded" bind:value={job.newArgs[key]} />
							</div>
						{/each}
					{/if}
				</div>
			</div>
		{/each}
	</div> -->
</div>
