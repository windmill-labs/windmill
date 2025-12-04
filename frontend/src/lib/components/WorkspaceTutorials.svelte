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

</script>

<WorkspaceOnboardingTutorial
	bind:this={workspaceOnboardingTutorial}
	index={getTutorialIndex('workspace-onboarding')}
	on:skipAll={skipAll}
	on:error
	on:reload
/>
