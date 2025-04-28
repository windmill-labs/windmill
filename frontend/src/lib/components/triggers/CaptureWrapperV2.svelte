<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { CaptureService, type CaptureConfig, type CaptureTriggerKind } from '$lib/gen'
	import { onDestroy } from 'svelte'
	import { capitalize, isObject, sendUserToast, sleep } from '$lib/utils'
	import { isCloudHosted } from '$lib/cloud'
	import Alert from '../common/alert/Alert.svelte'
	import RouteCapture from './http/RouteCapture.svelte'
	import type { ConnectionInfo } from '../common/alert/ConnectionIndicator.svelte'
	import type { CaptureInfo } from './CaptureSection.svelte'
	import CaptureTable from './CaptureTable.svelte'
	import MqttEditorConfigSection from './mqtt/MqttEditorConfigSection.svelte'
	import SqsTriggerEditorConfigSection from './sqs/SqsTriggerEditorConfigSection.svelte'

	import { invalidRelations } from './postgres/utils'
	import { DEFAULT_V3_CONFIG, DEFAULT_V5_CONFIG } from './mqtt/constant'
	import WebhooksCapture from './webhook/WebhooksCapture.svelte'
	import EmailTriggerCaptures from '../details/EmailTriggerCaptures.svelte'
	import WebsocketCapture from './websocket/WebsocketCapture.svelte'
	import PostgresCapture from './postgres/PostgresCapture.svelte'
	import KafkaCapture from './kafka/KafkaCapture.svelte'
	import NatsCapture from './nats/NatsCapture.svelte'

	export let isFlow: boolean
	export let path: string
	export let hasPreprocessor: boolean
	export let canHavePreprocessor: boolean
	export let captureType: CaptureTriggerKind = 'webhook'
	export let showCapture = false
	export let data: any = {}
	export let connectionInfo: ConnectionInfo | undefined = undefined
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
			await CaptureService.setCaptureConfig({
				requestBody: {
					trigger_kind: captureType,
					path,
					is_flow: isFlow,
					trigger_config: args && Object.keys(args).length > 0 ? args : undefined
				},
				workspace: $workspaceStore!
			})
			return true
		} catch (error) {
			sendUserToast(error.body, true)
			return false
		}
	}

	let captureActive = false

	let captureConfigs: {
		[key: string]: CaptureConfig
	} = {}
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
		const streamingCapture = ['postgres', 'websocket', 'kafka', 'sqs', 'mqtt']
		if (streamingCapture.includes(captureType) && captureActive) {
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
	const streamingCaptures = ['mqtt', 'sqs', 'websocket', 'postgres', 'kafka', 'nats']
	let cloudDisabled = streamingCaptures.includes(captureType) && isCloudHosted()

	function updateConnectionInfo(config: CaptureConfig | undefined, captureActive: boolean) {
		if (streamingCaptures.includes(captureType) && config && captureActive) {
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
		connectionInfo
	}

	$: args && (captureActive = false)
</script>

{#key ready}
	<div class="flex flex-col gap-4 w-full h-full">
		{#if cloudDisabled}
			<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
				{capitalize(captureType)} triggers are disabled in the multi-tenant cloud.
			</Alert>
		{:else if captureType === 'websocket'}
			<WebsocketCapture
				isValid={args.isValid}
				{captureInfo}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'postgres'}
			<PostgresCapture
				{captureInfo}
				bind:captureTable
				postgres_resource_path={args.postgres_resource_path}
				{hasPreprocessor}
				{isFlow}
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'webhook'}
			<WebhooksCapture
				{hasPreprocessor}
				{isFlow}
				{path}
				runnableArgs={data?.args}
				{captureInfo}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'http'}
			<RouteCapture
				runnableArgs={data?.args}
				route_path={args.route_path}
				http_method={args.http_method}
				isValid={args.isValid}
				{captureInfo}
				{hasPreprocessor}
				{isFlow}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'email'}
			<EmailTriggerCaptures
				{path}
				{isFlow}
				emailDomain={data?.emailDomain}
				{captureInfo}
				{hasPreprocessor}
				bind:captureTable
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'kafka'}
			<KafkaCapture
				isValid={args.isValid}
				{captureInfo}
				bind:captureTable
				{hasPreprocessor}
				{isFlow}
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
			/>
		{:else if captureType === 'nats'}
			<NatsCapture
				isValid={args.isValid}
				{captureInfo}
				bind:captureTable
				{hasPreprocessor}
				{isFlow}
				on:applyArgs
				on:updateSchema
				on:addPreprocessor
				on:captureToggle={handleCapture}
				on:testWithArgs
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
