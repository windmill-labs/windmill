<script lang="ts">
	import WorkspaceOnboardingTutorial from './tutorials/workspace/WorkspaceOnboardingTutorial.svelte'
	import { skipAllTodos } from '$lib/tutorialUtils'
	import { TUTORIALS_CONFIG } from '$lib/tutorials/config'

	let workspaceOnboardingTutorial: WorkspaceOnboardingTutorial | undefined = $state(undefined)

	// Helper function to get tutorial index from config
	function getTutorialIndex(id: string): number {
		for (const tab of Object.values(TUTORIALS_CONFIG)) {
			const tutorial = tab.tutorials.find((t) => t.id === id)
			if (tutorial?.index !== undefined) return tutorial.index
		}
		throw new Error(`Tutorial index not found for id: ${id}. Make sure the tutorial has an index defined in config.`)
	}

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
