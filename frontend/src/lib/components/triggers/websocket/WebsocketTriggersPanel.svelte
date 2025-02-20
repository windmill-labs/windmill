<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import { WebsocketTriggerService, type WebsocketTrigger } from '$lib/gen'

	import { canWrite } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import WebsocketTriggerEditor from './WebsocketTriggerEditor.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import Section from '$lib/components/Section.svelte'
	import { Alert } from '$lib/components/common'
	import Description from '$lib/components/Description.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import TriggersEditorSection from '../TriggersEditorSection.svelte'
	export let isFlow: boolean
	export let path: string
	export let newItem: boolean = false
	export let isEditor: boolean = false
	export let canHavePreprocessor: boolean = false
	export let hasPreprocessor: boolean = false

	let wsTriggerEditor: WebsocketTriggerEditor
	let openForm = true
	let dontCloseOnLoad = false

	$: path && loadTriggers()

	const { triggersCount, selectedTrigger, defaultValues } =
		getContext<TriggerContext>('TriggerContext')

	onMount(() => {
		if (
			defaultValues &&
			$selectedTrigger === 'websockets' &&
			Object.keys($defaultValues ?? {}).length > 0
		) {
			wsTriggerEditor.openNew(isFlow, path, $defaultValues)
			defaultValues.set(undefined)
		}
	})

	let wsTriggers: (WebsocketTrigger & { canWrite: boolean })[] | undefined = undefined
	export async function loadTriggers() {
		try {
			wsTriggers = (
				await WebsocketTriggerService.listWebsocketTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
			$triggersCount = { ...($triggersCount ?? {}), websocket_count: wsTriggers?.length }
			openForm = wsTriggers?.length === 0 || dontCloseOnLoad
		} catch (e) {
			console.error('impossible to load WS triggers', e)
		}
	}
</script>

<WebsocketTriggerEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={wsTriggerEditor}
/>

{#if isCloudHosted()}
	<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
		WebSocket triggers are disabled in the multi-tenant cloud.
	</Alert>
{:else}
	<div class="flex flex-col gap-4">
		<Description link="https://www.windmill.dev/docs/core_concepts/websocket_triggers">
			WebSocket triggers allow real-time bidirectional communication between your scripts/flows and
			external systems. Each trigger creates a unique WebSocket endpoint.
		</Description>

		{#if !newItem && wsTriggers && wsTriggers.length > 0}
			<Section label="WebSockets">
				<div class="flex flex-col gap-4">
					<div class="flex flex-col divide-y pt-2">
						{#each wsTriggers as wsTriggers (wsTriggers.path)}
							<div class="grid grid-cols-5 text-2xs items-center py-2">
								<div class="col-span-2 truncate">{wsTriggers.path}</div>
								<div class="col-span-2 truncate">
									{wsTriggers.url}
								</div>
								<div class="flex justify-end">
									<button
										on:click={() => wsTriggerEditor?.openEdit(wsTriggers.path, isFlow)}
										class="px-2"
									>
										{#if wsTriggers.canWrite}
											Edit
										{:else}
											View
										{/if}
									</button>
								</div>
							</div>
						{/each}
					</div>
				</div>
			</Section>
		{/if}

		<TriggersEditorSection
			on:applyArgs
			on:saveTrigger={(e) => {
				wsTriggerEditor?.openNew(isFlow, path, e.detail.config)
			}}
			on:addPreprocessor
			on:updateSchema
			on:testWithArgs
			cloudDisabled={false}
			triggerType="websocket"
			{isFlow}
			{path}
			{isEditor}
			{canHavePreprocessor}
			{hasPreprocessor}
			{newItem}
			{openForm}
		/>
	</div>
{/if}
