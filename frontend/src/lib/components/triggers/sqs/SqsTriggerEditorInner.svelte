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
	import { SqsTriggerService, type AwsAuthResourceType } from '$lib/gen'
	import SqsTriggerEditorConfigSection from './SqsTriggerEditorConfigSection.svelte'
	import Section from '$lib/components/Section.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'

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
	let aws_resource_path: string = ''
	let queue_url = ''
	let message_attributes: string[] = []
	let aws_auth_resource_type: AwsAuthResourceType = 'credentials'
	let isValid = false
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
			sendUserToast(`Could not load sqs trigger: ${err.body}`, true)
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
			aws_resource_path = defaultValues?.aws_resource_path ?? ''
			queue_url = defaultValues?.queue_url ?? ''
			path = ''
			message_attributes = defaultValues?.message_attributes ?? []
			aws_auth_resource_type = defaultValues?.aws_auth_resource_type ?? 'credentials'
			initialPath = ''
			edit = false
			dirtyPath = false
		} finally {
			drawerLoading = false
		}
	}

	async function loadTrigger(): Promise<void> {
		try {
			const s = await SqsTriggerService.getSqsTrigger({
				workspace: $workspaceStore!,
				path: initialPath
			})
			script_path = s.script_path
			initialScriptPath = s.script_path
			aws_resource_path = s.aws_resource_path
			queue_url = s.queue_url
			is_flow = s.is_flow
			message_attributes = s.message_attributes ?? []
			path = s.path
			enabled = s.enabled
			aws_auth_resource_type = s.aws_auth_resource_type
			can_write = canWrite(s.path, s.extra_perms, $userStore)
		} catch (error) {
			sendUserToast(`Could not load SQS trigger: ${error.body}`, true)
		}
	}

	async function updateTrigger(): Promise<void> {
		if (edit) {
			await SqsTriggerService.updateSqsTrigger({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					path,
					script_path,
					aws_auth_resource_type,
					enabled,
					is_flow,
					queue_url,
					aws_resource_path,
					message_attributes
				}
			})
			sendUserToast(`SQS trigger ${path} updated`)
		} else {
			await SqsTriggerService.createSqsTrigger({
				workspace: $workspaceStore!,
				requestBody: {
					enabled: true,
					aws_resource_path,
					queue_url,
					aws_auth_resource_type,
					path,
					script_path,
					is_flow,
					message_attributes
				}
			})
			sendUserToast(`SQS trigger ${path} created`)
		}

		if (!$usedTriggerKinds.includes('sqs')) {
			$usedTriggerKinds = [...$usedTriggerKinds, 'sqs']
		}
		dispatch('update')
		drawer.closeDrawer()
	}
</script>

<Drawer size="800px" bind:this={drawer}>
	<DrawerContent
		title={edit
			? can_write
				? `Edit SQS trigger ${initialPath}`
				: `SQS trigger ${initialPath}`
			: 'New SQS trigger'}
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
								sendUserToast(`${e.detail ? 'enabled' : 'disabled'} sqs trigger ${initialPath}`)
							}}
						/>
					</div>
				{/if}
				<Button
					startIcon={{ icon: Save }}
					disabled={pathError != '' || emptyString(script_path) || !isValid || !can_write}
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
						New sqs triggers can take up to 30 seconds to start listening.
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
							namePlaceholder="sqs_trigger"
							kind="sqs_trigger"
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
								href={itemKind === 'flow' ? '/flows/add?hub=59' : '/scripts/add?hub=hub%2F11637'}
								target="_blank">Create from template</Button
							>
						{/if}
					</div>
				</Section>

				<SqsTriggerEditorConfigSection
					bind:isValid
					bind:queue_url
					bind:message_attributes
					bind:aws_resource_path
					bind:aws_auth_resource_type
					{can_write}
					headless={true}
				/>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
