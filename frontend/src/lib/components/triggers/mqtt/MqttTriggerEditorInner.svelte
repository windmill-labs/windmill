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
	import { MqttTriggerService } from '$lib/gen'

	let drawer: Drawer
	let is_flow: boolean = false
	let initialPath = ''
	let edit = true
	let itemKind: 'flow' | 'script' = 'script'
	let script_path = ''
	let initialScriptPath = ''
	let fixedScriptPath = ''
	let path: string = ''
	let pathError = ''
	let enabled = false
	let dirtyPath = false
	let can_write = true
	let drawerLoading = true
	const dispatch = createEventDispatcher()

	$: is_flow = itemKind === 'flow'

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
			drawer?.openDrawer()
			is_flow = nis_flow
			itemKind = nis_flow ? 'flow' : 'script'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = ''
			initialPath = ''
			edit = false
			dirtyPath = false
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
			script_path = s.script_path
			initialScriptPath = s.script_path
			is_flow = s.is_flow
			path = s.path
			enabled = s.enabled
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
					path,
					script_path,
					enabled,
					is_flow
				}
			})
			sendUserToast(`Mqtt trigger ${path} updated`)
		} else {
			await MqttTriggerService.createMqttTrigger({
				workspace: $workspaceStore!,
				requestBody: {
					enabled: true,
					path,
					script_path,
					is_flow
				}
			})
			sendUserToast(`Mqtt trigger ${path} created`)
		}

		if (!$usedTriggerKinds.includes('mqtt')) {
			$usedTriggerKinds = [...$usedTriggerKinds, 'mqtt']
		}
		dispatch('update')
		drawer.closeDrawer()
	}
</script>

<Drawer size="800px" bind:this={drawer}>
	<DrawerContent
		title={edit
			? can_write
				? `Edit Mqtt trigger ${initialPath}`
				: `Mqtt trigger ${initialPath}`
			: 'New Mqtt trigger'}
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
					disabled={pathError != '' || emptyString(script_path) || !can_write}
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
						New postgres triggers can take up to 30 seconds to start listening.
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
						Pick a script or flow to be triggered <Required required={true} />
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
					</div>
				</Section>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
