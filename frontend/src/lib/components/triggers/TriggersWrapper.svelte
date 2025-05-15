<script lang="ts">
	import RoutesPanel from './http/RoutesPanel.svelte'
	import WebhooksPanel from './webhook/WebhooksPanel.svelte'
	import EmailTriggerPanel from '../details/EmailTriggerPanel.svelte'
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
	import { twMerge } from 'tailwind-merge'

	interface Props {
		selectedTrigger: Trigger
		isFlow: boolean
		initialPath: string
		fakeInitialPath: string
		currentPath: string
		hash?: string
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
	}

	let {
		selectedTrigger,
		isFlow = false,
		initialPath,
		fakeInitialPath,
		currentPath,
		hash,
		small,
		args,
		newItem,
		schema,
		...props
	}: Props = $props()
</script>

{#if selectedTrigger.type === 'http'}
	<RoutesPanel
		{selectedTrigger}
		{isFlow}
		path={initialPath || fakeInitialPath}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
		customLabel={small ? customLabel : undefined}
		{...props}
	/>
{:else if selectedTrigger.type === 'webhook'}
	<WebhooksPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{hash}
		token=""
		{args}
		scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
		{newItem}
	/>
{:else if selectedTrigger.type === 'email'}
	<EmailTriggerPanel
		token=""
		scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
		path={initialPath || fakeInitialPath}
		{isFlow}
		on:email-domain
	/>
{:else if selectedTrigger.type === 'schedule'}
	<SchedulePanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
		{schema}
		customLabel={small ? customLabel : undefined}
		{...props}
	/>
{:else if selectedTrigger.type === 'websocket'}
	<WebsocketTriggersPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
		customLabel={small ? customLabel : undefined}
		{...props}
	/>
{:else if selectedTrigger.type === 'kafka'}
	<KafkaTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
		customLabel={small ? customLabel : undefined}
		{...props}
	/>
{:else if selectedTrigger.type === 'postgres'}
	<PostgresTriggersPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
		customLabel={small ? customLabel : undefined}
		{...props}
	/>
{:else if selectedTrigger.type === 'nats'}
	<NatsTriggersPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		customLabel={small ? customLabel : undefined}
		{...props}
	/>
{:else if selectedTrigger.type === 'mqtt'}
	<MqttTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
		customLabel={small ? customLabel : undefined}
		{...props}
	/>
{:else if selectedTrigger.type === 'sqs'}
	<SqsTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
		customLabel={small ? customLabel : undefined}
		{...props}
	/>
{:else if selectedTrigger.type === 'gcp'}
	<GcpTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
		customLabel={small ? customLabel : undefined}
		{...props}
	/>
{:else if selectedTrigger.type === 'poll'}
	<ScheduledPollPanel />
{:else if selectedTrigger.type === 'cli'}
	<div class="py-1 flex flex-col gap-4">
		<ClipboardPanel content={selectedTrigger.extra?.cliCommand ?? ''} />
		<CliHelpBox />
	</div>
{/if}

{#snippet customLabel()}
	{@const IconComponent = triggerIconMap[selectedTrigger.type]}
	<div class="flex flex-row gap-2 items-center grow min-w-0 pr-2">
		<IconComponent
			size={16}
			class={twMerge(selectedTrigger.isDraft ? 'text-frost-400' : '', 'shrink-0')}
		/>
		<TriggerLabel trigger={selectedTrigger} />
	</div>
{/snippet}
