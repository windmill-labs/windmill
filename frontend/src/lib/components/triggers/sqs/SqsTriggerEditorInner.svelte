<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { Loader2, Save, X, Pen, Trash } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { SqsTriggerService, type AwsAuthResourceType } from '$lib/gen'
	import SqsTriggerEditorConfigSection from './SqsTriggerEditorConfigSection.svelte'
	import Section from '$lib/components/Section.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Snippet } from 'svelte'

	interface Props {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		editMode?: boolean
		preventSave?: boolean
		hideTooltips?: boolean
		useEditButton?: boolean
	}

	let {
		useDrawer = true,
		description = undefined,
		hideTarget = false,
		editMode = true,
		preventSave = false,
		hideTooltips = false,
		useEditButton = false
	}: Props = $props()

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
	let showLoading = $state(false)
	let aws_resource_path: string = $state('')
	let queue_url = $state('')
	let message_attributes: string[] = $state([])
	let aws_auth_resource_type: AwsAuthResourceType = $state('credentials')
	let isValid = $state(false)
	let resetEditMode = $state<(() => void) | undefined>(undefined)
	let isDraft = $state(false)

	const dispatch = createEventDispatcher()

	$effect(() => {
		is_flow = itemKind === 'flow'
	})

	export async function openEdit(ePath: string, isFlow: boolean) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100) // Do not show loading spinner for the first 100ms
		drawerLoading = true
		resetEditMode = () => openEdit(ePath, isFlow)
		try {
			drawer?.openDrawer()
			initialPath = ePath
			itemKind = isFlow ? 'flow' : 'script'
			edit = true
			isDraft = false
			dirtyPath = false
			await loadTrigger()
		} catch (err) {
			sendUserToast(`Could not load sqs trigger: ${err.body}`, true)
		} finally {
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		defaultValues?: Record<string, any>
	) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100)
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
			isDraft = true
			dirtyPath = false
		} finally {
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
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
		dispatch('update', path)
		drawer?.closeDrawer()
		toggleEditMode(false)
	}

	function toggleEditMode(newEditMode: boolean) {
		dispatch('toggle-edit-mode', newEditMode)
	}

	$effect(() => {
		dispatch('update-config', {
			aws_resource_path,
			queue_url,
			message_attributes,
			aws_auth_resource_type,
			isValid
		})
	})
</script>

{#if useDrawer}
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
				{@render actionsButtons('sm')}
			</svelte:fragment>
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label="SQS trigger">
		<svelte:fragment slot="action">
			{@render actionsButtons('xs')}
		</svelte:fragment>
		{@render config()}
	</Section>
{/if}

{#snippet actionsButtons(size: 'xs' | 'sm' = 'sm')}
	{#if !drawerLoading && can_write}
		<div class="flex flex-row gap-2 items-center">
			{#if isDraft}
				<Button
					{size}
					startIcon={{ icon: Trash }}
					iconOnly
					color={'light'}
					on:click={() => {
						dispatch('delete')
					}}
					btnClasses="hover:bg-red-500 hover:text-white"
				/>
			{/if}
			{#if !isDraft && edit}
				<div class={twMerge('center-center', size === 'sm' ? '-mt-1' : '')}>
					<Toggle
						{size}
						disabled={!can_write || !editMode}
						checked={enabled}
						options={{ right: 'enable', left: 'disable' }}
						on:change={async (e) => {
							await SqsTriggerService.setSqsTriggerEnabled({
								path: initialPath,
								workspace: $workspaceStore ?? '',
								requestBody: { enabled: e.detail }
							})
							sendUserToast(`${e.detail ? 'enabled' : 'disabled'} SQS trigger ${initialPath}`)
						}}
					/>
				</div>
			{/if}
			{#if !preventSave}
				{#if can_write && editMode}
					<Button
						{size}
						startIcon={{ icon: Save }}
						disabled={pathError != '' || emptyString(script_path) || !isValid || !can_write}
						on:click={updateTrigger}
					>
						Save
					</Button>
				{/if}
				{#if !editMode && useEditButton}
					<Button
						{size}
						color="light"
						startIcon={{ icon: Pen }}
						on:click={() => toggleEditMode(true)}
					>
						Edit
					</Button>
				{:else if editMode && !!resetEditMode && useEditButton}
					<Button
						{size}
						color="light"
						startIcon={{ icon: X }}
						on:click={() => {
							toggleEditMode(false)
							resetEditMode?.()
						}}
					>
						Cancel
					</Button>
				{/if}
			{/if}
		</div>
	{/if}
{/snippet}

{#snippet config()}
	{#if drawerLoading}
		{#if showLoading}
			<Loader2 class="animate-spin" />
		{/if}
	{:else}
		<div class="flex flex-col gap-4">
			{#if description}
				{@render description()}
			{/if}
			{#if !hideTooltips}
				<Alert title="Info" type="info" size="xs">
					{#if edit}
						Changes can take up to 30 seconds to take effect.
					{:else}
						New SQS triggers can take up to 30 seconds to start listening.
					{/if}
				</Alert>
			{/if}
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
						disableEditing={!editMode}
					/>
				</Label>
			</div>

			{#if !hideTarget}
				<Section label="Runnable">
					<p class="text-xs mb-1 text-tertiary">
						Pick a script or flow to be triggered <Required required={true} />
					</p>
					<div class="flex flex-row mb-2">
						<ScriptPicker
							disabled={fixedScriptPath != '' || !can_write || !editMode}
							initialPath={fixedScriptPath || initialScriptPath}
							kinds={['script']}
							allowFlow={true}
							bind:itemKind
							bind:scriptPath={script_path}
							allowRefresh={can_write}
							allowEdit={!$userStore?.operator}
						/>
						{#if emptyString(script_path)}
							<Button
								btnClasses="ml-4 mt-2"
								color="dark"
								size="xs"
								href={itemKind === 'flow' ? '/flows/add?hub=59' : '/scripts/add?hub=hub%2F11637'}
								target="_blank"
								disabled={!editMode}
							>
								Create from template
							</Button>
						{/if}
					</div>
				</Section>
			{/if}

			<SqsTriggerEditorConfigSection
				bind:isValid
				bind:queue_url
				bind:message_attributes
				bind:aws_resource_path
				bind:aws_auth_resource_type
				can_write={can_write && editMode}
				headless={true}
			/>
		</div>
	{/if}
{/snippet}
