<script lang="ts">
	import { Button } from '$lib/components/common'
	import { GraduationCap, X } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'
	import { onMount } from 'svelte'

	const DISMISSED_KEY = 'tutorial_banner_dismissed'
	let isDismissed = $state(false)

	onMount(() => {
		// Check if banner has been dismissed
		isDismissed = getLocalSetting(DISMISSED_KEY) === 'true'
	})

	function dismissBanner() {
		storeLocalSetting(DISMISSED_KEY, 'true')
		isDismissed = true
		sendUserToast(
			'You can still access tutorials from the Tutorials page in the main menu or in the Help submenu.',
			false,
			[],
			undefined,
			5000
		)
	}

	function goToTutorials() {
		goto(`${base}/tutorials`)
	}
</script>

{#if !isDismissed}
	<!-- Previous larger version for comparison -->
	<div
		class="flex items-center justify-between gap-4 p-4 rounded-lg border bg-surface border-light mb-4"
	>
		<div class="flex items-center gap-3 flex-1">
			<div class="flex-shrink-0 p-2 rounded-lg bg-primary/10 dark:bg-primary/20">
				<GraduationCap size={20} class="text-primary" />
			</div>
			<div class="flex-1 min-w-0">
				<h3 class="text-sm font-semibold text-emphasis mb-1">Learn with interactive tutorials</h3>
				<p class="text-xs text-secondary">
					Get started quickly with step-by-step guides on building flows, scripts, and more.
				</p>
			</div>
		</div>
		<div class="flex items-center gap-2 flex-shrink-0">
			<Button size="xs" variant="accent" onclick={goToTutorials} startIcon={{ icon: GraduationCap }}>
				View Tutorials
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

