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
	import { tutorialsToDo, userStore } from '$lib/stores'
	import { TUTORIALS_CONFIG } from '$lib/tutorials/config'
	import { hasRoleAccess } from '$lib/tutorials/roleUtils'
	import { onMount } from 'svelte'

	let isDismissed = $state(true)

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

			// Check if banner has been manually dismissed
			const manuallyDismissed = getLocalSetting(TUTORIAL_BANNER_DISMISSED_KEY) === 'true'

			// Safe to check tutorialsToDo here since we awaited syncTutorialsTodos() above
			// Filter tutorialsToDo to only include tutorials accessible to the user
			const remainingAccessibleTutorials = $tutorialsToDo.filter((index) =>
				accessibleTutorialIndexes.has(index)
			)

			// Check if all accessible tutorials are completed
			const allTutorialsCompleted = remainingAccessibleTutorials.length === 0

			// Dismiss banner if manually dismissed OR all accessible tutorials completed
			if (manuallyDismissed || allTutorialsCompleted) {
				isDismissed = true
				// Set localStorage when all tutorials are completed to persist dismissal
				// Note: This will re-set the key on every page load if localStorage is cleared
				// but tutorials remain completed in backend. This is intentional - the banner
				// should stay hidden if tutorials are completed, regardless of localStorage state.
				if (allTutorialsCompleted) {
					storeLocalSetting(TUTORIAL_BANNER_DISMISSED_KEY, 'true')
				}
			}
		} catch (error) {
			console.error('Failed to sync tutorial progress:', error)
			// Fallback to manual dismissal check only if API call fails
			isDismissed = getLocalSetting(TUTORIAL_BANNER_DISMISSED_KEY) === 'true'
		}
	})

	async function handleSkipAllTutorials() {
		await skipAllTodos()
		await syncTutorialsTodos()
		storeLocalSetting(TUTORIAL_BANNER_DISMISSED_KEY, 'true')
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
					Learn with interactive tutorials
				</div>
				<div class="text-hint text-3xs truncate text-left font-normal">
					Get started quickly with step-by-step guides on building flows, scripts, and more.
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
