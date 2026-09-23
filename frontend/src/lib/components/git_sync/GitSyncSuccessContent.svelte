<!--
@component
What the user is told once a git sync connection is saved: shown as the last step of the
setup dialog, and in the standalone modal for a connection saved outside it.
-->
<script lang="ts">
	import { Alert } from '$lib/components/common'
	import { CheckCircle2 } from 'lucide-svelte'

	interface Props {
		savedWithoutInit?: boolean
		autoPullOn?: boolean
	}

	let { savedWithoutInit = false, autoPullOn = false }: Props = $props()
</script>

<!-- The notices belong at the top, where the steps before them put their own; what the dialog
     ends on is centered in what is left. -->
<div class="flex flex-col gap-4 h-full">
	{#if savedWithoutInit}
		<Alert type="info" title="Repository saved without initialization">
			Only new changes will be pushed to this repository. What this workspace holds today has not
			been pushed to it.
		</Alert>
	{/if}

	<!-- Nothing here about turning pull on: the step that offers the toggle says what it does,
	     and by now the choice has been made. -->
	{#if autoPullOn}
		<Alert type="success" title="Pull from Git is on">
			New commits to the tracked branch deploy into this workspace automatically. You can adjust
			this anytime on the repository's settings.
		</Alert>
	{/if}

	<!-- `pb`: centered in what the notices leave, it would otherwise read as sitting low on the
	     step rather than in the middle of it. -->
	<div class="flex-1 min-h-24 pb-16 flex flex-col items-center justify-center gap-2 text-center">
		<CheckCircle2 class="h-8 w-8 text-green-600" />
		<div class="flex flex-col gap-1">
			<h3 class="text-lg font-semibold text-primary">Git sync connection saved successfully!</h3>
			<p class="text-sm text-secondary">
				Your repository is now configured to receive changes from Windmill.
			</p>
		</div>
	</div>
</div>
