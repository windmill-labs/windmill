<script lang="ts">
	import { Alert } from '$lib/components/common'
	import HighlightTheme from '../HighlightTheme.svelte'
	import TriggersEditorSection from './TriggersEditorSection.svelte'

	export let token: string
	export let args: any
	export let scopes: string[] = []
	export let isFlow: boolean = false
	export let hash: string | undefined = undefined
	export let path: string
	export let newItem: boolean = false

	let data: any = {
		hash,
		token,
		scopes,
		args,
		path
	}
</script>

<HighlightTheme />

<div class="flex flex-col w-full gap-4">
	{#if newItem}
		<div class="mt-10" />
		<Alert type="warning" title="Attached to a deployed path">
			The webhooks are only valid for a given path and will only trigger the deployed version of the
			{isFlow ? 'flow' : 'script'}.
		</Alert>
	{/if}

	<TriggersEditorSection cloudDisabled={false} captureType="webhook" {isFlow} {data} noSave />
</div>
