<script lang="ts">
	import RoutesPanel from './http/RoutesPanelV2.svelte'
	import WebhooksPanel from './webhook/WebhooksPanelV2.svelte'
	import EmailTriggerPanel from '../details/EmailTriggerPanelV2.svelte'
	import SchedulePanel from '$lib/components/SchedulePanel.svelte'
	import PostgresTriggersPanel from './postgres/PostgresTriggersPanelV2.svelte'
	import KafkaTriggerPanel from './kafka/KafkaTriggerPanelV2.svelte'
	import NatsTriggerPanel from './nats/NatsTriggerPanelV2.svelte'
	import MqttTriggerPanel from './mqtt/MqttTriggerPanelV2.svelte'
	import SqsTriggerPanel from './sqs/SqsTriggerPanelV2.svelte'
	import GcpTriggerPanel from './gcp/GcpTriggerPanelV2.svelte'
	import ScheduledPollPanel from './scheduled/ScheduledPollPanel.svelte'
	import WebsocketTriggersPanel from './websocket/WebsocketTriggersPanelV2.svelte'
	import { type Trigger } from './utils'
	import { createEventDispatcher } from 'svelte'
	import ClipboardPanel from '../details/ClipboardPanel.svelte'
	import CliHelpBox from '../CliHelpBox.svelte'

	const dispatch = createEventDispatcher()

	interface Props {
		selectedTrigger: Trigger
		isFlow: boolean
		initialPath: string
		fakeInitialPath: string
		currentPath: string
		edit: boolean
		hash?: string
		isDeployed: boolean
		small: boolean
		args: Record<string, any>
		newItem: boolean
		schema: Record<string, any> | undefined
		isEditor?: boolean
	}

	let {
		selectedTrigger,
		isFlow = false,
		initialPath,
		fakeInitialPath,
		currentPath,
		edit,
		hash,
		isDeployed,
		small,
		args,
		newItem,
		schema,
		isEditor = false
	}: Props = $props()

	// Forward config updates to parent
	function updateConfig(config: Record<string, any>) {
		dispatch('update-config', config)
	}
</script>

{#if selectedTrigger.type === 'http'}
	<RoutesPanel
		{selectedTrigger}
		{isFlow}
		path={initialPath || fakeInitialPath}
		{edit}
		on:update-config={({ detail }) => updateConfig(detail)}
		on:update
		on:delete
		on:toggle-edit-mode
		on:save-draft
		on:reset
		{isDeployed}
		{small}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
	/>
{:else if selectedTrigger.type === 'webhook'}
	<WebhooksPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{hash}
		token=""
		{args}
		on:update-config={({ detail }) => updateConfig(detail)}
		scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
		{newItem}
	/>
{:else if selectedTrigger.type === 'email'}
	<EmailTriggerPanel
		token=""
		scopes={isFlow ? [`run:flow/${currentPath}`] : [`run:script/${currentPath}`]}
		path={initialPath || fakeInitialPath}
		{isFlow}
		on:update-config={({ detail }) => updateConfig(detail)}
		on:emailDomain={({ detail }) => {
			updateConfig({ emailDomain: detail })
		}}
	/>
{:else if selectedTrigger.type === 'schedule'}
	<SchedulePanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{isDeployed}
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
		{edit}
		{schema}
		{isEditor}
		on:update-config={({ detail }) => updateConfig(detail)}
		on:update
		on:save-draft
		on:reset
		on:toggle-edit-mode
		on:delete
	/>
{:else if selectedTrigger.type === 'websocket'}
	<WebsocketTriggersPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		{isEditor}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config={({ detail }) => updateConfig(detail)}
		on:save-draft
		on:reset
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
	/>
{:else if selectedTrigger.type === 'kafka'}
	<KafkaTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		{isEditor}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config
		on:save-draft
		on:reset
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
	/>
{:else if selectedTrigger.type === 'postgres'}
	<PostgresTriggersPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		{isEditor}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config={({ detail }) => updateConfig(detail)}
		on:save-draft
		on:reset
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
	/>
{:else if selectedTrigger.type === 'nats'}
	<NatsTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		{isEditor}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config={({ detail }) => updateConfig(detail)}
		on:save-draft
		on:reset
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
	/>
{:else if selectedTrigger.type === 'mqtt'}
	<MqttTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		{isEditor}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config={({ detail }) => updateConfig(detail)}
		on:save-draft
		on:reset
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
	/>
{:else if selectedTrigger.type === 'sqs'}
	<SqsTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config={({ detail }) => updateConfig(detail)}
		on:save-draft
		on:reset
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
	/>
{:else if selectedTrigger.type === 'gcp'}
	<GcpTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		{isEditor}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config={({ detail }) => updateConfig(detail)}
		on:save-draft
		on:reset
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.draftConfig === undefined}
	/>
{:else if selectedTrigger.type === 'poll'}
	<ScheduledPollPanel on:update-config={({ detail }) => updateConfig(detail)} />
{:else if selectedTrigger.type === 'cli'}
	<div class="p-2 flex flex-col gap-4">
		<ClipboardPanel content={selectedTrigger.extra?.cliCommand ?? ''} />
		<CliHelpBox />
	</div>
{/if}
