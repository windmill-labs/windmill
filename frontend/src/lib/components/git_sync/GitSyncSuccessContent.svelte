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
		/** Whether to say what pulling off means. The setup dialog says it where it offers the
		 * toggle, so only the standalone modal — whose save came from a card that says it
		 * nowhere — needs it here. */
		explainAutoPullOff?: boolean
		/** Set when pulling is on but its webhook could not be registered. */
		webhookError?: string
	}

	let {
		savedWithoutInit = false,
		autoPullOn = false,
		explainAutoPullOff = false,
		webhookError = undefined
	}: Props = $props()
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

	{#if autoPullOn && webhookError}
		<Alert type="warning" title="Pull from Git is on, but falling back to polling">
			{webhookError}
		</Alert>
	{:else if autoPullOn}
		<Alert type="success" title="Pull from Git is on">
			New commits to the tracked branch deploy into this workspace automatically. You can adjust
			this anytime on the repository's settings.
		</Alert>
	{:else if explainAutoPullOff}
		<Alert
			type="warning"
			title="Deploy changes from Git back to Windmill"
			documentationLink="https://www.windmill.dev/docs/advanced/deploy_gh_gl#github-actions-setup"
		>
			Turn on "Automatically deploy changes from Git" on the repository to have Windmill pull new
			commits into this workspace for you. Prefer to control deployment from your own pipeline
			(tests, custom gating, deploy on PR merge)? Set up GitHub Actions or a similar CI/CD workflow
			instead.
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
