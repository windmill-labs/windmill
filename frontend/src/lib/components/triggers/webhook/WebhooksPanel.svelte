<script lang="ts">
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import HighlightTheme from '$lib/components/HighlightTheme.svelte'
	import WebhooksConfigSection from './WebhooksConfigSection.svelte'
	import { Section } from '$lib/components/common'

	interface Props {
		token: string
		args?: Record<string, any>
		scopes?: string[]
		isFlow?: boolean
		hash?: string | undefined
		path: string
		newItem?: boolean
	}

	let {
		token,
		args = {},
		scopes = [],
		isFlow = false,
		hash = undefined,
		path,
		newItem = false
	}: Props = $props()
</script>

<HighlightTheme />

<Section label="Webhooks" class="flex flex-col gap-4">
	<Description link="https://www.windmill.dev/docs/core_concepts/webhooks">
		Webhooks trigger scripts or flows via HTTP requests. Each webhook can be configured to run
		synchronously or asynchronously. You can secure webhooks using tokens with specific permissions.
	</Description>
	{#if newItem}
		<Alert type="warning" title="Attached to a deployed path">
			The webhooks are only valid for a given path and will only trigger the deployed version of the
			{isFlow ? 'flow' : 'script'}.
		</Alert>
	{/if}
	<WebhooksConfigSection {isFlow} {path} {hash} {token} runnableArgs={args} {scopes} />
</Section>
