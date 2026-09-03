<script lang="ts">
	import { Button } from '$lib/components/common'
	import CloseButton from '$lib/components/common/CloseButton.svelte'
	import { GraduationCap } from 'lucide-svelte'
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

	type BannerState = 'hidden' | 'start' | 'new'

	// Deciding what to show needs an API round-trip, so the banner paints the state the last visit
	// resolved to and reconciles once the sync answers. Guessing wrong once in a while beats
	// reflowing the home page on every load; nothing cached means hidden, the direction that does
	// not push the page down.
	const TUTORIAL_BANNER_STATE_KEY = 'tutorial_banner_state'

	const cachedState =
		getLocalSetting(TUTORIAL_BANNER_DISMISSED_KEY) === 'true'
			? 'hidden'
			: getLocalSetting(TUTORIAL_BANNER_STATE_KEY)
	let isDismissed = $state(cachedState !== 'start' && cachedState !== 'new')
	let hasCompletedAny = $state(cachedState === 'new')

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

	function resolveState(state: BannerState) {
		isDismissed = state === 'hidden'
		hasCompletedAny = state === 'new'
		// Last: persisting is best-effort, and a storage failure must not leave the banner stuck on
		// whatever the cache said
		storeLocalSetting(TUTORIAL_BANNER_STATE_KEY, state)
	}

	// The banner is interactive while the initial sync is still in flight, so a dismiss or a skip
	// can land mid-await. Once that happens the user's choice wins and the sync must not resurrect
	// the banner.
	let userHidBanner = false

	function hideBannerForUser() {
		userHidBanner = true
		resolveState('hidden')
	}

	onMount(async () => {
		// Manually dismissed via the X button (soft dismiss, per-device). Checked before the network
		// call so a dismissed banner can never flash back in.
		if (getLocalSetting(TUTORIAL_BANNER_DISMISSED_KEY) === 'true') {
			resolveState('hidden')
			return
		}

		try {
			// Sync tutorial progress from backend first
			await syncTutorialsTodos()
		} catch (error) {
			console.error('Failed to sync tutorial progress:', error)
			// Keep whatever the last successful sync resolved to rather than guessing again
			return
		}

		if (userHidBanner) {
			return
		}

		// Check if user deliberately skipped all tutorials (permanent dismiss, from backend)
		if ($skippedAll) {
			resolveState('hidden')
			return
		}

		// Safe to check tutorialsToDo here since we awaited syncTutorialsTodos() above
		// Filter tutorialsToDo to only include tutorials accessible to the user's role
		const remainingAccessibleTutorials = $tutorialsToDo.filter((index) =>
			accessibleTutorialIndexes.has(index)
		)

		// Hide banner if all accessible tutorials are completed (but can reappear with new tutorials)
		if (remainingAccessibleTutorials.length === 0) {
			resolveState('hidden')
			return
		}

		// Having completed at least one accessible tutorial switches the wording to
		// "New tutorial available!" instead of "Learn with interactive tutorials"
		resolveState(
			remainingAccessibleTutorials.length < accessibleTutorialIndexes.size ? 'new' : 'start'
		)
	})

	async function handleSkipAllTutorials() {
		// Skip all tutorials and set skipped_all flag in backend (permanent)
		await skipAllTodos()
		await syncTutorialsTodos()
		// No need to set the dismissed flag - backend skipped_all flag is the source of truth
		hideBannerForUser()
	}

	function dismissBanner() {
		storeLocalSetting(TUTORIAL_BANNER_DISMISSED_KEY, 'true')
		hideBannerForUser()

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
	<!-- A standing invitation, not an announcement: it sits inline at the start of the row rather
	     than filling the page, so it reads as one more control and not as a card the user has to
	     dispatch before getting to their work. -->
	<div class="flex items-center gap-2 mt-4 mb-4">
		<span class="text-hint text-xs truncate min-w-0">
			{#if hasCompletedAny}
				New tutorial available!
			{:else}
				First time?
			{/if}
		</span>
		<Button
			unifiedSize="sm"
			variant="default"
			onclick={goToTutorials}
			startIcon={{ icon: GraduationCap }}
		>
			Tutorials
		</Button>
		<CloseButton small noBg title="Dismiss tutorial banner" onClick={dismissBanner} />
	</div>
{/if}
