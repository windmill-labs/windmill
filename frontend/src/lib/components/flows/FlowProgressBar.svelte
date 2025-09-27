<script lang="ts">
	import { type Job } from '$lib/gen'
	import ProgressBar from '../progressBar/ProgressBar.svelte'

	interface Props {
		job?: Job | undefined
		currentSubJobProgress?: number | undefined
		class?: string
	}

	let {
		job = undefined,
		currentSubJobProgress = $bindable(undefined),
		class: className
	}: Props = $props()

	let error: number | undefined = $state(undefined)
	let index = $state(0)
	let subIndex: number | undefined = $state(undefined)
	let subLength: number | undefined = $state(undefined)
	let length = $state(1)
	let nextInProgress = $state(false)
	let subIndexIsPercent: boolean = $state(false)

	let progressBar = $state<ProgressBar | undefined>(undefined)

	function updateJobProgress(job: Job) {
		const modules = job?.flow_status?.modules
		if (!modules?.length) {
			return
		}

		let subStepIndex: undefined | number = undefined
		let subStepLength: undefined | number = undefined
		let newError: number | undefined = undefined
		let newNextInProgress = false

		let maxDone = Math.max(job?.flow_status?.step ?? 0, 0)
		if (modules.length > maxDone) {
			const nextModule = modules[maxDone]
			if (nextModule.type === 'InProgress') {
				newNextInProgress = true
			}
		}

		let module = modules[maxDone]
		if (module) {
			if (module.type === 'Failure' || (module.type === 'Success' && job['success'] === false)) {
				newError = maxDone
				maxDone = maxDone + 1
			}
		}
		subIndexIsPercent = false

		// Loop is still iterating
		if (module?.iterator) {
			const stepIndex = module.iterator.index || 0
			const stepLength = module.iterator.itered?.length || 0
			if (module.iterator.index != undefined) {
				subStepIndex = stepIndex
				subStepLength = stepLength
			}
		} else if (module?.branchall) {
			subStepIndex = module.branchall.branch
			subStepLength = module.branchall.len
		} else if (module?.progress) {
			const clamp = (num, min, max) => Math.min(Math.max(num, min), max)
			subStepIndex = clamp(module?.progress, subIndex ?? 0, 99)
			//                  Jitter protection >^^^^^^^^
			subStepLength = 100
			subIndexIsPercent = true
			currentSubJobProgress = subStepIndex
		} else {
			currentSubJobProgress = undefined
		}

		error = newError
		subLength = subStepLength ? Math.max(subStepLength, 1) : undefined
		subIndex = subStepIndex
		length = Math.max(modules.length, 1)
		index = maxDone
		nextInProgress = newNextInProgress
	}

	export function reset() {
		progressBar?.resetP()
		error = undefined
		subIndex = undefined
		subLength = undefined
		length = 1
		index = 0
	}
	$effect(() => {
		job && updateJobProgress(job)
	})
</script>

<ProgressBar
	bind:this={progressBar}
	{length}
	{index}
	{nextInProgress}
	{subLength}
	{subIndex}
	{error}
	{subIndexIsPercent}
	class={className}
/>
