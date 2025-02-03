<script lang="ts">
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import HighlightTheme from '$lib/components/HighlightTheme.svelte'
	import TriggersEditorSection from '../TriggersEditorSection.svelte'

	export let token: string
	export let args: Record<string, any> = {}
	export let scopes: string[] = []
	export let isFlow: boolean = false
	export let hash: string | undefined = undefined
	export let path: string
	export let newItem: boolean = false
	export let isEditor: boolean = false
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false

	$: data = {
		hash,
		token,
		scopes,
		args
	}
</script>

<HighlightTheme />

<div class="flex flex-col w-full gap-4">
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

	<TriggersEditorSection
		on:applyArgs
		on:addPreprocessor
		on:refreshCaptures
		on:updateSchema
		on:testWithArgs
		cloudDisabled={false}
		triggerType="webhook"
		{isFlow}
		{data}
		noSave
		{path}
		{isEditor}
		{canHavePreprocessor}
		{hasPreprocessor}
		{newItem}
		alwaysOpened={true}
	/>
</div>
