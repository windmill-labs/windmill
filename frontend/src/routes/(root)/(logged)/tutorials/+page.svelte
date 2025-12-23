<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Tab } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
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
		resetTutorialByIndex,
		completeTutorialByIndex
	} from '$lib/tutorialUtils'
	import { Button } from '$lib/components/common'
	import { RefreshCw, CheckCheck, CheckCircle2, Circle, Shield, Code, UserCog } from 'lucide-svelte'
	import { TUTORIALS_CONFIG, type TabId, type TabConfig } from '$lib/tutorials/config'
	import { userStore } from '$lib/stores'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { hasRoleAccess, hasRoleAccessForPreview, getUserEffectiveRole, type Role } from '$lib/tutorials/roleUtils'

	// Get user's effective role (derived from userStore)
	const userEffectiveRole = $derived.by(() => {
		return getUserEffectiveRole($userStore) ?? 'admin'
	})
	
	// State for the role selector (only used when user is admin)
	// Defaults to user's actual role
	let selectedPreviewRole: Role = $state('admin')
	
	// Initialize selectedPreviewRole to user's role when admin, reset when not admin
	$effect(() => {
		const user = $userStore
		if (user?.is_admin) {
			// Initialize to user's actual role if not already set to a valid role
			// This ensures it's always set to the user's role when they're admin
			selectedPreviewRole = userEffectiveRole
		} else {
			// Reset to 'admin' as default (though this shouldn't matter for non-admins)
			selectedPreviewRole = 'admin'
		}
	})


	// Memoize access check dependencies to avoid unnecessary recalculations
	// This derived value only recalculates when userStore or selectedPreviewRole changes
	const accessCheckContext = $derived.by(() => {
		const user = $userStore
		// Always use preview mode for admins to show role-specific tutorials
		// This ensures admins only see tutorials for the selected role
		const usePreview = user?.is_admin
		return { user, usePreview, previewRole: selectedPreviewRole }
	})

	// Get active tabs only (filtered by active and roles)
	// Optimized: $derived.by() automatically memoizes - only recalculates when dependencies change
	const activeTabs = $derived.by(() => {
		// Access context to establish reactive dependency
		const context = accessCheckContext
		return (Object.entries(TUTORIALS_CONFIG) as [TabId, TabConfig][]).filter(([, config]) => {
			// Filter by active
			if (config.active === false) return false
			// Filter by roles (context is captured in closure)
			if (context.usePreview) {
				return hasRoleAccessForPreview(context.previewRole, config.roles)
			}
			return hasRoleAccess(context.user, config.roles)
		})
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
	// Optimized: $derived.by() automatically memoizes - only recalculates when tab or accessCheckContext changes
	const visibleTutorials = $derived.by(() => {
		// Access context to establish reactive dependency
		const context = accessCheckContext
		return currentTabConfig.tutorials.filter((tutorial) => {
			if (tutorial.active === false) return false
			// Use context directly to avoid function call overhead
			if (context.usePreview) {
				return hasRoleAccessForPreview(context.previewRole, tutorial.roles)
			}
			return hasRoleAccess(context.user, tutorial.roles)
		})
	})

	// Create tutorial index mapping for current tab (only visible tutorials with index defined)
	// Optimized: only recalculates when visibleTutorials changes
	const currentTabTutorialIndexes = $derived.by(() => {
		return Object.fromEntries(
			visibleTutorials
				.filter((tutorial) => tutorial.index !== undefined)
				.map((tutorial) => [tutorial.id, tutorial.index!])
		)
	})

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
		try {
			await skipTutorialsByIndexes(currentTabIndexes)
			await syncTutorialsTodos()
		} catch (error) {
			console.error('Error marking tutorials as completed:', error)
		}
	}

	// Reset all tutorials in current tab
	async function resetCurrentTabTutorials() {
		if (currentTabIndexes.length === 0) return
		try {
			await resetTutorialsByIndexes(currentTabIndexes)
			await syncTutorialsTodos()
		} catch (error) {
			console.error('Error resetting tutorials:', error)
		}
	}

	// Update a single tutorial's completion status
	async function updateSingleTutorial(tutorialId: string, completed: boolean) {
		const tutorial = currentTabConfig.tutorials.find((t) => t.id === tutorialId)
		if (!tutorial || tutorial.index === undefined) {
			console.warn(`Tutorial not found or has no index: ${tutorialId}`)
			return
		}

		try {
			if (completed) {
				await completeTutorialByIndex(tutorial.index)
			} else {
				await resetTutorialByIndex(tutorial.index)
			}
			await syncTutorialsTodos()
		} catch (error) {
			console.error(`Error ${completed ? 'completing' : 'resetting'} tutorial:`, error)
		}
	}

	// Calculate progress for each tab
	function getTabProgress(tabId: TabId) {
		const tabConfig = TUTORIALS_CONFIG[tabId]
		const context = accessCheckContext
		
		// Get all tutorial indexes for this tab (filtered by role)
		const indexes: number[] = []
		for (const tutorial of tabConfig.tutorials) {
			if (tutorial.active === false || tutorial.index === undefined) continue
			// Use context directly to check access
			if (context.usePreview) {
				if (!hasRoleAccessForPreview(context.previewRole, tutorial.roles)) continue
			} else {
				if (!hasRoleAccess(context.user, tutorial.roles)) continue
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
	<div class="flex flex-col gap-4 pb-2 my-4 mr-2">
		<div class="flex flex-row flex-wrap justify-between items-start">
			<span class="flex items-center gap-2">
				<h1 class="text-2xl font-semibold text-emphasis whitespace-nowrap leading-6 tracking-tight"
					>Tutorials</h1
				>
				<Tooltip documentationLink="https://www.windmill.dev/docs/intro">
					Learn how to use Windmill with our interactive tutorials
				</Tooltip>
			</span>
			{#if activeTabs.length > 0}
				<div class="flex items-start gap-2 pt-1">
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
		</div>
		{#if $userStore?.is_admin}
			<div class="flex flex-col gap-1">
				<div class="flex items-center gap-2">
					<span class="text-xs text-secondary">View as an</span>
					<ToggleButtonGroup
						bind:selected={selectedPreviewRole}
						onSelected={(v) => {
							selectedPreviewRole = (v || userEffectiveRole) as Role
						}}
						noWFull
					>
						{#snippet children({ item })}
							<ToggleButton
								value={userEffectiveRole}
								label="Admin (me)"
								icon={Shield}
								size="sm"
								{item}
								tooltip="View tutorials as yourself (admin)"
							/>
							<ToggleButton
								value="developer"
								label="Developer"
								icon={Code}
								size="sm"
								{item}
								tooltip="Preview tutorials visible to developers"
							/>
							<ToggleButton
								value="operator"
								label="Operator"
								icon={UserCog}
								size="sm"
								{item}
								tooltip="Preview tutorials visible to operators"
							/>
						{/snippet}
					</ToggleButtonGroup>
				</div>
				<span class="text-3xs text-secondary">
					This allows you to see which tutorials your team members can access
				</span>
			</div>
		{/if}
	</div>

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
							onReset={() => updateSingleTutorial(tutorial.id, false)}
							onComplete={() => updateSingleTutorial(tutorial.id, true)}
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
