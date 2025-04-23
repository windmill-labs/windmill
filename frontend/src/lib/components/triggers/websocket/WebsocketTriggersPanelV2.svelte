<script lang="ts">
	import WebsocketTriggerEditor from './WebsocketTriggerEditor.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'

	let { selectedTrigger, isFlow, path } = $props()
	let wsTriggerEditor: WebsocketTriggerEditor | undefined = $state(undefined)

	async function openWebsocketTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			wsTriggerEditor?.openNew(isFlow, path)
		} else {
			wsTriggerEditor?.openEdit(selectedTrigger.path, isFlow)
		}
	}

	$effect(() => {
		wsTriggerEditor && openWebsocketTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
	})
</script>

{#if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		WebSocket triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<Description link="https://www.windmill.dev/docs/core_concepts/websocket_triggers">
			WebSocket triggers allow real-time bidirectional communication between your scripts/flows and
			external systems. Each trigger creates a unique WebSocket endpoint.
		</Description>

		<WebsocketTriggerEditor bind:this={wsTriggerEditor} useDrawer={false} />
	</div>
{/if}
