<script lang="ts">
	import { Button } from '$lib/components/common'
	import { GraduationCap, X } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { goto } from '$app/navigation'
	import { sendUserToast, type ToastAction } from '$lib/toast'
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'
	import { skipAllTodos, syncTutorialsTodos } from '$lib/tutorialUtils'
	import { tutorialsToDo } from '$lib/stores'
	import { TUTORIALS_CONFIG } from '$lib/tutorials/config'
	import { onMount } from 'svelte'

	const DISMISSED_KEY = 'tutorial_banner_dismissed'
	let isDismissed = $state(false)

	/**
	 * Get all tutorial indexes that are actually defined in the config
	 */
	function getDefinedTutorialIndexes(): Set<number> {
		const indexes = new Set<number>()
		for (const tab of Object.values(TUTORIALS_CONFIG)) {
			for (const tutorial of tab.tutorials) {
				if (tutorial.index !== undefined) {
					indexes.add(tutorial.index)
				}
			}
		}
		return indexes
	}

	onMount(async () => {
		// Sync tutorial progress from backend first
		await syncTutorialsTodos()

		// Check if banner has been manually dismissed
		const manuallyDismissed = getLocalSetting(DISMISSED_KEY) === 'true'

		// Get all defined tutorial indexes
		const definedIndexes = getDefinedTutorialIndexes()

		// Filter tutorialsToDo to only include tutorials that actually exist
		const remainingRealTutorials = $tutorialsToDo.filter((index) => definedIndexes.has(index))

		// Check if all real tutorials are completed
		const allTutorialsCompleted = remainingRealTutorials.length === 0

		// Dismiss banner if manually dismissed OR all tutorials completed
		if (manuallyDismissed || allTutorialsCompleted) {
			isDismissed = true
			// Set localStorage to ensure banner stays dismissed
			if (allTutorialsCompleted) {
				storeLocalSetting(DISMISSED_KEY, 'true')
			}
		}
	})

	async function handleSkipAllTutorials() {
		await skipAllTodos()
		await syncTutorialsTodos()
		storeLocalSetting(DISMISSED_KEY, 'true')
		isDismissed = true
	}

	function dismissBanner() {
		storeLocalSetting(DISMISSED_KEY, 'true')
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
			<Button size="xs" variant="accent" onclick={goToTutorials} startIcon={{ icon: GraduationCap }}>
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

