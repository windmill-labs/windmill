<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import { yaml } from 'svelte-highlight/languages'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import { slackAppManifest } from './slackAppManifest'

	/** How to create the Slack app Windmill connects through, for the instance or a workspace. */
	let { instance = false }: { instance?: boolean } = $props()
</script>

<div class="flex flex-col gap-2 text-xs text-primary">
	<span>
		Create a Slack app <a href="https://api.slack.com/apps?new_app=1" target="_blank"
			>in the Slack API console <ExternalLink size={12} class="inline-block" /></a
		>: pick "From a manifest", then YAML, and paste the manifest below. Then copy the Client ID,
		Client Secret and Signing Secret here{#if instance}
			{' '}(or set the Signing Secret as the server's <code>SLACK_SIGNING_SECRET</code> environment variable){/if}.
		Socket Mode and token rotation must stay off. More in the
		<a href="https://www.windmill.dev/docs/misc/setup_oauth#slack" target="_blank"
			>Windmill docs <ExternalLink size={12} class="inline-block" /></a
		>.
	</span>
	<div class="flex flex-col max-h-80 [&>div]:min-h-0">
		<CopyableCodeBlock code={slackAppManifest(window.location.origin)} language={yaml} />
	</div>
</div>
