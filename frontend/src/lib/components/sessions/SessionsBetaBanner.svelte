<script lang="ts">
	import { base } from '$lib/base'
	import { Button } from '$lib/components/common'
	import { classes as alertClasses, icons as alertIcons } from '$lib/components/common/alert/model'
	import { setSessionsBetaOptOut } from '$lib/components/copilot/chat/global/gate'
	import { ExternalLink } from 'lucide-svelte'

	// Prefilled title marks the issue as beta feedback for triage.
	const FEEDBACK_ISSUE_URL =
		'https://github.com/windmill-labs/windmill/issues/new?title=' +
		encodeURIComponent('AI Sessions beta feedback: ')

	// Which chat hosts the banner: the session chat offers the way out of the
	// beta, the legacy docked chat offers the way (back) in.
	let { variant }: { variant: 'session' | 'legacy' } = $props()

	const InfoIcon = alertIcons.info
</script>

<!-- Slim footer sharing the Alert info palette (bg/border/text), minus the
     Alert component's card layout, which is too tall for a one-line bar. The
     wrapper mirrors the composer column of the hosting chat (wide session
     layout vs narrow docked pane) so the bar lines up with the input. -->
<div
	class="w-full mx-auto my-1 shrink-0 {variant === 'session' ? 'max-w-3xl px-6' : 'max-w-2xl px-2'}"
>
	<div
		class="flex flex-wrap items-center justify-center gap-x-1.5 gap-y-0.5 px-3 py-1 rounded-md {alertClasses
			.info.bgClass} border-0 text-xs {alertClasses.info.titleClass}"
	>
		<InfoIcon size={14} class="shrink-0 {alertClasses.info.iconClass}" />
		{#if variant === 'session'}
			<span class="font-medium">AI Sessions is in beta</span>
			<span>·</span>
			<Button
				variant="subtle"
				size="xs"
				btnClasses="!text-accent font-medium !py-0.5"
				href={FEEDBACK_ISSUE_URL}
				target="_blank"
				endIcon={{ icon: ExternalLink }}
			>
				Give feedback
			</Button>
			<span>·</span>
			<Button
				variant="subtle"
				size="xs"
				btnClasses="!py-0.5"
				onclick={() => setSessionsBetaOptOut(true, `${base}/`)}
			>
				Switch back to legacy chat
			</Button>
		{:else}
			<span class="font-medium">Try AI Sessions (beta)</span>
			<span>·</span>
			<Button
				variant="subtle"
				size="xs"
				btnClasses="!py-0.5"
				onclick={() => setSessionsBetaOptOut(false, `${base}/sessions`)}
			>
				Activate
			</Button>
		{/if}
	</div>
</div>
