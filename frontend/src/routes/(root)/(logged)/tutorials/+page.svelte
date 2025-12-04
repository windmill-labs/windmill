<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Tab } from '$lib/components/common'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import WorkspaceTutorials from '$lib/components/WorkspaceTutorials.svelte'
	import TutorialButton from '$lib/components/home/TutorialButton.svelte'
	import TutorialProgressBar from '$lib/components/tutorials/TutorialProgressBar.svelte'
	import { tutorialsToDo } from '$lib/stores'
	import { onMount } from 'svelte'
	import { afterNavigate } from '$app/navigation'
	import {
		syncTutorialsTodos,
		resetAllTodos,
		getTutorialProgressTotal,
		getTutorialProgressCompleted
	} from '$lib/tutorialUtils'
	import { Button } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'
	import { TUTORIALS_CONFIG, type TabId } from '$lib/tutorials/config'

	let tab: TabId = $state('quickstart')

	let workspaceTutorials: WorkspaceTutorials | undefined = $state(undefined)

	// Tutorial index mapping
	const TUTORIAL_INDEXES = {
		'workspace-onboarding': 1,
		'flow-live-tutorial': 2,
		'troubleshoot-flow': 3
	} as const

	// Get current tab configuration
	const currentTabConfig = $derived(TUTORIALS_CONFIG[tab])

	// Create tutorial index mapping for current tab
	const currentTabTutorialIndexes = $derived(
		Object.fromEntries(
			currentTabConfig.tutorials
				.filter((tutorial) => tutorial.id in TUTORIAL_INDEXES)
				.map((tutorial) => [tutorial.id, TUTORIAL_INDEXES[tutorial.id as keyof typeof TUTORIAL_INDEXES]])
		)
	)

	// Calculate progress for current tab
	const totalTutorials = $derived(getTutorialProgressTotal(currentTabTutorialIndexes))
	const completedTutorials = $derived(
		getTutorialProgressCompleted(currentTabTutorialIndexes, $tutorialsToDo)
	)

	// Tutorials are ready to use directly from config
	const tutorials = $derived(currentTabConfig.tutorials)

	// Sync tutorial progress on mount and when navigating to this page
	onMount(() => {
		// Initial sync
		syncTutorialsTodos()

		// Sync when page becomes visible (user returns from completing a tutorial)
		const handleVisibilityChange = () => {
			if (!document.hidden) {
				syncTutorialsTodos()
			}
		}
		document.addEventListener('visibilitychange', handleVisibilityChange)

		// Also sync on window focus
		const handleFocus = () => {
			syncTutorialsTodos()
		}
		window.addEventListener('focus', handleFocus)

		return () => {
			document.removeEventListener('visibilitychange', handleVisibilityChange)
			window.removeEventListener('focus', handleFocus)
		}
	})

	// Sync when navigating to this page (e.g., after completing a tutorial)
	afterNavigate(() => {
		syncTutorialsTodos()
	})

	// Check if a tutorial is completed
	function isTutorialCompleted(tutorialId: string): boolean {
		const index = TUTORIAL_INDEXES[tutorialId as keyof typeof TUTORIAL_INDEXES]
		if (index === undefined) return false
		return !$tutorialsToDo.includes(index)
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
			{#each Object.entries(TUTORIALS_CONFIG) as [tabId, config]}
				<Tab value={tabId} label={config.label} icon={config.icon} />
			{/each}
		</Tabs>
	</div>

	{#if currentTabConfig && currentTabConfig.tutorials.length > 0}
		<div class="pt-8">
			<TutorialProgressBar
				completed={completedTutorials}
				total={totalTutorials}
				label="tutorials"
			/>

			<div class="border rounded-md bg-surface-tertiary">
				{#each tutorials as tutorial}
					<TutorialButton
						icon={tutorial.icon}
						title={tutorial.title}
						description={tutorial.description}
						onclick={tutorial.onClick}
						isCompleted={isTutorialCompleted(tutorial.id)}
					/>
				{/each}
			</div>
		</div>
	{:else if currentTabConfig}
		<div class="pt-8">
			<div class="text-center text-secondary text-sm py-8">
				No tutorials available for this section yet.
			</div>
		</div>
	{/if}
</CenteredPage>

<WorkspaceTutorials bind:this={workspaceTutorials} />
