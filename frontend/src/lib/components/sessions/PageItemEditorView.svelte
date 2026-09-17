<script lang="ts">
	import { setContext, untrack } from 'svelte'
	import { Loader2 } from 'lucide-svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { resource } from 'runed'
	import { Button } from '$lib/components/common'
	import { lookupPageItem } from './pageItemLookup'
	import VariableEditor from '$lib/components/VariableEditor.svelte'
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import { setOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import {
		pageItemKindLabel,
		pageItemUrl,
		TRIGGER_PAGES,
		type PageItemRef,
		type TriggerKind
	} from './previewPaths'
	import type { SessionRuntime } from './sessionRuntime.svelte'

	let {
		runtime,
		item,
		workspaceId,
		reloadNonce = 0
	}: {
		runtime: SessionRuntime
		item: PageItemRef
		workspaceId: string
		/** Bumped to reload the item from the server, discarding the mounted editor. */
		reloadNonce?: number
	} = $props()

	// Mounted in the preview panel, outside the chat's own subtree: chat-aware components
	// below would otherwise resolve the app-wide manager rather than this session's.
	// Captured at init, as SessionEditorTarget does: descendants rely on its presence.
	setContext(
		'aiChatManager',
		untrack(() => runtime.manager)
	)
	setOperatingWorkspace(() => workspaceId)

	type TriggerEditorHandle = { openEdit: (path: string, isFlow: boolean) => Promise<void> }
	type EditorModule = { default: any }

	const TRIGGER_EDITORS: Record<TriggerKind | 'schedule', () => Promise<EditorModule>> = {
		schedule: () => import('$lib/components/triggers/schedules/ScheduleEditorInner.svelte'),
		http: () => import('$lib/components/triggers/http/RouteEditorInner.svelte'),
		websocket: () =>
			import('$lib/components/triggers/websocket/WebsocketTriggerEditorInner.svelte'),
		postgres: () => import('$lib/components/triggers/postgres/PostgresTriggerEditorInner.svelte'),
		kafka: () => import('$lib/components/triggers/kafka/KafkaTriggerEditorInner.svelte'),
		nats: () => import('$lib/components/triggers/nats/NatsTriggerEditorInner.svelte'),
		mqtt: () => import('$lib/components/triggers/mqtt/MqttTriggerEditorInner.svelte'),
		amqp: () => import('$lib/components/triggers/amqp/AmqpTriggerEditorInner.svelte'),
		sqs: () => import('$lib/components/triggers/sqs/SqsTriggerEditorInner.svelte'),
		gcp: () => import('$lib/components/triggers/gcp/GcpTriggerEditorInner.svelte'),
		azure: () => import('$lib/components/triggers/azure/AzureTriggerEditorInner.svelte'),
		email: () => import('$lib/components/triggers/email/EmailTriggerEditorInner.svelte')
	}

	const triggerKey = $derived(
		item.kind === 'schedule' ? 'schedule' : item.kind === 'trigger' ? item.triggerKind : undefined
	)
	const eeLocked = $derived(
		item.kind === 'trigger' && !!TRIGGER_PAGES[item.triggerKind].ee && !$enterpriseLicense
	)

	let variableEditor: VariableEditor | undefined = $state()
	let resourceEditor: ResourceEditorDrawer | undefined = $state()
	let triggerEditor: TriggerEditorHandle | undefined = $state()

	// Each editor loads through the same call its list page makes, once its instance is bound.
	// Keyed below on everything that names what it shows, so each binding is a fresh instance.
	$effect(() => {
		const path = item.path
		const v = variableEditor
		const r = resourceEditor
		const t = triggerEditor
		untrack(() => {
			v?.editVariable(path)
			// An agent opens on JSON: the generic form would render its configuration field by
			// field and write a default into each one the value leaves out, drafting just by opening.
			void r?.initEdit(path, { json: lookup.current?.resourceType === 'ai_agent' })
			void t?.openEdit(path, false)
		})
	})

	// A save can move the item; the tab follows it, which remounts the editor on what was
	// written. Saving in place remounts it too: the editors keep the pre-save baseline, and
	// their drawers only ever relied on being closed after a save.
	let savedNonce = $state(0)

	// Whether the item still exists, deployed or as a draft, read again whenever the editor
	// remounts: a delete or a discarded draft reloads the tab too, and the editors have no state
	// of their own for an item that is gone. A lookup that fails otherwise is left to the editor,
	// which reports its own load errors.
	const lookup = resource(
		() => ({ ref: item, workspace: workspaceId, reload: `${reloadNonce}:${savedNonce}` }),
		({ ref, workspace }) =>
			lookupPageItem(ref, workspace).then(
				(found) => ({ exists: !!found, resourceType: found?.resource_type as string | undefined }),
				() => ({ exists: true, resourceType: undefined })
			)
	)

	function closeTab() {
		const tab = runtime.previewTabs.tabs.find((t) => t.url === pageItemUrl(item))
		if (tab) runtime.previewTabs.close(tab.id)
	}
	function onSaved(path: string | undefined) {
		if (path && path !== item.path) {
			runtime.previewTabs.retargetPageItem(item, { ...item, path })
		} else {
			savedNonce++
		}
	}
</script>

{#snippet loading()}
	<div class="flex-1 flex items-center justify-center text-tertiary">
		<Loader2 class="animate-spin" />
	</div>
{/snippet}

<div class="flex h-full min-h-0 flex-col">
	{#if eeLocked}
		<div class="p-4 text-sm text-secondary">This trigger requires an enterprise license.</div>
	{:else if lookup.loading || !lookup.current}
		{@render loading()}
	{:else if !lookup.current.exists}
		<div class="flex flex-col items-start gap-3 p-4 text-sm text-secondary">
			<p>
				{pageItemKindLabel(item)} <span class="font-mono">{item.path}</span> no longer exists in this
				workspace.
			</p>
			<Button variant="default" unifiedSize="sm" onClick={closeTab}>Close tab</Button>
		</div>
	{:else}
		{#key `${item.kind}:${triggerKey}:${item.path}:${workspaceId}:${reloadNonce}:${savedNonce}`}
			{#if item.kind === 'variable'}
				<VariableEditor
					bind:this={variableEditor}
					inline
					workspace={workspaceId}
					{onSaved}
					onClose={closeTab}
				/>
			{:else if item.kind === 'resource'}
				<ResourceEditorDrawer
					bind:this={resourceEditor}
					inline
					workspace={workspaceId}
					onClose={closeTab}
					onSaved={(path) => {
						if (path !== undefined) onSaved(path)
					}}
					onRestored={() => savedNonce++}
				/>
			{:else if triggerKey}
				{#await TRIGGER_EDITORS[triggerKey]()}
					{@render loading()}
				{:then Module}
					<Module.default
						bind:this={triggerEditor}
						useDrawer
						inline
						onClose={closeTab}
						onUpdate={(path?: string) => onSaved(path)}
					/>
				{/await}
			{/if}
		{/key}
	{/if}
</div>
