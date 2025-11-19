<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import { isFlowTainted } from './utils'
	import Tutorial from './Tutorial.svelte'
	import type { DriveStep } from 'driver.js'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	let tutorial: Tutorial | undefined = undefined

	export function runTutorial() {
		tutorial?.runTutorial()
	}
</script>

<Tutorial
	bind:this={tutorial}
	index={8}
	name="workspace-onboarding-continue"
	tainted={isFlowTainted(flowStore.val)}
	on:error
	on:skipAll
	getSteps={(driver) => {
		const steps: DriveStep[] = [
			{
				popover: {
					title: 'Now let\'s create a flow together',
					description:
						'<img src="/languages.png" alt="Programming Languages" style="width: 100%; max-width: 400px; margin-bottom: 12px; border-radius: 8px;" /><p>Let\'s build your first flow step by step!</p>'
				}
			}
		]

		return steps
	}}
/>

