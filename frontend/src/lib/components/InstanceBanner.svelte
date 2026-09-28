<script lang="ts">
	import { Button } from '$lib/components/common'
	import { classes as alertClasses, icons as alertIcons } from '$lib/components/common/alert/model'
	import { ExternalLink, X } from 'lucide-svelte'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { isInstanceBannerVisible, resolveInstanceBanner } from './instanceBanner'
	import { instanceUi } from '$lib/instanceUi'

	// Fetched by `InstanceUiSync`, which also carries the accent color.
	let banner = $derived(resolveInstanceBanner($instanceUi?.instance_banner))

	// Per-viewer, per-announcement. Holds the fingerprint of the dismissed banner, so
	// a new announcement shows up again for everyone who dismissed the previous one.
	const dismissed = useLocalStorageValue<string>('instance_banner_dismissed', '', 'string')

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
