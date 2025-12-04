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
	import { RefreshCw, CheckCheck, CheckCircle2, Circle } from 'lucide-svelte'
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

	// Calculate progress for each tab
	function getTabProgress(tabId: TabId) {
		const tabConfig = TUTORIALS_CONFIG[tabId]
		const user = $userStore
		
		// Get all tutorial indexes for this tab (filtered by role)
		const indexes: number[] = []
		for (const tutorial of tabConfig.tutorials) {
			if (tutorial.disabled || tutorial.index === undefined) continue
			if (tutorial.requiredRole && user) {
				const { requiredRole } = tutorial
				if (requiredRole === 'admin' && !user.is_admin && !user.is_super_admin) continue
				if (requiredRole === 'operator' && !user.operator && !user.is_admin && !user.is_super_admin) continue
				if (requiredRole === 'developer' && user.operator && !user.is_admin && !user.is_super_admin) continue
			}
			indexes.push(tutorial.index)
		}
		
		const total = indexes.length
		const completed = indexes.filter((index) => !$tutorialsToDo.includes(index)).length
		
		return { total, completed }
	}

	// Get badge info for a tab
	function getTabBadge(tabId: TabId) {
		const { total, completed } = getTabProgress(tabId)
		
		if (total === 0) return { type: 'none' as const }
		if (completed === 0) {
			// Circle icon if not started
			return { type: 'dot' as const }
		}
		if (completed === total) {
			// CheckCircle2 icon if completed
			return { type: 'check' as const }
		}
		// (1/3) format if started
		return { type: 'progress' as const, text: `(${completed}/${total})` }
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
				Mark all as completed
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
				Reset all
			</Button>
		</div>
	</PageHeader>

	<div class="flex justify-between pt-4">
		<Tabs class="w-full" bind:selected={tab}>
			{#each Object.entries(TUTORIALS_CONFIG) as [tabId, config]}
				{@const badge = getTabBadge(tabId as TabId)}
				{#if badge.type === 'progress'}
					<Tab value={tabId} label={config.label}>
						{#snippet extra()}
							<span class="text-xs text-secondary ml-1.5 flex-shrink-0">{badge.text}</span>
						{/snippet}
					</Tab>
				{:else if badge.type === 'check'}
					<Tab value={tabId} label={config.label}>
						{#snippet extra()}
							<CheckCircle2 size={14} class="ml-1.5 flex-shrink-0" />
						{/snippet}
					</Tab>
				{:else if badge.type === 'dot'}
					<Tab value={tabId} label={config.label}>
						{#snippet extra()}
							<Circle size={14} class="ml-1.5 flex-shrink-0" />
						{/snippet}
					</Tab>
				{:else}
					<Tab value={tabId} label={config.label} />
				{/if}
			{/each}
		</Tabs>
	</div>

	{#if currentTabConfig && currentTabConfig.tutorials.length > 0}
		<div class="pt-8">
			<div class="flex items-start gap-4 mb-6">
				{#if currentTabConfig.progressBar !== false}
					<TutorialProgressBar
						completed={completedTutorials}
						total={totalTutorials}
						label="tutorials"
					/>
				{/if}
				<div class="flex gap-2 flex-shrink-0 pt-1">
					<Button
						size="xs"
						variant="default"
						startIcon={{ icon: CheckCheck }}
						onclick={skipCurrentTabTutorials}
					>
						Mark as completed
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
