<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Save } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import {
		MqttTriggerService,
		type MqttClientVersion,
		type MqttV3Config,
		type MqttV5Config,
		type MqttSubscribeTopic
	} from '$lib/gen'
	import MqttEditorConfigSection from './MqttEditorConfigSection.svelte'

	let mqtt_resource_path: string = $state('')
	let drawer: Drawer | undefined = $state(undefined)
	let is_flow: boolean = $state(false)
	let initialPath = $state('')
	let edit = $state(true)
	let itemKind: 'flow' | 'script' = $state('script')
	let script_path = $state('')
	let initialScriptPath = $state('')
	let fixedScriptPath = $state('')
	let path: string = $state('')
	let pathError = $state('')
	let enabled = $state(false)
	let dirtyPath = $state(false)
	let can_write = $state(true)
	let drawerLoading = $state(true)
	let subscribe_topics: MqttSubscribeTopic[] = $state([])
	let v3_config: MqttV3Config | undefined = $state()
	let v5_config: MqttV5Config | undefined = $state()
	let client_version: MqttClientVersion | undefined = $state()
	let client_id: string | undefined = $state(undefined)
	let isValid: boolean | undefined = $state(undefined)
	const dispatch = createEventDispatcher()

	$effect(() => {
		is_flow = itemKind === 'flow'
	})

	export async function openEdit(ePath: string, isFlow: boolean) {
		drawerLoading = true
		try {
			drawer?.openDrawer()
			initialPath = ePath
			itemKind = isFlow ? 'flow' : 'script'
			edit = true
			dirtyPath = false
			await loadTrigger()
		} catch (err) {
			sendUserToast(`Could not load mqtt trigger: ${err.body}`, true)
		} finally {
			drawerLoading = false
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		defaultValues?: Record<string, any>
	) {
		drawerLoading = true
		try {
			mqtt_resource_path = ''
			drawer?.openDrawer()
			is_flow = nis_flow
			itemKind = nis_flow ? 'flow' : 'script'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			subscribe_topics = defaultValues?.topics ?? []
			path = ''
			initialPath = ''
			edit = false
			dirtyPath = false
			client_version = defaultValues?.client_version ?? 'v5'
			client_id = defaultValues?.client_id ?? ''
		} finally {
			drawerLoading = false
		}
	}

	async function loadTrigger(): Promise<void> {
		try {
			const s = await MqttTriggerService.getMqttTrigger({
				workspace: $workspaceStore!,
				path: initialPath
			})
			mqtt_resource_path = s.mqtt_resource_path
			subscribe_topics = s.subscribe_topics
			script_path = s.script_path
			initialScriptPath = s.script_path
			is_flow = s.is_flow
			path = s.path
			enabled = s.enabled
			client_version = s.client_version
			v3_config = s.v3_config
			v5_config = s.v5_config
			client_id = s.client_id ?? ''
			can_write = canWrite(s.path, s.extra_perms, $userStore)
		} catch (error) {
			sendUserToast(`Could not load mqtt trigger: ${error.body}`, true)
		}
	}

	async function updateTrigger(): Promise<void> {
		if (edit) {
			await MqttTriggerService.updateMqttTrigger({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					client_id,
					client_version,
					v3_config,
					v5_config,
					mqtt_resource_path,
					subscribe_topics,
					path,
					script_path,
					enabled,
					is_flow
				}
			})
			sendUserToast(`MQTT trigger ${path} updated`)
		} else {
			await MqttTriggerService.createMqttTrigger({
				workspace: $workspaceStore!,
				requestBody: {
					client_id,
					client_version,
					v3_config,
					v5_config,
					mqtt_resource_path,
					subscribe_topics,
					enabled: true,
					path,
					script_path,
					is_flow
				}
			})
			sendUserToast(`MQTT trigger ${path} created`)
		}

		if (!$usedTriggerKinds.includes('mqtt')) {
			$usedTriggerKinds = [...$usedTriggerKinds, 'mqtt']
		}
		dispatch('update')
		drawer?.closeDrawer()
	}
</script>

<Drawer size="800px" bind:this={drawer}>
	<DrawerContent
		title={edit
			? can_write
				? `Edit MQTT trigger ${initialPath}`
				: `MQTT trigger ${initialPath}`
			: 'New MQTT trigger'}
		on:close={drawer.closeDrawer}
	>
		<svelte:fragment slot="actions">
			{#if !drawerLoading && can_write}
				{#if edit}
					<div class="mr-8 center-center -mt-1">
						<Toggle
							disabled={!can_write}
							checked={enabled}
							options={{ right: 'enable', left: 'disable' }}
							on:change={async (e) => {
								sendUserToast(`${e.detail ? 'enabled' : 'disabled'} mqtt trigger ${initialPath}`)
							}}
						/>
					</div>
				{/if}
				<Button
					startIcon={{ icon: Save }}
					disabled={pathError != '' || emptyString(script_path) || !can_write || !isValid}
					on:click={updateTrigger}
				>
					Save
				</Button>
			{/if}
		</svelte:fragment>
		{#if drawerLoading}
			<div class="flex flex-col items-center justify-center h-full w-full">
				<Loader2 size="50" class="animate-spin" />
				<p>Loading...</p>
			</div>
		{:else}
			<div class="flex flex-col gap-5">
				<Alert title="Info" type="info">
					{#if edit}
						Changes can take up to 30 seconds to take effect.
					{:else}
						New MQTT triggers can take up to 30 seconds to start listening.
					{/if}
				</Alert>
			</div>
			<div class="flex flex-col gap-12 mt-6">
				<div class="flex flex-col gap-4">
					<Label label="Path">
						<Path
							bind:dirty={dirtyPath}
							bind:error={pathError}
							bind:path
							{initialPath}
							checkInitialPathExistence={!edit}
							namePlaceholder="mqtt_trigger"
							kind="mqtt_trigger"
							disabled={!can_write}
						/>
					</Label>
				</div>

				<Section label="Runnable">
					<p class="text-xs mb-1 text-tertiary">
						Pick a script or flow to be triggered<Required required={true} />
					</p>
					<div class="flex flex-row mb-2">
						<ScriptPicker
							disabled={fixedScriptPath != '' || !can_write}
							initialPath={fixedScriptPath || initialScriptPath}
							kinds={['script']}
							allowFlow={true}
							bind:itemKind
							bind:scriptPath={script_path}
							allowRefresh
						/>
						{#if emptyString(script_path)}
							<Button
								btnClasses="ml-4 mt-2"
								color="dark"
								size="xs"
								href={itemKind === 'flow' ? '/flows/add?hub=61' : '/scripts/add?hub=hub%2F11619'}
								target="_blank">Create from template</Button
							>
						{/if}
					</div>
				</Section>

				<MqttEditorConfigSection
					bind:mqtt_resource_path
					bind:subscribe_topics
					bind:can_write
					bind:client_version
					bind:v3_config
					bind:v5_config
					bind:isValid
					bind:client_id
					headless={true}
				/>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
