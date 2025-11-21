<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Tab } from '$lib/components/common'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import { BookOpen, Users, Workflow, GraduationCap } from 'lucide-svelte'
	import { base } from '$lib/base'
	import WorkspaceTutorials from '$lib/components/WorkspaceTutorials.svelte'

	let tab: 'quickstart' | 'team' = $state('quickstart')

	let workspaceTutorials: WorkspaceTutorials | undefined = $state(undefined)

	function startWorkspaceOnboarding() {
		workspaceTutorials?.runTutorialById('workspace-onboarding')
	}

	function startFlowTutorial() {
		window.location.href = `${base}/flows/add?tutorial=flow-live-tutorial&nodraft=true`
	}
</script>

<CenteredPage>
	<PageHeader
		title="Tutorials"
		tooltip="Learn how to use Windmill with our interactive tutorials"
		documentationLink="https://www.windmill.dev/docs/intro"
	/>
	<div class="flex justify-between pt-4">
		<Tabs class="w-full" bind:selected={tab}>
			<Tab value="quickstart" label="Quickstart" icon={BookOpen} />
			<Tab value="team" label="Team Collaboration" icon={Users} />
		</Tabs>
	</div>

	{#if tab === 'quickstart'}
		<div class="pt-8">
			<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
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
						<h2 class="text-base font-semibold">Create a simple flow</h2>
					</div>
					<p class="text-sm font-normal text-secondary">
						Learn how to build workflows in Windmill with our interactive step-by-step tutorial.
					</p>
				</button>
			</div>
		</div>
	{:else if tab === 'team'}
		<!-- Team Collaboration content will go here -->
	{/if}
</CenteredPage>

<WorkspaceTutorials bind:this={workspaceTutorials} />
