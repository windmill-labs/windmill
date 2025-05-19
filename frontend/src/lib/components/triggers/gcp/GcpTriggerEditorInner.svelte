<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { Loader2, Save } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import {
		GcpTriggerService,
		type DeliveryType,
		type PushConfig,
		type SubscriptionMode
	} from '$lib/gen'
	import Section from '$lib/components/Section.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'
	import GcpTriggerEditorConfigSection from './GcpTriggerEditorConfigSection.svelte'
	import { base } from '$app/paths'

	let is_loading = false
	let drawer: Drawer
	let is_flow: boolean = false
	let initialPath = ''
	let edit = true
	let delivery_type: DeliveryType = 'pull'
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
	let topic_id: string = ''
	let gcp_resource_path: string = ''
	let subscription_id: string = ''
	let isValid = false
	let delivery_config: PushConfig | undefined = undefined
	let subscription_mode: SubscriptionMode = 'create_update'
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
			sendUserToast(`Could not load GCP Pub/Sub trigger: ${err.body}`, true)
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
			gcp_resource_path = defaultValues?.gcp_resource_path ?? ''
			delivery_type = defaultValues?.delivery_type ?? 'pull'
			delivery_config = defaultValues?.delivery_config ?? undefined
			subscription_id = ''
			topic_id = defaultValues?.topic_id
			subscription_mode = defaultValues?.subscription_mode ?? 'create_update'
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
			const s = await GcpTriggerService.getGcpTrigger({
				workspace: $workspaceStore!,
				path: initialPath
			})
			script_path = s.script_path
			initialScriptPath = s.script_path
			gcp_resource_path = s.gcp_resource_path
			delivery_type = s.delivery_type
			subscription_id = s.subscription_id
			delivery_config = s.delivery_config
			subscription_mode = s.subscription_mode
			is_flow = s.is_flow
			path = s.path
			enabled = s.enabled
			topic_id = s.topic_id
			can_write = canWrite(s.path, s.extra_perms, $userStore)
		} catch (error) {
			sendUserToast(`Could not load GCP Pub/Sub trigger: ${error.body}`, true)
		}
	}

	async function updateTrigger(): Promise<void> {
		try {
			is_loading = true
			const base_endpoint = `${window.location.origin}${base}`
			if (delivery_type === 'push') {
				if (!delivery_config) {
					sendUserToast('Must set route path when delivery type is push', true)
					return
				}
			} else {
				delivery_config = undefined
			}
			if (edit) {
				await GcpTriggerService.updateGcpTrigger({
					workspace: $workspaceStore!,
					path: initialPath,
					requestBody: {
						gcp_resource_path,
						subscription_mode,
						subscription_id,
						delivery_type,
						delivery_config,
						base_endpoint,
						topic_id,
						path,
						script_path,
						enabled,
						is_flow
					}
				})
				sendUserToast(`GCP Pub/Sub trigger ${path} updated`)
			} else {
				await GcpTriggerService.createGcpTrigger({
					workspace: $workspaceStore!,
					requestBody: {
						gcp_resource_path,
						subscription_mode,
						subscription_id,
						delivery_type,
						delivery_config,
						base_endpoint,
						topic_id,
						path,
						script_path,
						enabled: true,
						is_flow
					}
				})
				sendUserToast(`GCP Pub/Sub trigger ${path} created`)
			}

			if (!$usedTriggerKinds.includes('gcp')) {
				$usedTriggerKinds = [...$usedTriggerKinds, 'gcp']
			}
			dispatch('update')
			drawer.closeDrawer()
			is_loading = false
		} catch (error) {
			is_loading = false
			sendUserToast(error.body, true)
		}
	}
</script>

<Drawer size="800px" bind:this={drawer}>
	<DrawerContent
		title={edit
			? can_write
				? `Edit GCP Pub/Sub trigger ${initialPath}`
				: `GCP Pub/Sub trigger ${initialPath}`
			: 'New GCP Pub/Sub trigger'}
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
								sendUserToast(
									`${e.detail ? 'enabled' : 'disabled'} GCP Pub/Sub trigger ${initialPath}`
								)
							}}
						/>
					</div>
				{/if}
				<Button
					startIcon={{ icon: Save }}
					disabled={pathError != '' || emptyString(script_path) || !isValid || !can_write}
					on:click={updateTrigger}
					loading={is_loading}
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
						New GCP Pub/Sub trigger can take up to 30 seconds to start listening.
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
							namePlaceholder="gcp_trigger"
							kind="gcp_trigger"
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
						{#if emptyString(script_path)}
							<Button
								btnClasses="ml-4 mt-2"
								color="dark"
								size="xs"
								href={itemKind === 'flow' ? '/flows/add?hub=68' : '/scripts/add?hub=hub%2F19662'}
								target="_blank">Create from template</Button
							>
						{/if}
					</div>
				</Section>

				<GcpTriggerEditorConfigSection
					bind:isValid
					bind:gcp_resource_path
					bind:subscription_id
					bind:delivery_type
					bind:delivery_config
					bind:topic_id
					bind:subscription_mode
					bind:path
					cloud_subscription_id={subscription_id}
					create_update_subscription_id={subscription_id}
					{can_write}
					headless={true}
				/>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
