<script lang="ts">
	import { type CaptureTriggerKind } from '$lib/gen'
	import { capitalize } from '$lib/utils'
	import Alert from '../common/alert/Alert.svelte'
	import RouteEditorConfigSection from './http/RouteEditorConfigSection.svelte'
	import WebsocketEditorConfigSection from './websocket/WebsocketEditorConfigSection.svelte'
	import WebhooksConfigSection from './webhook/WebhooksConfigSection.svelte'
	import EmailTriggerConfigSection from '../details/EmailTriggerConfigSectionV2.svelte'
	import KafkaTriggersConfigSection from './kafka/KafkaTriggersConfigSection.svelte'
	import NatsTriggersConfigSection from './nats/NatsTriggersConfigSection.svelte'
	import MqttEditorConfigSection from './mqtt/MqttEditorConfigSection.svelte'
	import SqsTriggerEditorConfigSection from './sqs/SqsTriggerEditorConfigSection.svelte'
	import PostgresEditorConfigSection from './postgres/PostgresEditorConfigSection.svelte'

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
	{:else if triggerType === 'postgres'}
		<PostgresEditorConfigSection
			can_write={true}
			headless={true}
			showCapture={false}
			bind:publication={args.publication}
			bind:postgres_resource_path={args.postgres_resource_path}
		/>
	{:else if triggerType === 'webhook'}
		<WebhooksConfigSection
			{isFlow}
			{path}
			hash={data?.hash}
			token={data?.token}
			runnableArgs={data?.args}
			scopes={data?.scopes}
			showCapture={false}
		/>
	{:else if triggerType === 'http'}
		<RouteEditorConfigSection
			showCapture={false}
			can_write={true}
			bind:route_path={args.route_path}
			bind:http_method={args.http_method}
			bind:raw_string={args.raw_string}
			bind:wrap_body={args.wrap_body}
			capture_mode={true}
			headless
		/>
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
		<KafkaTriggersConfigSection headless={true} bind:args staticInputDisabled={false} {path} />
	{:else if triggerType === 'nats'}
		<NatsTriggersConfigSection headless={true} bind:args staticInputDisabled={false} {path} />
	{:else if triggerType === 'mqtt'}
		<MqttEditorConfigSection showCapture={false} headless={true} can_write={true} />
	{:else if triggerType === 'sqs'}
		<SqsTriggerEditorConfigSection
			bind:queue_url={args.queue_url}
			bind:aws_resource_path={args.aws_resource_path}
			bind:message_attributes={args.message_attributes}
			headless={true}
			can_write={true}
			showCapture={false}
		/>
	{/if}
</div>
