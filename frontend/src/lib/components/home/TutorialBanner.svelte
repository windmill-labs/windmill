<script lang="ts">
	import { Button } from '$lib/components/common'
	import { GraduationCap, X } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { goto } from '$app/navigation'
	import { sendUserToast, type ToastAction } from '$lib/toast'
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'
	import {
		skipAllTodos,
		syncTutorialsTodos,
		TUTORIAL_BANNER_DISMISSED_KEY
	} from '$lib/tutorialUtils'
	import { tutorialsToDo, userStore, skippedAll } from '$lib/stores'
	import { TUTORIALS_CONFIG } from '$lib/tutorials/config'
	import { hasRoleAccess } from '$lib/tutorials/roleUtils'
	import { onMount } from 'svelte'

	let isDismissed = $state(false)
	let hasCompletedAny = $state(false)

	/**
	 * Get all tutorial indexes that are accessible to the current user based on their role.
	 * Automatically recomputes when $userStore changes.
	 */
	const accessibleTutorialIndexes = $derived.by(() => {
		const indexes = new Set<number>()
		const user = $userStore

		for (const tab of Object.values(TUTORIALS_CONFIG)) {
			// Check if user has access to this tab category
			if (!hasRoleAccess(user, tab.roles)) {
				continue
			}

			for (const tutorial of tab.tutorials) {
				// Check if tutorial has an index and user has access to it
				if (tutorial.index !== undefined && hasRoleAccess(user, tutorial.roles)) {
					indexes.add(tutorial.index)
				}
			}
		}
		return indexes
	})

	onMount(async () => {
		try {
			// Sync tutorial progress from backend first
			await syncTutorialsTodos()

			// Check if banner has been manually dismissed via X button (soft dismiss, per-device)
			const manuallyDismissed = getLocalSetting(TUTORIAL_BANNER_DISMISSED_KEY) === 'true'
			if (manuallyDismissed) {
				isDismissed = true
				return
			}

			// Check if user deliberately skipped all tutorials (permanent dismiss, from backend)
			if ($skippedAll) {
				isDismissed = true
				return
			}

			// Safe to check tutorialsToDo here since we awaited syncTutorialsTodos() above
			// Filter tutorialsToDo to only include tutorials accessible to the user's role
			const remainingAccessibleTutorials = $tutorialsToDo.filter((index) =>
				accessibleTutorialIndexes.has(index)
			)

			// Calculate if user has completed at least one tutorial (for banner wording)
			// This determines whether to show "New tutorial available!" or "Learn with interactive tutorials"
			hasCompletedAny = remainingAccessibleTutorials.length < accessibleTutorialIndexes.size

			// Hide banner if all accessible tutorials are completed (but can reappear with new tutorials)
			if (remainingAccessibleTutorials.length === 0) {
				isDismissed = true
				return
			}

			// Show banner - user has accessible tutorials to complete
			isDismissed = false
		} catch (error) {
			console.error('Failed to sync tutorial progress:', error)
			// Fallback to manual dismissal check only if API call fails
			isDismissed = getLocalSetting(TUTORIAL_BANNER_DISMISSED_KEY) === 'true'
		}
	})

	async function handleSkipAllTutorials() {
		// Skip all tutorials and set skipped_all flag in backend (permanent)
		await skipAllTodos()
		await syncTutorialsTodos()
		// No need to set localStorage - backend skipped_all flag is the source of truth
		isDismissed = true
	}

	function dismissBanner() {
		storeLocalSetting(TUTORIAL_BANNER_DISMISSED_KEY, 'true')
		isDismissed = true

		const actions: ToastAction[] = [
			{
				label: 'Skip tutorials',
				callback: handleSkipAllTutorials,
				buttonType: 'default'
			}
		]

		sendUserToast(
			'You can still access tutorials from the Tutorials page in the main menu or in the Help submenu.',
			false,
			actions,
			undefined,
			8000
		)
	}

	function goToTutorials() {
		goto(`${base}/tutorials`)
	}
</script>

{#if !isDismissed}
	<div
		class="flex items-center justify-between gap-4 px-4 py-3 rounded-lg border border-light bg-surface-tertiary mb-4"
	>
		<div class="flex items-center gap-3 flex-1 min-w-0">
			<GraduationCap size={20} class="text-accent-primary flex-shrink-0" />
			<div class="flex-1 min-w-0">
				<div class="text-emphasis flex-wrap text-left text-xs font-semibold">
					{#if hasCompletedAny}
						New tutorial available!
					{:else}
						Learn with interactive tutorials
					{/if}
				</div>
				<div class="text-hint text-3xs truncate text-left font-normal">
					{#if hasCompletedAny}
						Continue your learning journey and master new Windmill skills.
					{:else}
						Get started quickly with step-by-step guides on building flows, scripts, and more.
					{/if}
				</div>
			</div>
		</div>
		<div class="flex items-center gap-2 flex-shrink-0">
			<Button
				size="xs"
				variant="accent"
				onclick={goToTutorials}
				startIcon={{ icon: GraduationCap }}
			>
				View tutorials
			</Button>
			<button
				onclick={dismissBanner}
				class="p-1.5 rounded hover:bg-surface-hover text-secondary hover:text-primary transition-colors"
				aria-label="Dismiss tutorial banner"
			>
				<X size={16} />
			</button>
		</div>
	</div>
{/if}
