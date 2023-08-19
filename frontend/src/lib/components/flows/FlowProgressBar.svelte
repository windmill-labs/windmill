<script lang="ts">
	import { FlowStatusModule, Job } from '$lib/gen'
	import ProgressBar from '../progressBar/ProgressBar.svelte'

	export let job: Job | undefined = undefined

	let error: number | undefined = undefined
	let index = 0
	let subIndex: number | undefined = undefined
	let subLength: number | undefined = undefined
	let length = 1

	$: if (job) updateJobProgress(job)

	function updateJobProgress(job: Job) {
		const modules = job?.flow_status?.modules
		if (!modules?.length) {
			return
		}

		let maxDone = 0
		let subStepIndex: undefined | number = undefined
		let subStepLength: undefined | number = undefined
		let newError: number | undefined = undefined
		modules.forEach((module, i) => {
			if (
				module.type === FlowStatusModule.type.FAILURE ||
				(module.type === FlowStatusModule.type.SUCCESS && job['success'] === false)
			) {
				newError = i
			}

			let isDone = isJobStepDone(module.type)
			if (isDone) {
				maxDone = i + 1
				return
			}

			// Loop is still iterating
			if (module.iterator) {
				const stepIndex = module.iterator.index || 0
				const stepLength = module.iterator.itered?.length || 0
				if (module.iterator.index != undefined) {
					subStepIndex = stepIndex
					subStepLength = stepLength
				}
			} else if (module.branchall) {
				subStepIndex = module.branchall.branch
				subStepLength = module.branchall.len
			}
		})
		error = newError
		subLength = subStepLength ? Math.max(subStepLength, 1) : undefined
		subIndex = subStepIndex
		length = Math.max(modules.length, 1)
		index = maxDone
	}

	function isJobStepDone(type: FlowStatusModule.type | undefined) {
		return type === FlowStatusModule.type.SUCCESS || type === FlowStatusModule.type.FAILURE
	}

	let resetP: any

	export function reset() {
		resetP?.()
		error = undefined
		subIndex = undefined
		subLength = undefined
		length = 1
		index = 0
	}
</script>

<ProgressBar bind:resetP {length} {index} {subLength} {subIndex} {error} class={$$props.class} />
