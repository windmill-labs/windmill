<script lang="ts">
	import { GraduationCap, Workflow, Play } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { getContext } from 'svelte'
	import type WorkspaceTutorials from '../WorkspaceTutorials.svelte'

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
			<button
				onclick={startWorkspaceOnboarding}
				class="block border rounded-lg px-6 py-5 bg-surface-secondary hover:bg-surface-hover transition-colors text-left"
			>
				<div class="flex items-center gap-2 mb-2 text-primary">
					<GraduationCap size={24} />
					<h2 class="text-base font-semibold">Workspace onboarding</h2>
				</div>
				<p class="text-sm font-normal text-secondary">
					Discover the basics of Windmill with a quick tour of the workspace.
				</p>
			</button>
			<button
				onclick={startFlowTutorial}
				class="block border rounded-lg px-6 py-5 bg-surface-secondary hover:bg-surface-hover transition-colors text-left"
			>
				<div class="flex items-center gap-2 mb-2 text-primary">
					<Workflow size={24} />
					<h2 class="text-base font-semibold">Build a flow</h2>
				</div>
				<p class="text-sm font-normal text-secondary">
					Learn how to build workflows in Windmill with our interactive tutorial.
				</p>
			</button>
			<button
				onclick={startRunsTutorial}
				class="block border rounded-lg px-6 py-5 bg-surface-secondary hover:bg-surface-hover transition-colors text-left"
			>
				<div class="flex items-center gap-2 mb-2 text-primary">
					<Play size={24} />
					<h2 class="text-base font-semibold">Fix a broken flow</h2>
				</div>
				<p class="text-sm font-normal text-secondary">
					Learn how to monitor and debug your script and flow executions.
				</p>
			</button>
		</div>
	</div>
{/if}
