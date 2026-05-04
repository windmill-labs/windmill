<script lang="ts">
	import { onDestroy, tick } from 'svelte'
	import type { Component } from 'svelte'
	import { Drawer } from '$lib/components/common'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { registerToolDisplayActionHandler } from './createdResourceActions.svelte'
	import type { CreatedResourceTriggerKind, ToolDisplayAction } from './shared'

	type DrawerKey = 'schedule' | CreatedResourceTriggerKind
	type EditorHandle = {
		openEdit: (path: string, isFlow: boolean) => Promise<void>
	}
	type EditorComponent = Component<
		{
			useDrawer?: boolean
			onUpdate?: (path?: string) => void
		},
		EditorHandle
	>
	type EditorModule = { default: unknown }
	type DrawerConfig = {
		label: string
		load: () => Promise<EditorModule>
	}

	const DRAWER_SIZE = '800px'

	let drawer: Drawer | undefined = $state(undefined)
	let editor: EditorHandle | undefined = $state(undefined)
	let activeDrawer: DrawerKey | undefined = $state(undefined)
	let activePath = $state('')
	let editorPromise: Promise<EditorModule> | undefined = $state(undefined)

	const drawerConfigs: Record<DrawerKey, DrawerConfig> = {
		schedule: {
			label: 'schedule',
			load: () => import('$lib/components/triggers/schedules/ScheduleEditorInner.svelte')
		},
		http: {
			label: 'HTTP trigger',
			load: () => import('$lib/components/triggers/http/RouteEditorInner.svelte')
		},
		websocket: {
			label: 'WebSocket trigger',
			load: () => import('$lib/components/triggers/websocket/WebsocketTriggerEditorInner.svelte')
		},
		postgres: {
			label: 'Postgres trigger',
			load: () => import('$lib/components/triggers/postgres/PostgresTriggerEditorInner.svelte')
		},
		kafka: {
			label: 'Kafka trigger',
			load: () => import('$lib/components/triggers/kafka/KafkaTriggerEditorInner.svelte')
		},
		nats: {
			label: 'NATS trigger',
			load: () => import('$lib/components/triggers/nats/NatsTriggerEditorInner.svelte')
		},
		mqtt: {
			label: 'MQTT trigger',
			load: () => import('$lib/components/triggers/mqtt/MqttTriggerEditorInner.svelte')
		},
		sqs: {
			label: 'SQS trigger',
			load: () => import('$lib/components/triggers/sqs/SqsTriggerEditorInner.svelte')
		},
		gcp: {
			label: 'GCP Pub/Sub trigger',
			load: () => import('$lib/components/triggers/gcp/GcpTriggerEditorInner.svelte')
		},
		azure: {
			label: 'Azure Event Grid trigger',
			load: () => import('$lib/components/triggers/azure/AzureTriggerEditorInner.svelte')
		}
	}

	const activeConfig = $derived.by(() => {
		const current = activeDrawer
		return current ? drawerConfigs[current] : undefined
	})
	const drawerTitle = $derived(
		activeConfig ? `Edit ${activeConfig.label} ${activePath}` : 'Edit trigger'
	)

	function waitForAnimationFrame(): Promise<void> {
		return new Promise((resolve) => {
			if (typeof requestAnimationFrame === 'function') {
				requestAnimationFrame(() => resolve())
			} else {
				resolve()
			}
		})
	}

	async function getEditor(
		expectedDrawer: DrawerKey,
		expectedPromise: Promise<EditorModule>,
		expectedPath: string
	): Promise<EditorHandle | undefined> {
		await expectedPromise
		await tick()
		await waitForAnimationFrame()

		if (
			activeDrawer !== expectedDrawer ||
			editorPromise !== expectedPromise ||
			activePath !== expectedPath
		) {
			return undefined
		}

		if (!editor) {
			await tick()
			await waitForAnimationFrame()
		}

		if (!editor) {
			throw new Error(`${activeConfig?.label ?? 'Trigger'} drawer is not ready`)
		}
		return editor
	}

	async function openCreatedResource(action: ToolDisplayAction) {
		if (action.type !== 'open_created_resource') {
			return
		}

		const key = action.resource === 'schedule' ? 'schedule' : action.triggerKind
		if (!key) {
			throw new Error('Missing trigger kind')
		}

		const config = drawerConfigs[key]
		const nextEditorPromise = activeDrawer === key && editorPromise ? editorPromise : config.load()
		if (activeDrawer !== key) {
			editor = undefined
		}
		activeDrawer = key
		activePath = action.path
		editorPromise = nextEditorPromise
		drawer?.openDrawer()

		const currentEditor = await getEditor(key, nextEditorPromise, action.path)
		if (
			!currentEditor ||
			activeDrawer !== key ||
			editorPromise !== nextEditorPromise ||
			activePath !== action.path
		) {
			return
		}
		await currentEditor.openEdit(action.path, action.targetKind === 'flow')
	}

	function onInnerUpdate() {
		drawer?.closeDrawer()
	}

	const unregisterCreatedResourceHandler = registerToolDisplayActionHandler(
		'open_created_resource',
		openCreatedResource
	)

	onDestroy(() => {
		unregisterCreatedResourceHandler()
	})
</script>

<Drawer bind:this={drawer} size={DRAWER_SIZE}>
	<DrawerContent title={drawerTitle} on:close={() => drawer?.closeDrawer()}>
		{#if activeDrawer && editorPromise}
			{#key activeDrawer}
				{#await editorPromise}
					<div class="p-4 flex justify-center">
						<Loader2 class="animate-spin" />
					</div>
				{:then Module}
					{@const Editor = Module.default as EditorComponent}
					<Editor bind:this={editor} useDrawer={false} onUpdate={onInnerUpdate} />
				{/await}
			{/key}
		{/if}
	</DrawerContent>
</Drawer>
