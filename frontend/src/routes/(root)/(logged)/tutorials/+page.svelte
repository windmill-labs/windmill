<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Tab } from '$lib/components/common'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import { BookOpen, Users, Workflow, GraduationCap, Wrench } from 'lucide-svelte'
	import { base } from '$lib/base'
	import WorkspaceTutorials from '$lib/components/WorkspaceTutorials.svelte'
	import TutorialButton from '$lib/components/home/TutorialButton.svelte'

	let tab: 'quickstart' | 'team' = $state('quickstart')

	let workspaceTutorials: WorkspaceTutorials | undefined = $state(undefined)

	function startWorkspaceOnboarding() {
		workspaceTutorials?.runTutorialById('workspace-onboarding')
	}

	function startFlowTutorial() {
		window.location.href = `${base}/flows/add?tutorial=flow-live-tutorial&nodraft=true`
	}

	function startTroubleshootFlowTutorial() {
		window.location.href = `${base}/flows/add?tutorial=troubleshoot-flow&nodraft=true`
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
					icon={Wrench}
					title="Fix a broken flow"
					description="Learn how to monitor and debug your script and flow executions."
					onclick={startTroubleshootFlowTutorial}
				/>
			</div>
		</div>
	{:else if tab === 'team'}
		<!-- Team Collaboration content will go here -->
	{/if}
</CenteredPage>

<WorkspaceTutorials bind:this={workspaceTutorials} />
