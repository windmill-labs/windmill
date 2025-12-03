<script lang="ts">
	import WorkspaceOnboardingTutorial from './tutorials/workspace/WorkspaceOnboardingTutorial.svelte'
	import { skipAllTodos } from '$lib/tutorialUtils'

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
	on:skipAll={skipAll}
/>
