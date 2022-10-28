<script lang="ts">
	import { FlowStatusModule, Job } from '$lib/gen'
	import { ProgressBar, type GeneralStep, type LoopStep, type ProgressStep } from '../progressBar'

	export let job: Job | undefined = undefined
	export let duration = 200
	let jobProgress: { steps: ProgressStep[]; error: boolean } = { steps: [], error: false }

	$: if (job) updateJobProgress(job)

	function updateJobProgress(job: Job) {
		const modules = job?.flow_status?.modules
		if (!modules?.length) {
			return
		}

		jobProgress.steps = modules.map((module) => {
			if (
				module.type === FlowStatusModule.type.FAILURE ||
				(module.type === FlowStatusModule.type.SUCCESS && job['success'] === false)
			) {
				jobProgress.error = true
			}

			// Loop is still iterating
			if (module.iterator) {
				return <LoopStep>{
					type: 'loop',
					isDone: isJobStepDone(module.type),
					index: module.iterator.index || 0,
					length: module.iterator.itered?.length || 0
				}
			}
			// Loop is finished
			else if (module['flow_jobs']) {
				return <LoopStep>{
					type: 'loop',
					isDone: isJobStepDone(module.type),
					index: module['flow_jobs'].length,
					length: module['flow_jobs'].length
				}
			}
			// Not a loop
			return <GeneralStep>{
				type: 'general',
				isDone: isJobStepDone(module.type)
			}
		})
	}

	function isJobStepDone(type: FlowStatusModule.type | undefined) {
		if (!type) {
			return false
		}
		return type === FlowStatusModule.type.SUCCESS || type === FlowStatusModule.type.FAILURE
	}

	export function reset() {
		jobProgress.error = false
		jobProgress.steps = []
	}
</script>

<ProgressBar {...jobProgress} {duration} class={$$props.class} />
