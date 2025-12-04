<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Tab } from '$lib/components/common'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import TutorialButton from '$lib/components/home/TutorialButton.svelte'
	import TutorialProgressBar from '$lib/components/tutorials/TutorialProgressBar.svelte'
	import { tutorialsToDo } from '$lib/stores'
	import { onMount } from 'svelte'
	import { afterNavigate } from '$app/navigation'
	import {
		syncTutorialsTodos,
		resetAllTodos,
		getTutorialProgressTotal,
		getTutorialProgressCompleted,
		skipAllTodos,
		skipTutorialsByIndexes,
		resetTutorialsByIndexes
	} from '$lib/tutorialUtils'
	import { Button } from '$lib/components/common'
	import { RefreshCw, CheckCheck } from 'lucide-svelte'
	import { TUTORIALS_CONFIG, type TabId } from '$lib/tutorials/config'
	import { userStore } from '$lib/stores'

	let tab: TabId = $state('quickstart')

	// Get current tab configuration
	const currentTabConfig = $derived(TUTORIALS_CONFIG[tab])

	// Create tutorial index mapping for current tab (only tutorials with index defined)
	const currentTabTutorialIndexes = $derived(
		Object.fromEntries(
			currentTabConfig.tutorials
				.filter((tutorial) => tutorial.index !== undefined)
				.map((tutorial) => [tutorial.id, tutorial.index!])
		)
	)

	// Calculate progress for current tab
	const totalTutorials = $derived(getTutorialProgressTotal(currentTabTutorialIndexes))
	const completedTutorials = $derived(
		getTutorialProgressCompleted(currentTabTutorialIndexes, $tutorialsToDo)
	)

	// Filter and sort tutorials based on props
	const tutorials = $derived(
		currentTabConfig.tutorials
			.filter((tutorial) => {
				if (tutorial.disabled) return false
				if (!tutorial.requiredRole) return true
				const user = $userStore
				if (!user) return false

				// Check role flags directly
				const { requiredRole } = tutorial
				if (requiredRole === 'admin') return user.is_admin || user.is_super_admin
				if (requiredRole === 'operator') return user.operator || user.is_admin || user.is_super_admin
				if (requiredRole === 'developer') return !user.operator || user.is_admin || user.is_super_admin
				return true
			})
			.sort((a, b) => (a.order ?? 999) - (b.order ?? 999))
	)

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
		const tutorial = currentTabConfig.tutorials.find((t) => t.id === tutorialId)
		if (!tutorial || tutorial.index === undefined) return false
		return !$tutorialsToDo.includes(tutorial.index)
	}

	// Get list of tutorial indexes for current tab
	const currentTabIndexes = $derived(
		Object.values(currentTabTutorialIndexes)
	)

	// Skip all tutorials in current tab
	async function skipCurrentTabTutorials() {
		if (currentTabIndexes.length === 0) return
		await skipTutorialsByIndexes(currentTabIndexes)
		await syncTutorialsTodos()
	}

	// Reset all tutorials in current tab
	async function resetCurrentTabTutorials() {
		if (currentTabIndexes.length === 0) return
		await resetTutorialsByIndexes(currentTabIndexes)
		await syncTutorialsTodos()
	}
</script>

<CenteredPage>
	<PageHeader
		title="Tutorials"
		tooltip="Learn how to use Windmill with our interactive tutorials"
		documentationLink="https://www.windmill.dev/docs/intro"
	>
		<div class="flex gap-2">
			<Button
				size="xs"
				variant="default"
				startIcon={{ icon: CheckCheck }}
				onclick={async () => {
					await skipAllTodos()
					await syncTutorialsTodos()
				}}
			>
				Mark all as complete
			</Button>
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
		</div>
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
			<div class="flex items-start gap-4 mb-6">
				<TutorialProgressBar
					completed={completedTutorials}
					total={totalTutorials}
					label="tutorials"
				/>
				<div class="flex gap-2 flex-shrink-0 pt-1">
					<Button
						size="xs"
						variant="default"
						startIcon={{ icon: CheckCheck }}
						onclick={skipCurrentTabTutorials}
					>
						Skip all
					</Button>
					<Button
						size="xs"
						variant="default"
						startIcon={{ icon: RefreshCw }}
						onclick={resetCurrentTabTutorials}
					>
						Reset
					</Button>
				</div>
			</div>

			<div class="border rounded-md bg-surface-tertiary">
				{#each tutorials as tutorial}
					<TutorialButton
						icon={tutorial.icon}
						title={tutorial.title}
						description={tutorial.description}
						onclick={tutorial.onClick}
						isCompleted={isTutorialCompleted(tutorial.id)}
						disabled={tutorial.disabled}
						comingSoon={tutorial.comingSoon}
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
