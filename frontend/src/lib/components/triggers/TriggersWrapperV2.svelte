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
	import { userStore } from '$lib/stores'
	import { type Trigger } from './utils'
	import { createEventDispatcher } from 'svelte'
	import PrimarySchedulePanel from './PrimarySchedulePanel.svelte'
	import { canWrite } from '$lib/utils'

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
		schema: any
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
		schema
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
		newDraft={selectedTrigger.saveCb === undefined}
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
{:else if selectedTrigger.type === 'schedule' && selectedTrigger.isPrimary}
	<PrimarySchedulePanel
		{schema}
		{isFlow}
		path={initialPath}
		{newItem}
		can_write={canWrite(currentPath, {}, $userStore)}
		on:update
		on:delete
		on:delete-pending
		on:save-pending
		isNewSchedule={selectedTrigger.isDraft}
		{isDeployed}
	/>
{:else if selectedTrigger.type === 'schedule'}
	<SchedulePanel
		{selectedTrigger}
		{isFlow}
		path={initialPath}
		{isDeployed}
		on:update-config={({ detail }) => updateConfig(detail)}
		on:update={async ({ detail }) => {
			if (selectedTrigger && selectedTrigger.isDraft && detail?.path) {
				dispatch('update', detail.path)
			}
		}}
		defaultValues={selectedTrigger.isDraft ? selectedTrigger.draftConfig : undefined}
	/>
{:else if selectedTrigger.type === 'websocket'}
	<WebsocketTriggersPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		isEditor={true}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config={({ detail }) => updateConfig(detail)}
		on:save-draft
		on:reset
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.saveCb === undefined}
	/>
{:else if selectedTrigger.type === 'kafka'}
	<KafkaTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		isEditor={true}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config
		on:save-draft
		on:reset
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.saveCb === undefined}
	/>
{:else if selectedTrigger.type === 'postgres'}
	<PostgresTriggersPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		isEditor={true}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config={({ detail }) => updateConfig(detail)}
		on:save-draft
		on:reset
		defaultValues={selectedTrigger.draftConfig ?? selectedTrigger.captureConfig ?? undefined}
		newDraft={selectedTrigger.saveCb === undefined}
	/>
{:else if selectedTrigger.type === 'nats'}
	<NatsTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		isEditor={true}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config={({ detail }) => updateConfig(detail)}
		defaultValues={selectedTrigger.isDraft ? selectedTrigger.draftConfig : undefined}
	/>
{:else if selectedTrigger.type === 'mqtt'}
	<MqttTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		isEditor={true}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config={({ detail }) => updateConfig(detail)}
		defaultValues={selectedTrigger.isDraft ? selectedTrigger.draftConfig : undefined}
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
		defaultValues={selectedTrigger.isDraft ? selectedTrigger.draftConfig : undefined}
	/>
{:else if selectedTrigger.type === 'gcp'}
	<GcpTriggerPanel
		{isFlow}
		path={initialPath || fakeInitialPath}
		{selectedTrigger}
		{edit}
		{isDeployed}
		isEditor={true}
		on:toggle-edit-mode
		on:update
		on:delete
		on:update-config={({ detail }) => updateConfig(detail)}
		defaultValues={selectedTrigger.isDraft ? selectedTrigger.draftConfig : undefined}
	/>
{:else if selectedTrigger.type === 'poll'}
	<ScheduledPollPanel on:update-config={({ detail }) => updateConfig(detail)} />
{/if}
