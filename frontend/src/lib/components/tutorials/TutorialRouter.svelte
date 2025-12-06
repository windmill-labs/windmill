<script lang="ts">
	import { skipAllTodos } from '$lib/tutorialUtils'
	import TutorialWrapper from './TutorialWrapper.svelte'

	interface TutorialDefinition {
		id: string
		component: any // Svelte component type - using any to avoid complex type issues
		name?: string // Optional name prop (used by some tutorials like AppTutorials)
		supportsSkipSteps?: boolean // Whether runTutorial accepts skipStepsCount parameter
	}

	interface Props {
		tutorials: TutorialDefinition[]
	}

	let { tutorials }: Props = $props()

	// Map tutorial IDs to their component instances
	const tutorialInstances = new Map<
		string,
		{ runTutorial: (options?: number) => void } | { runTutorial: () => void } | undefined
	>()

	function skipAll() {
		skipAllTodos()
	}

	// Helper function to register a tutorial instance
	function registerInstance(id: string, instance: any) {
		tutorialInstances.set(id, instance)
	}

	// Export function to run tutorial by ID
	export function runTutorialById(id: string, options?: { skipStepsCount?: number }) {
		const instance = tutorialInstances.get(id)
		if (!instance) {
			console.warn(`Tutorial instance not found for id: ${id}`)
			return
		}

		// Check if this tutorial supports skipStepsCount
		const tutorial = tutorials.find((t) => t.id === id)
		if (tutorial?.supportsSkipSteps && options?.skipStepsCount !== undefined) {
			// Type assertion needed because TypeScript can't narrow the union type
			;(instance as { runTutorial: (options?: number) => void }).runTutorial(options.skipStepsCount)
		} else {
			// Call runTutorial without parameters
			if ('runTutorial' in instance && typeof instance.runTutorial === 'function') {
				instance.runTutorial()
			}
		}
	}
</script>

{#each tutorials as tutorial}
	<TutorialWrapper
		id={tutorial.id}
		component={tutorial.component}
		name={tutorial.name}
		onInstanceReady={registerInstance}
		onSkipAll={skipAll}
	/>
{/each}

