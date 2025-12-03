<script lang="ts">
	import { GraduationCap, Workflow, Play } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { getContext } from 'svelte'
	import type WorkspaceTutorials from '../WorkspaceTutorials.svelte'
	import TutorialButton from './TutorialButton.svelte'

	interface Props {
		hasFilters?: boolean
	}

	let { hasFilters = false }: Props = $props()

	const workspaceTutorialsContext: { value: WorkspaceTutorials | undefined } | undefined = getContext('workspaceTutorials')

	function startWorkspaceOnboarding() {
		workspaceTutorialsContext?.value?.runTutorialById('workspace-onboarding')
	}

	function startFlowTutorial() {
		window.location.href = `${base}/flows/add?tutorial=flow-live-tutorial&nodraft=true`
	}

	function startRunsTutorial() {
		// Navigate to flow editor with pre-built flow for the runs tutorial
		window.location.href = `${base}/flows/add?tutorial=troubleshoot-flow&nodraft=true`
	}
</script>

{#if hasFilters}
	<div class="flex justify-center items-center h-48">
		<div class="text-primary text-center">
			<div class="text-lg font-semibold text-emphasis">No items found</div>
			<div class="text-xs font-normal text-hint">Try changing your search or filters</div>
		</div>
	</div>
{:else}
	<div class="flex flex-col items-center justify-center py-12">
		<div class="text-center mb-8">
			<div class="text-lg font-semibold text-emphasis mb-2">Welcome to Windmill!</div>
			<div class="text-sm font-normal text-secondary">Get started with these tutorials</div>
		</div>
		<div class="grid grid-cols-1 lg:grid-cols-3 gap-6 max-w-5xl w-full px-4">
			<TutorialButton
				icon={GraduationCap}
				title="Workspace onboarding"
				description="Discover the basics of Windmill with a quick tour of the workspace."
				onclick={startWorkspaceOnboarding}
			/>
			<TutorialButton
				icon={Workflow}
				title="Build a flow"
				description="Learn how to build workflows in Windmill with our interactive tutorial."
				onclick={startFlowTutorial}
			/>
			<TutorialButton
				icon={Play}
				title="Fix a broken flow"
				description="Learn how to monitor and debug your script and flow executions."
				onclick={startRunsTutorial}
			/>
		</div>
	</div>
{/if}
