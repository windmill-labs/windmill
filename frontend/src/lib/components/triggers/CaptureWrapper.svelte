<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { CaptureService, type CaptureConfig, type CaptureTriggerKind } from '$lib/gen'
	import { onDestroy } from 'svelte'
	import { capitalize, isObject, sendUserToast, sleep } from '$lib/utils'
	import { isCloudHosted } from '$lib/cloud'
	import Alert from '../common/alert/Alert.svelte'
	import RouteEditorConfigSection from './http/RouteEditorConfigSection.svelte'
	import WebsocketEditorConfigSection from './websocket/WebsocketEditorConfigSection.svelte'
	import WebhooksConfigSection from './webhook/WebhooksConfigSection.svelte'
	import EmailTriggerConfigSection from '../details/EmailTriggerConfigSection.svelte'
	import KafkaTriggersConfigSection from './kafka/KafkaTriggersConfigSection.svelte'
	import type { ConnectionInfo } from '../common/alert/ConnectionIndicator.svelte'
	import type { CaptureInfo } from './CaptureSection.svelte'
	import CaptureTable from './CaptureTable.svelte'
	import NatsTriggersConfigSection from './nats/NatsTriggersConfigSection.svelte'
	import MqttEditorConfigSection from './mqtt/MqttEditorConfigSection.svelte'
	import SqsTriggerEditorConfigSection from './sqs/SqsTriggerEditorConfigSection.svelte'
	import PostgresEditorConfigSection from './postgres/PostgresEditorConfigSection.svelte'
	import { invalidRelations } from './postgres/utils'
	import { DEFAULT_V3_CONFIG, DEFAULT_V5_CONFIG } from './mqtt/constant'
	import GcpTriggerEditorConfigSection from './gcp/GcpTriggerEditorConfigSection.svelte'

	export let isFlow: boolean
	export let path: string
	export let hasPreprocessor: boolean
	export let canHavePreprocessor: boolean
	export let captureType: CaptureTriggerKind = 'webhook'
	export let showCapture = false
	export let data: any = {}
	export let connectionInfo: ConnectionInfo | undefined = undefined
	export let loading = false
	export let args: Record<string, any> = {}
	export let captureTable: CaptureTable | undefined = undefined

	export async function setConfig(): Promise<boolean> {
		if (captureType === 'postgres') {
			if (!args?.publication?.table_to_track) {
				sendUserToast('Table to track must be set', true)
				return false
			}
			if (
				invalidRelations(args.publication.table_to_track, {
					showError: true,
					trackSchemaTableError: true
				}) === true
			) {
				return false
			}
		}
		try {
			loading = true
			args = await CaptureService.setCaptureConfig({
				requestBody: {
					trigger_kind: captureType,
					path,
					is_flow: isFlow,
					trigger_config: args && Object.keys(args).length > 0 ? args : undefined
				},
				workspace: $workspaceStore!
			})
			loading = false
			return true
		} catch (error) {
			loading = false
			sendUserToast(error.body, true)
			return false
		}
	}

	let captureActive = false

	let captureConfigs: {
		[key: string]: CaptureConfig
	} = {}

	const STREAMING_CAPTURES = ['mqtt', 'sqs', 'websocket', 'postgres', 'kafka', 'nats', 'gcp']

	async function getCaptureConfigs() {
		const captureConfigsList = await CaptureService.getCaptureConfigs({
			workspace: $workspaceStore!,
			runnableKind: isFlow ? 'flow' : 'script',
			path
		})
		captureConfigs = captureConfigsList.reduce((acc, c) => {
			acc[c.trigger_kind] = c
			return acc
		}, {})
		if (isStreamingCapture() && captureActive) {
			const config = captureConfigs[captureType]
			if (config && config.error) {
				const serverEnabled = getServerEnabled(config)
				if (!serverEnabled) {
					sendUserToast('Capture was stopped because of error: ' + config.error, true)
					captureActive = false
				}
			}
		}
		return captureConfigs
	}
	getCaptureConfigs().then((captureConfigs) => setDefaultArgs(captureConfigs))

	async function capture() {
		let i = 0
		captureActive = true
		while (captureActive) {
			if (i % 3 === 0) {
				await CaptureService.pingCaptureConfig({
					workspace: $workspaceStore!,
					triggerKind: captureType,
					runnableKind: isFlow ? 'flow' : 'script',
					path
				})
				await getCaptureConfigs()
			}
			i++
			await sleep(1000)
			captureTable?.loadCaptures(true)
		}
	}

	let ready = false
	function setDefaultArgs(captureConfigs: { [key: string]: CaptureConfig }) {
		if (captureType in captureConfigs) {
			const triggerConfig = captureConfigs[captureType].trigger_config
			args = isObject(triggerConfig) ? triggerConfig : {}
		} else {
			switch (captureType) {
				case 'mqtt':
					//define these field so any reactive statement that may use them will not crash trying to access their property
					args = {
						v3_config: DEFAULT_V3_CONFIG,
						v5_config: DEFAULT_V5_CONFIG,
						client_version: 'v5',
						subscribe_topics: []
					}
					break

				default:
					args = {}
			}
		}
		ready = true
	}

	onDestroy(() => {
		captureActive = false
	})

	function getServerEnabled(config: CaptureConfig) {
		return (
			!!config.last_server_ping &&
			new Date(config.last_server_ping).getTime() > new Date().getTime() - 15 * 1000
		)
	}

	export async function handleCapture(e: CustomEvent<{ disableOnly?: boolean }>) {
		if (captureActive || e.detail.disableOnly) {
			captureActive = false
		} else {
			const configSet = await setConfig()
			if (configSet) {
				capture()
			}
		}
	}

	let config: CaptureConfig | undefined
	$: config = captureConfigs[captureType]

	let cloudDisabled = STREAMING_CAPTURES.includes(captureType) && isCloudHosted()

	function isStreamingCapture() {
		if (captureType === 'gcp' && args.delivery_type === 'push') {
			return false
		}
		return ['mqtt', 'sqs', 'websocket', 'postgres', 'kafka', 'nats', 'gcp'].includes(captureType)
	}

	function updateConnectionInfo(config: CaptureConfig | undefined, captureActive: boolean) {
		if (isStreamingCapture() && config && captureActive) {
			const serverEnabled = getServerEnabled(config)
			const connected = serverEnabled && !config.error
			const message = connected
				? `Connected`
				: `Not connected${config.error ? ': ' + config.error : ''}`
			connectionInfo = {
				connected,
				message
			}
		} else {
			connectionInfo = undefined
		}
	}
	$: updateConnectionInfo(config, captureActive)

	let captureInfo: CaptureInfo
	$: captureInfo = {
		active: captureActive,
		hasPreprocessor,
		canHavePreprocessor,
		isFlow,
		path,
		connectionInfo,
		loading: loading
	}

	$: args && (captureActive = false)
</script>

{#key ready}
	<div class="flex flex-col gap-4 w-full">
		{#if cloudDisabled}
			<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
				{capitalize(captureType)} triggers are disabled in the multi-tenant cloud.
			</Alert>
		{:else if captureType === 'websocket'}
			<WebsocketEditorConfigSection
				can_write={true}
				headless={true}
				bind:url={args.url}
				bind:url_runnable_args={args.url_runnable_args}
				{showCapture}
				{captureInfo}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'postgres'}
			<PostgresEditorConfigSection
				bind:postgres_resource_path={args.postgres_resource_path}
				bind:publication={args.publication}
				{showCapture}
				{captureInfo}
				can_write={true}
				headless={true}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'webhook'}
			<WebhooksConfigSection
				{isFlow}
				{path}
				hash={data?.hash}
				token={data?.token}
				runnableArgs={data?.args}
				scopes={data?.scopes}
				{showCapture}
				{captureInfo}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'http'}
			<RouteEditorConfigSection
				{showCapture}
				can_write={true}
				runnableArgs={data?.args}
				bind:route_path={args.route_path}
				bind:http_method={args.http_method}
				bind:raw_string={args.raw_string}
				bind:wrap_body={args.wrap_body}
				capture_mode={true}
				headless
				{captureInfo}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'email'}
			<EmailTriggerConfigSection
				hash={data?.hash}
				token={data?.token}
				{path}
				{isFlow}
				userSettings={data?.userSettings}
				emailDomain={data?.emailDomain}
				{showCapture}
				{captureInfo}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'kafka'}
			<KafkaTriggersConfigSection
				headless={true}
				{path}
				bind:args
				staticInputDisabled={false}
				{showCapture}
				{captureInfo}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'nats'}
			<NatsTriggersConfigSection
				headless={true}
				bind:args
				{path}
				staticInputDisabled={false}
				{showCapture}
				{captureInfo}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
			/>
		{:else if captureType === 'mqtt'}
			<MqttEditorConfigSection
				can_write={true}
				headless={true}
				{showCapture}
				{captureInfo}
				bind:v3_config={args.v3_config}
				bind:v5_config={args.v5_config}
				bind:client_version={args.client_version}
				bind:subscribe_topics={args.subscribe_topics}
				bind:mqtt_resource_path={args.mqtt_resource_path}
				bind:client_id={args.client_id}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'sqs'}
			<SqsTriggerEditorConfigSection
				can_write={true}
				headless={true}
				bind:queue_url={args.queue_url}
				bind:aws_resource_path={args.aws_resource_path}
				bind:message_attributes={args.message_attributes}
				bind:aws_auth_resource_type={args.aws_auth_resource_type}
				{showCapture}
				{captureInfo}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'gcp'}
			<GcpTriggerEditorConfigSection
				can_write={true}
				headless={true}
				bind:gcp_resource_path={args.gcp_resource_path}
				bind:topic_id={args.topic_id}
				bind:subscription_id={args.subscription_id}
				bind:cloud_subscription_id={args.subscription_id}
				bind:create_update_subscription_id={args.subscription_id}
				bind:delivery_config={args.delivery_config}
				bind:delivery_type={args.delivery_type}
				bind:subscription_mode={args.subscription_mode}
				bind:base_endpoint={args.base_endpoint}
				{showCapture}
				{captureInfo}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{/if}
	</div>
{/key}
