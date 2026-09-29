<script lang="ts">
	import { base } from '$lib/base'
	import { Button } from '$lib/components/common'
	import { classes as alertClasses, icons as alertIcons } from '$lib/components/common/alert/model'
	import { clearSessionsBetaOptOut } from '$lib/components/copilot/chat/global/gate'

	const InfoIcon = alertIcons.info
</script>

<!-- Shown in the legacy docked chat, the way (back) into AI Sessions. Slim footer
     sharing the Alert info palette (bg/border/text), minus the Alert component's
     card layout, which is too tall for a one-line bar. The wrapper mirrors the
     docked pane's composer column so the bar lines up with the input. -->
<div class="w-full mx-auto my-1 shrink-0 max-w-2xl px-2">
	<div
		class="flex flex-wrap items-center justify-center gap-x-1.5 gap-y-0.5 px-3 py-1 rounded-md {alertClasses
			.info.bgClass} border-0 text-xs {alertClasses.info.titleClass}"
	>
		<InfoIcon size={14} class="shrink-0 {alertClasses.info.iconClass}" />
		<span class="font-medium">Try AI Sessions (beta)</span>
		<span>·</span>
		<Button
			variant="subtle"
			unifiedSize="xs"
			onclick={() => clearSessionsBetaOptOut(`${base}/sessions`)}
		>
			Activate
		</Button>
	</div>
</div>
