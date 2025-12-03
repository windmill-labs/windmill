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

