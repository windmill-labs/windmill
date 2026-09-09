<script lang="ts">
	import { Button } from '$lib/components/common'
	import { classes as alertClasses, icons as alertIcons } from '$lib/components/common/alert/model'
	import { ExternalLink, X } from 'lucide-svelte'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import {
		fetchInstanceBanner,
		isInstanceBannerVisible,
		type ResolvedInstanceBanner
	} from './instanceBanner'
	import { instanceSettingsSaved } from './instanceSettings'

	// Mounted only on the managed cloud (see the render site in the logged layout), so the
	// poll costs nothing anywhere else. An announcement is only worth broadcasting while it
	// is current, hence polling rather than waiting for the next full page load: a session
	// left open all day is exactly the one that needs to hear about the maintenance window.
	const POLL_MS = 60_000

	let banner = $state<ResolvedInstanceBanner | undefined>(undefined)

	// Per-viewer, per-announcement. Holds the fingerprint of the dismissed banner, so
	// a new announcement shows up again for everyone who dismissed the previous one.
	const dismissed = useLocalStorageValue<string>('instance_banner_dismissed', '', 'string')

	// The poll, the tab-focus refresh and the post-save refresh all call `load()` and can
	// overlap. Responses are not ordered, so without this a slow earlier fetch lands last
	// and puts a retracted announcement back on screen until the next successful poll.
	let latestLoad = 0

	async function load() {
		const generation = ++latestLoad
		try {
			const next = await fetchInstanceBanner()
			if (generation === latestLoad) banner = next
		} catch (e) {
			// Keep whatever is on screen: a transient failure must not silently retract
			// an announcement that is still in force.
			console.warn('Could not fetch the instance banner', e)
		}
	}

	$effect(() => {
		// Re-runs on save so a superadmin who just published an announcement sees it
		// immediately rather than on the next poll or page load.
		$instanceSettingsSaved
		load()
		const interval = setInterval(() => {
			if (!document.hidden) load()
		}, POLL_MS)
		const onVisible = () => {
			if (!document.hidden) load()
		}
		document.addEventListener('visibilitychange', onVisible)
		return () => {
			clearInterval(interval)
			document.removeEventListener('visibilitychange', onVisible)
		}
	})

	let shown = $derived(isInstanceBannerVisible(banner, dismissed.val))
	let palette = $derived(alertClasses[banner?.severity ?? 'info'])
	let Icon = $derived(alertIcons[banner?.severity ?? 'info'])
</script>

{#if shown && banner}
	<!-- Sits in the content column above the page, in flow rather than overlaid, so it
	     pushes the app down instead of covering the top of whatever page is open. -->
	<div
		class="shrink-0 px-4 py-1.5 flex items-center justify-center gap-x-3 gap-y-1 flex-wrap {palette.bgClass}"
		role="status"
	>
		<Icon size={16} class="shrink-0 {palette.iconClass}" />
		<span class="text-xs font-medium {palette.titleClass}">{banner.message}</span>
		{#if banner.link}
			<Button
				variant="subtle"
				unifiedSize="xs"
				href={banner.link}
				target="_blank"
				endIcon={{ icon: ExternalLink }}
			>
				{banner.linkLabel}
			</Button>
		{/if}
		{#if banner.dismissible}
			<Button
				variant="subtle"
				unifiedSize="xs"
				iconOnly
				startIcon={{ icon: X }}
				title="Dismiss"
				aria-label="Dismiss announcement"
				onclick={() => (dismissed.val = banner?.fingerprint ?? '')}
			/>
		{/if}
	</div>
{/if}
