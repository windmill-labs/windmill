<script lang="ts">
	import { type CaptureTriggerKind } from '$lib/gen'
	import { capitalize } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import RouteEditorConfigSection from './RouteEditorConfigSection.svelte'
	import WebsocketEditorConfigSection from './WebsocketEditorConfigSection.svelte'
	import WebhooksConfigSection from './WebhooksConfigSection.svelte'
	import EmailTriggerConfigSection from '../details/EmailTriggerConfigSection.svelte'
	import KafkaTriggersConfigSection from './KafkaTriggersConfigSection.svelte'
	import NatsTriggersConfigSection from './NatsTriggersConfigSection.svelte'

	export let triggerType: CaptureTriggerKind = 'webhook'
	export let cloudDisabled: boolean = false
	export let args: any
	export let isFlow: boolean = false
	export let path: string = ''
	export let data: any = {}
</script>

<div class="flex flex-col gap-4 w-full">
	{#if cloudDisabled}
		<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
			{capitalize(triggerType)} triggers are disabled in the multi-tenant cloud.
		</Alert>
	{:else if triggerType === 'websocket'}
		<WebsocketEditorConfigSection
			can_write={true}
			headless={true}
			bind:url={args.url}
			bind:url_runnable_args={args.url_runnable_args}
			showCapture={false}
		/>
	{:else if triggerType === 'webhook'}
		<WebhooksConfigSection
			{isFlow}
			{path}
			hash={data?.hash}
			token={data?.token}
			{args}
			scopes={data?.scopes}
			showCapture={false}
		/>
	{:else if triggerType === 'http'}
		<RouteEditorConfigSection showCapture={false} can_write={true} bind:args headless />
	{:else if triggerType === 'email'}
		<EmailTriggerConfigSection
			hash={data?.hash}
			token={data?.token}
			{path}
			{isFlow}
			userSettings={data?.userSettings}
			emailDomain={data?.emailDomain}
		/>
	{:else if triggerType === 'kafka'}
		<KafkaTriggersConfigSection headless={true} bind:args staticInputDisabled={false} />
	{:else if triggerType === 'nats'}
		<NatsTriggersConfigSection headless={true} bind:args staticInputDisabled={false} {path} />
	{/if}
</div>
