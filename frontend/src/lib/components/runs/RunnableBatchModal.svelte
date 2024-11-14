<script lang="ts">
	import { JobService, type Job, type RunScriptByPathData, type ScriptArgs } from '$lib/gen'
	import Modal from '../common/modal/Modal.svelte'
	import Button from '../common/button/Button.svelte'
	import Toggle from '../Toggle.svelte'

	export let jobs: Job[] | undefined
	export let modalIsOpen = false

	type NewJob = {
		prior: Job & { args: ScriptArgs }
		newArgs: ScriptArgs
		isSelected: boolean
	}

	// local state
	let newJobs: NewJob[] = []

	if (jobs) {
		const fewJobs = jobs.slice(0, 10)
		const promises = fewJobs.map((job) => JobService.getJobArgs({ id: job.id, workspace: 'demo' }))
		Promise.allSettled(promises).then((results) => {
			const priorArgs = results
				.filter((result) => result.status === 'fulfilled')
				.map((p) => p.value) as ScriptArgs[]

			newJobs = fewJobs.map((job, i) => ({
				prior: { ...job, args: priorArgs[i] },
				newArgs: structuredClone(priorArgs[i]),
				isSelected: false
			}))
		})
	}

	type RunnableNewJob = NewJob & {
		prior: {
			script_path: string
			workspace_id: string
		}
	}

	function isRunnableNewJob(newJob: NewJob): newJob is RunnableNewJob {
		const { prior } = newJob

		return prior.script_path !== undefined && prior.workspace_id !== undefined
	}

	async function executeBatch() {
		console.log(newJobs)

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

		console.log(payloads)

		const promises = payloads.map((payload) => JobService.runScriptByPath(payload))

		Promise.allSettled(promises).then((results) => {
			console.log(results)
		})
	}
</script>

<Modal
	bind:open={modalIsOpen}
	on:confirmed={() => {
		modalIsOpen = false
	}}
	on:canceled
	title="Runnable batch"
	class="w-full max-w-full xl:max-w-7xl"
>
	<div class="flex space-x-6">
		<!-- Left Side: Job List -->
		<div class="w-1/3 p-4 border-r">
			<h3 class="text-lg font-semibold mb-4">Runnable Jobs</h3>
			<ul class="space-y-2">
				{#each newJobs as job}
					<li>
						<Toggle bind:checked={job.isSelected} />
						<span>{job.prior.id}</span>
						<span>{job.prior.script_path}</span>
						<span>{job.prior.tag}</span>
					</li>
				{/each}
			</ul>
		</div>

		<!-- Right Side: Job Configurations -->
		<div class="w-2/3 p-4">
			<h3 class="text-lg font-semibold mb-4">Job Configurations</h3>
			<Button
				on:click={() => {
					newJobs = newJobs.map((job) => ({
						...job,
						isSelected: true
					}))
				}}
				color="light"
				size="sm">Select all</Button
			>
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

			{#each newJobs as job}
				{#if job.isSelected}
					<div class="mb-6">
						<h4 class="text-md font-semibold mb-2">{job.prior.id} Config</h4>
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
				{/if}
			{/each}
		</div>
	</div>

	<Button
		slot="actions"
		on:click={() => {
			executeBatch()
			// modalIsOpen = false
		}}
		color="light"
		size="sm"
	>
		<span class="inline-flex gap-2">Execute</span>
	</Button>
</Modal>
