<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Tab } from '$lib/components/common'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import { BookOpen, Users, Workflow, GraduationCap, Wrench } from 'lucide-svelte'
	import { base } from '$lib/base'
	import WorkspaceTutorials from '$lib/components/WorkspaceTutorials.svelte'
	import TutorialButton from '$lib/components/home/TutorialButton.svelte'
	import { tutorialsToDo } from '$lib/stores'
	import { onMount } from 'svelte'
	import { syncTutorialsTodos, resetAllTodos } from '$lib/tutorialUtils'
	import { Button } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'

	let tab: 'quickstart' | 'team' = $state('quickstart')

	let workspaceTutorials: WorkspaceTutorials | undefined = $state(undefined)

	// Tutorial index mapping
	const TUTORIAL_INDEXES = {
		'workspace-onboarding': 1,
		'flow-live-tutorial': 2,
		'troubleshoot-flow': 3
	} as const

	// Sync tutorial progress on mount
	onMount(async () => {
		await syncTutorialsTodos()
	})

	// Check if a tutorial is completed
	function isTutorialCompleted(tutorialId: keyof typeof TUTORIAL_INDEXES): boolean {
		const index = TUTORIAL_INDEXES[tutorialId]
		return !$tutorialsToDo.includes(index)
	}

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
	>
		<Button
			size="xs"
			variant="default"
			startIcon={{ icon: RefreshCw }}
			onclick={async () => {
				await resetAllTodos()
				await syncTutorialsTodos()
			}}
		>
			Reset tutorials
		</Button>
	</PageHeader>
	<div class="flex justify-between pt-4">
		<Tabs class="w-full" bind:selected={tab}>
			<Tab value="quickstart" label="Quickstart" icon={BookOpen} />
			<Tab value="team" label="Team Collaboration" icon={Users} />
		</Tabs>
	</div>

	{#if tab === 'quickstart'}
		<div class="pt-8">
			<div class="border rounded-md bg-surface-tertiary">
				<TutorialButton
					icon={GraduationCap}
					title="Workspace onboarding"
					description="Discover the basics of Windmill with a quick tour of the workspace."
					onclick={startWorkspaceOnboarding}
					isCompleted={isTutorialCompleted('workspace-onboarding')}
				/>
				<TutorialButton
					icon={Workflow}
					title="Build a flow"
					description="Learn how to build workflows in Windmill with our interactive tutorial."
					onclick={startFlowTutorial}
					isCompleted={isTutorialCompleted('flow-live-tutorial')}
				/>
				<TutorialButton
					icon={Wrench}
					title="Fix a broken flow"
					description="Learn how to monitor and debug your script and flow executions."
					onclick={startTroubleshootFlowTutorial}
					isCompleted={isTutorialCompleted('troubleshoot-flow')}
				/>
			</div>
		</div>
	{:else if tab === 'team'}
		<!-- Team Collaboration content will go here -->
	{/if}
</CenteredPage>

<WorkspaceTutorials bind:this={workspaceTutorials} />
