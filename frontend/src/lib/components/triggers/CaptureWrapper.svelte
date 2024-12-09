<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { CaptureService, type CaptureConfig, type CaptureTriggerKind } from '$lib/gen'
	import { onDestroy } from 'svelte'
	import { capitalize, isObject, sendUserToast, sleep } from '$lib/utils'
	import { isCloudHosted } from '$lib/cloud'
	import Alert from '../common/alert/Alert.svelte'
	import RouteEditorConfigSection from './RouteEditorConfigSection.svelte'
	import WebsocketEditorConfigSection from './WebsocketEditorConfigSection.svelte'
	import WebhooksConfigSection from './WebhooksConfigSection.svelte'
	import CaptureTable from './CaptureTable.svelte'
	import EmailTriggerConfigSection from '../details/EmailTriggerConfigSection.svelte'
	import KafkaTriggersConfigSection from './KafkaTriggersConfigSection.svelte'

	export let isFlow: boolean
	export let path: string
	export let hasPreprocessor: boolean
	export let canHavePreprocessor: boolean
	export let captureType: CaptureTriggerKind = 'webhook'
	export let captureActive = false
	export let data: any = {}
	export let connectionInfo:
		| {
				status: 'connected' | 'disconnected' | 'error'
				message?: string
		  }
		| undefined = undefined
	export let args: Record<string, any> = {}

	let isValid = true

	$: captureType && (isValid = true)

	export async function setConfig() {
		await CaptureService.setCaptureConfig({
			requestBody: {
				trigger_kind: captureType,
				path,
				is_flow: isFlow,
				trigger_config: args && Object.keys(args).length > 0 ? args : undefined
			},
			workspace: $workspaceStore!
		})
	}

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

		if ((captureType === 'websocket' || captureType === 'kafka') && captureActive) {
			const config = captureConfigs[captureType]
			if (config && config.error) {
				const serverEnabled = getServerEnabled(config)
				if (!serverEnabled) {
					sendUserToast('Capture was stopped because of error: ' + config.error, true)
					captureActive = false
				}
			}
		}
	}
	getCaptureConfigs()

	let refreshCaptures: () => Promise<void>

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
			refreshCaptures()
			i++
			await sleep(1000)
		}
	}

	function stopAndSetDefaultArgs() {
		captureActive = false
		if (captureType in captureConfigs) {
			const triggerConfig = captureConfigs[captureType].trigger_config
			args = isObject(triggerConfig) ? triggerConfig : {}
		} else if (captureType === 'kafka') {
			args = {
				brokers: [''],
				topics: [''],
				group_id: `windmill_consumer-${$workspaceStore}-${path.replaceAll('/', '__')}`
			}
		} else {
			args = {}
		}
	}
	$: captureType && stopAndSetDefaultArgs()

	onDestroy(() => {
		captureActive = false
	})

	function getServerEnabled(config: CaptureConfig) {
		return (
			!!config.last_server_ping &&
			new Date(config.last_server_ping).getTime() > new Date().getTime() - 15 * 1000
		)
	}

	export async function handleCapture() {
		if (!captureActive) {
			await setConfig()
			capture()
		} else {
			captureActive = false
		}
	}

	let config: CaptureConfig | undefined
	$: config = captureConfigs[captureType]

	$: cloudDisabled = (captureType === 'websocket' || captureType === 'kafka') && isCloudHosted()

	function updateConnectionInfo(
		captureType: CaptureTriggerKind,
		config: CaptureConfig | undefined,
		captureActive: boolean
	) {
		if ((captureType === 'websocket' || captureType === 'kafka') && config && captureActive) {
			const serverEnabled = getServerEnabled(config)
			const message = serverEnabled
				? 'Websocket is connected'
				: `Websocket is not connected${config.error ? ': ' + config.error : ''}`
			connectionInfo = {
				status: serverEnabled ? 'connected' : 'disconnected',
				message
			}
		} else {
			connectionInfo = undefined
		}
	}
	$: updateConnectionInfo(captureType, config, captureActive)
</script>

<div class="flex flex-col gap-4 w-full">
	{#if cloudDisabled}
		<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
			{capitalize(captureType)} triggers are disabled in the multi-tenant cloud.
		</Alert>
	{:else}
		{#if captureType === 'websocket'}
			<WebsocketEditorConfigSection
				url={''}
				can_write={true}
				headless={true}
				bind:args
				showCapture={captureActive}
			/>
		{:else if captureType === 'webhook'}
			<WebhooksConfigSection
				{isFlow}
				{path}
				hash={data?.hash}
				token={data?.token}
				{args}
				scopes={data?.scopes}
				showCapture={captureActive}
			/>
		{:else if captureType === 'http'}
			<RouteEditorConfigSection
				{path}
				showCapture={captureActive}
				can_write={true}
				bind:args
				headless
			/>
		{:else if captureType === 'email'}
			<EmailTriggerConfigSection
				hash={data?.hash}
				token={data?.token}
				{path}
				{isFlow}
				userSettings={data?.userSettings}
				emailDomain={data?.emailDomain}
				{captureActive}
			/>
		{:else if captureType === 'kafka'}
			<KafkaTriggersConfigSection headless={true} bind:args staticInputDisabled={false} />
		{/if}

		<CaptureTable
			{captureType}
			{hasPreprocessor}
			{canHavePreprocessor}
			{isFlow}
			{path}
			bind:refreshCaptures
			hideCapturesWhenEmpty={true}
		/>
	{/if}
</div>
