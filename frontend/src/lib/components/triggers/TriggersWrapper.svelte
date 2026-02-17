<script lang="ts">
	import RoutesPanel from './http/RoutesPanel.svelte'
	import WebhooksPanel from './webhook/WebhooksPanel.svelte'
	import EmailTriggerPanel from './email/EmailTriggerPanel.svelte'
	import DefaultEmailPanel from './email/DefaultEmailPanel.svelte'
	import SchedulePanel from '$lib/components/SchedulePanel.svelte'
	import PostgresTriggersPanel from './postgres/PostgresTriggersPanel.svelte'
	import KafkaTriggerPanel from './kafka/KafkaTriggersPanel.svelte'
	import NatsTriggersPanel from './nats/NatsTriggersPanel.svelte'
	import MqttTriggerPanel from './mqtt/MqttTriggersPanel.svelte'
	import SqsTriggerPanel from './sqs/SqsTriggerPanel.svelte'
	import GcpTriggerPanel from './gcp/GcpTriggerPanel.svelte'
	import ScheduledPollPanel from './scheduled/ScheduledPollPanel.svelte'
	import WebsocketTriggersPanel from './websocket/WebsocketTriggersPanel.svelte'
	import { triggerIconMap, type Trigger } from './utils'
	import ClipboardPanel from '../details/ClipboardPanel.svelte'
	import CliHelpBox from '../CliHelpBox.svelte'
	import TriggerLabel from './TriggerLabel.svelte'
	import NativeTriggersPanel from './native/NativeTriggersPanel.svelte'

	interface Props {
		selectedTrigger: Trigger
		isFlow: boolean
		initialPath: string
		fakeInitialPath: string
		currentPath: string
		runnableVersion?: string
		isDeployed: boolean
		small: boolean
		args: Record<string, any>
		newItem: boolean
		schema: Record<string, any> | undefined
		isEditor?: boolean
		onConfigChange?: (cfg: Record<string, any>, canSave: boolean, updated: boolean) => void
		onCaptureConfigChange?: (cfg: Record<string, any>, isValidConfig: boolean) => void
		onUpdate?: (path: string) => void
		onDelete?: () => void
		onReset?: () => void
		onEmailDomain: (domain: string) => void
	}

	let {
		selectedTrigger,
		isFlow = false,
		initialPath,
		fakeInitialPath,
		currentPath,
		runnableVersion,
		small,
		args,
		newItem,
		schema,
		onEmailDomain,
		...props
	}: Props = $props()

	$effect(() => {
		console.log('selectedTrigger', selectedTrigger)
	})
</script>

{#if selectedTrigger.type === 'http'}
	<RoutesPanel
		{selectedTrigger}
		{isFlow}
		path={initialPath || fakeInitialPath}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{customLabel}
		{...props}
	/>
{:else if selectedTrigger.type === 'webhook'}
	<WebhooksPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{runnableVersion}
		token=""
		{args}
		scopes={isFlow ? [`jobs:run:flows:${currentPath}`] : [`jobs:run:scripts:${currentPath}`]}
		{newItem}
	/>
{:else if selectedTrigger.type === 'default_email'}
	<DefaultEmailPanel
		token=""
		scopes={isFlow ? [`jobs:run:flows:${currentPath}`] : [`jobs:run:scripts:${currentPath}`]}
		path={initialPath || fakeInitialPath}
		{isFlow}
		runnableVersion={!isFlow ? runnableVersion : undefined}
		{onEmailDomain}
	/>
{:else if selectedTrigger.type === 'schedule'}
	<SchedulePanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{schema}
		{customLabel}
		{...props}
	/>
{:else if selectedTrigger.type === 'websocket'}
	<WebsocketTriggersPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{customLabel}
		{...props}
	/>
{:else if selectedTrigger.type === 'kafka'}
	<KafkaTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{customLabel}
		{...props}
	/>
{:else if selectedTrigger.type === 'postgres'}
	<PostgresTriggersPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{customLabel}
		{...props}
	/>
{:else if selectedTrigger.type === 'nats'}
	<NatsTriggersPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{customLabel}
		{...props}
	/>
{:else if selectedTrigger.type === 'mqtt'}
	<MqttTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{customLabel}
		{...props}
	/>
{:else if selectedTrigger.type === 'sqs'}
	<SqsTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{customLabel}
		{...props}
	/>
{:else if selectedTrigger.type === 'gcp'}
	<GcpTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{customLabel}
		{...props}
	/>
{:else if selectedTrigger.type === 'email'}
	<EmailTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{customLabel}
		{onEmailDomain}
		{...props}
	/>
{:else if selectedTrigger.type === 'poll'}
	<ScheduledPollPanel />
{:else if selectedTrigger.type === 'nextcloud'}
	<NativeTriggersPanel
		service="nextcloud"
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{customLabel}
		{...props}
	/>
{:else if selectedTrigger.type === 'google'}
	<NativeTriggersPanel
		service="google"
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		{customLabel}
		{...props}
	/>
{:else if selectedTrigger.type === 'cli'}
	<div class="py-1 flex flex-col gap-6">
		<ClipboardPanel content={selectedTrigger.extra?.cliCommand ?? ''} />
		<CliHelpBox />
	</div>
{/if}

{#snippet customLabel()}
	{@const IconComponent = triggerIconMap[selectedTrigger.type]}
	<div class="flex flex-row gap-2 items-center grow min-w-0 pr-2">
		<IconComponent size={16} class={'shrink-0'} />
		<TriggerLabel trigger={selectedTrigger} />
	</div>
{/snippet}
