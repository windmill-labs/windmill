<script lang="ts">
	import WorkspaceOnboardingTutorial from './tutorials/workspace/WorkspaceOnboardingTutorial.svelte'
	import { skipAllTodos } from '$lib/tutorialUtils'
	import { getTutorialIndex } from '$lib/tutorials/config'

	let workspaceOnboardingTutorial: WorkspaceOnboardingTutorial | undefined = $state(undefined)

	export function runTutorialById(id: string) {
		if (id === 'workspace-onboarding') {
			workspaceOnboardingTutorial?.runTutorial()
		}
	}

	function skipAll() {
		skipAllTodos()
	}

	// Get index from config
	const workspaceOnboardingIndex = getTutorialIndex('workspace-onboarding')
</script>

<WorkspaceOnboardingTutorial
	bind:this={workspaceOnboardingTutorial}
	index={workspaceOnboardingIndex}
	on:skipAll={skipAll}
/>
