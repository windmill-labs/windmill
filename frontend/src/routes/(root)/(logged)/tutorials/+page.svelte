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
		resetTutorialsByIndexes,
		resetTutorialByIndex
	} from '$lib/tutorialUtils'
	import { Button } from '$lib/components/common'
	import { RefreshCw, CheckCheck, CheckCircle2, Circle } from 'lucide-svelte'
	import { TUTORIALS_CONFIG, type TabId } from '$lib/tutorials/config'
	import { userStore } from '$lib/stores'
	import type { UserExt } from '$lib/stores'

	/**
	 * Check if a user has access based on a roles array.
	 */
	function hasRoleAccess(
		user: UserExt | null | undefined,
		roles?: ('admin' | 'developer' | 'operator')[]
	): boolean {
		// No roles specified = available to everyone
		if (!roles || roles.length === 0) return true
		if (!user) return false

		// Check if user has any of the required roles
		return roles.some((role) => {
			if (role === 'admin') return user.is_admin
			if (role === 'operator') return user.operator || user.is_admin
			if (role === 'developer') return !user.operator || user.is_admin
			return false
		})
	}

	// Get active tabs only (filtered by active and roles)
	const activeTabs = $derived.by(() => {
		const user = $userStore
		return Object.entries(TUTORIALS_CONFIG).filter(([, config]) => {
			// Filter by active
			if (config.active === false) return false
			// Filter by roles
			return hasRoleAccess(user, config.roles)
		}) as [TabId, typeof TUTORIALS_CONFIG[TabId]][]
	})

	// Initialize tab to first active tab (already filtered by role and active status)
	let tab: TabId = $state('quickstart')

	// Set initial tab and ensure current tab is active and accessible
	$effect(() => {
		const firstActiveTab = activeTabs[0]?.[0]
		if (firstActiveTab) {
			// If current tab is not in active tabs, switch to first active tab
			if (!activeTabs.some(([tabId]) => tabId === tab)) {
				tab = firstActiveTab
			}
		}
	})

	// Get current tab configuration
	const currentTabConfig = $derived(TUTORIALS_CONFIG[tab])

	// Filter tutorials by role and active status (same logic as displayed tutorials)
	const visibleTutorials = $derived(
		currentTabConfig.tutorials.filter((tutorial) => {
			if (tutorial.active === false) return false
			return hasRoleAccess($userStore, tutorial.roles)
		})
	)

	// Create tutorial index mapping for current tab (only visible tutorials with index defined)
	const currentTabTutorialIndexes = $derived(
		Object.fromEntries(
			visibleTutorials
				.filter((tutorial) => tutorial.index !== undefined)
				.map((tutorial) => [tutorial.id, tutorial.index!])
		)
	)

	// Calculate progress for current tab (only counting visible tutorials)
	const totalTutorials = $derived(getTutorialProgressTotal(currentTabTutorialIndexes))
	const completedTutorials = $derived(
		getTutorialProgressCompleted(currentTabTutorialIndexes, $tutorialsToDo)
	)

	// Sort visible tutorials by order
	const tutorials = $derived(
		visibleTutorials.sort((a, b) => (a.order ?? 999) - (b.order ?? 999))
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

	// Reset a single tutorial
	async function resetSingleTutorial(tutorialId: string) {
		const tutorial = currentTabConfig.tutorials.find((t) => t.id === tutorialId)
		if (!tutorial || tutorial.index === undefined) return

		await resetTutorialByIndex(tutorial.index)
		await syncTutorialsTodos()
	}

	// Calculate progress for each tab
	function getTabProgress(tabId: TabId) {
		const tabConfig = TUTORIALS_CONFIG[tabId]
		const user = $userStore
		
		// Get all tutorial indexes for this tab (filtered by role)
		const indexes: number[] = []
		for (const tutorial of tabConfig.tutorials) {
			if (tutorial.active === false || tutorial.index === undefined) continue
			if (!hasRoleAccess(user, tutorial.roles)) continue
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
		{#if activeTabs.length > 0}
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
		{/if}
	</PageHeader>

	{#if activeTabs.length > 0}
		<div class="flex justify-between pt-4">
			<Tabs class="w-full" bind:selected={tab}>
				{#each activeTabs as [tabId, config]}
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

		{#if tutorials.length > 0}
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
							disabled={tutorial.active === false}
							comingSoon={tutorial.comingSoon}
							onReset={() => resetSingleTutorial(tutorial.id)}
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
	{:else}
		<div class="pt-8">
			<div class="text-center text-secondary text-sm py-8">
				No tutorials available for now. Coming soon.
			</div>
		</div>
	{/if}
</CenteredPage>
