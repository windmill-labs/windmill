<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import WorkspaceOnboardingTutorial from './tutorials/workspace/WorkspaceOnboardingTutorial.svelte'

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
/>
