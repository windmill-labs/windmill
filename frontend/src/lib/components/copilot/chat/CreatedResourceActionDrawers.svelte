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
	type ActiveDrawerState = {
		id: number
		key: DrawerKey
		path: string
		promise: Promise<EditorModule>
	}

	const DRAWER_SIZE = '800px'

	let drawer: Drawer | undefined = $state(undefined)
	let editor: EditorHandle | undefined = $state(undefined)
	let activeDrawer: ActiveDrawerState | undefined = $state(undefined)
	let nextActiveDrawerId = 0

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

	const drawerTitle = $derived.by(() => {
		const current = activeDrawer
		return current ? `Edit ${drawerConfigs[current.key].label} ${current.path}` : 'Edit trigger'
	})

	function waitForAnimationFrame(): Promise<void> {
		return new Promise((resolve) => {
			if (typeof requestAnimationFrame === 'function') {
				requestAnimationFrame(() => resolve())
			} else {
				resolve()
			}
		})
	}

	async function getEditor(request: ActiveDrawerState): Promise<EditorHandle | undefined> {
		await request.promise
		await tick()
		await waitForAnimationFrame()

		if (activeDrawer?.id !== request.id) {
			return undefined
		}

		if (!editor) {
			await tick()
			await waitForAnimationFrame()
		}

		if (!editor) {
			throw new Error(`${drawerConfigs[request.key].label} drawer is not ready`)
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
		const promise = activeDrawer?.key === key ? activeDrawer.promise : config.load()
		if (activeDrawer?.key !== key) {
			editor = undefined
		}

		const request: ActiveDrawerState = { id: nextActiveDrawerId++, key, path: action.path, promise }
		activeDrawer = request
		drawer?.openDrawer()

		const currentEditor = await getEditor(request)
		if (!currentEditor) {
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
		{#if activeDrawer}
			{#key activeDrawer.key}
				{#await activeDrawer.promise}
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
