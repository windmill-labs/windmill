<script lang="ts">
	import WorkspaceOnboardingTutorial from './tutorials/workspace/WorkspaceOnboardingTutorial.svelte'
	import { skipAllTodos } from '$lib/tutorialUtils'
	import { getTutorialIndex } from '$lib/tutorials/config'

	let workspaceOnboardingTutorial: WorkspaceOnboardingTutorial | undefined = $state(undefined)

	// Map tutorial IDs to their component instances
	const tutorialInstances = new Map<string, { runTutorial: () => void } | undefined>()

	// Update map when instance changes
	$effect(() => {
		tutorialInstances.set('workspace-onboarding', workspaceOnboardingTutorial)
	})

	export function runTutorialById(id: string) {
		const instance = tutorialInstances.get(id)
		if (instance) {
			instance.runTutorial()
		} else {
			console.warn(`Tutorial instance not found for id: ${id}`)
		}
	}

	function skipAll() {
		skipAllTodos()
	}
</script>

<WorkspaceOnboardingTutorial
	bind:this={workspaceOnboardingTutorial}
	index={getTutorialIndex('workspace-onboarding')}
	on:skipAll={skipAll}
	on:error
	on:reload
/>
