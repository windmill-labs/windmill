<script lang="ts">
	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { getContext } from 'svelte'

	import { Alert, Tab, Tabs } from '$lib/components/common'
	import { GroupService, type FlowModule } from '$lib/gen'
	import { emptySchema, emptyString } from '$lib/utils'
	import { enterpriseLicense, workspaceStore } from '$lib/stores.js'
	import { SecondsInput } from '../../common'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import type { FlowEditorContext } from '../types'
	import Section from '$lib/components/Section.svelte'
	import Label from '$lib/components/Label.svelte'
	import SuspendDrawer from './SuspendDrawer.svelte'

	const { selectedId, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const result = $flowStateStore[$selectedId]?.previewResult ?? {}
	let editor: SimpleEditor | undefined = undefined

	export let flowModule: FlowModule
	export let previousModuleId: string | undefined

	let schema = emptySchema()

	let allUserGroups: string[] = []
	let suspendTabSelected: 'core' | 'form' | 'permissions' = 'core'

	$: isSuspendEnabled = Boolean(flowModule.suspend)

	async function loadGroups(): Promise<void> {
		allUserGroups = await GroupService.listGroupNames({ workspace: $workspaceStore! })
		schema.properties['groups'] = {
			type: 'array',
			items: {
				type: 'string',
				multiselect: allUserGroups
			}
		}
	}

	$: {
		if ($workspaceStore && allUserGroups.length === 0) {
			loadGroups()
		}
	}
</script>

<Section label="Suspend/Approval/Prompt" class="w-full">
	<svelte:fragment slot="header">
		<Tooltip documentationLink="https://www.windmill.dev/docs/flows/flow_approval">
			If defined, at the end of the step, the flow will be suspended until it receives external
			requests to be resumed or canceled. This is most useful to implement approval steps but can be
			used flexibly for other purposes.
		</Tooltip>
		<div class="ml-4">
			<div class="flex">
				<SuspendDrawer text="Approval/Prompt helpers" />
			</div>
		</div>
	</svelte:fragment>

	<Toggle
		checked={isSuspendEnabled}
		on:change={() => {
			if (isSuspendEnabled && flowModule.suspend != undefined) {
				flowModule.suspend = undefined
			} else {
				flowModule.suspend = {
					required_events: 1,
					timeout: 1800
				}
			}
		}}
		options={{
			right: 'Suspend flow execution until events/approvals received'
		}}
	/>

	<div class="overflow-x-auto scrollbar-hidden">
		<Tabs bind:selected={suspendTabSelected}>
			<Tab size="xs" value="core" disabled={!isSuspendEnabled}>
				<div class="flex gap-2 items-center my-1">Core</div>
			</Tab>
			<Tab size="xs" value="form" disabled={!isSuspendEnabled}>
				<div class="flex gap-2 items-center my-1">Form</div>
			</Tab>
			<Tab size="xs" value="permissions" disabled={!isSuspendEnabled}>
				<div class="flex gap-2 items-center my-1">Permissions</div>
			</Tab>
		</Tabs>
	</div>

	{#if suspendTabSelected === 'core'}
		<div class="flex flex-col mt-4 gap-4">
			<Label label="Number of approvals/events required for resuming flow">
				{#if flowModule.suspend}
					<input
						bind:value={flowModule.suspend.required_events}
						type="number"
						min="1"
						placeholder="1"
					/>
				{:else}
					<input type="number" disabled />
				{/if}
			</Label>
			<Label label="Timeout">
				{#if flowModule.suspend}
					<SecondsInput bind:seconds={flowModule.suspend.timeout} />
				{:else}
					<SecondsInput disabled />
				{/if}
			</Label>
		</div>
	{:else if suspendTabSelected === 'permissions'}
		<div class="flex flex-col mt-4 gap-4">
			{#if emptyString($enterpriseLicense)}
				<Alert type="warning" title="Editing permissions is only available in enterprise version" />
			{/if}
			{#if flowModule.suspend}
				<div class="flex flex-col gap-2">
					<Toggle
						disabled={emptyString($enterpriseLicense)}
						checked={Boolean(flowModule.suspend.user_auth_required)}
						options={{
							right: 'Require approvers to be logged in'
						}}
						on:change={(e) => {
							if (flowModule.suspend) {
								flowModule.suspend.user_auth_required = e.detail
								if (e.detail && flowModule.suspend?.user_groups_required === undefined) {
									flowModule.suspend.user_groups_required = {
										type: 'static',
										value: []
									}
								} else if (!e.detail) {
									flowModule.suspend.user_groups_required = undefined
									flowModule.suspend.self_approval_disabled = false
								}
							}
						}}
					/>

					<Toggle
						options={{
							right: 'Disable self-approval',
							rightTooltip: 'The user who triggered the flow will not be allowed to approve it'
						}}
						checked={Boolean(flowModule.suspend.self_approval_disabled)}
						disabled={!Boolean(flowModule.suspend.user_auth_required)}
						on:change={(e) => {
							if (flowModule.suspend) {
								flowModule.suspend.self_approval_disabled = e.detail
							}
						}}
					/>

					<div class="mb-4" />

					{#if Boolean(flowModule.suspend.user_auth_required) && allUserGroups.length !== 0 && flowModule.suspend && schema.properties['groups']}
						<span class="text-xs font-bold"
							>Require approvers to be members of one of the following user groups (leave empty for
							any)
						</span>
						<div class="border">
							<PropPickerWrapper
								{result}
								displayContext={false}
								pickableProperties={undefined}
								on:select={({ detail }) => {
									editor?.insertAtCursor(detail)
									editor?.focus()
								}}
							>
								<InputTransformForm
									class="min-h-[256px] items-start"
									bind:arg={flowModule.suspend.user_groups_required}
									argName="groups"
									{schema}
									{previousModuleId}
								/>
							</PropPickerWrapper>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	{:else}
		<div class="flex flex-col mt-4 gap-4">
			{#if flowModule.suspend}
				{#if emptyString($enterpriseLicense)}
					<Alert type="warning" title="Adding a form to the approval page is an EE feature" />
				{/if}

				<div class="flex gap-4">
					<Toggle
						checked={Boolean(flowModule.suspend.resume_form)}
						options={{
							right: 'Add a form to the approval page'
						}}
						disabled={emptyString($enterpriseLicense)}
						on:change={(e) => {
							if (flowModule.suspend) {
								if (e.detail) {
									flowModule.suspend.resume_form = {
										schema: emptySchema()
									}
								} else {
									flowModule.suspend.resume_form = undefined
								}
							}
						}}
					/>
					<div class="flex">
						<SuspendDrawer text="Default args & Dynamic enums help" />
					</div>
				</div>
			{/if}
			{#if flowModule.suspend?.resume_form}
				<SchemaEditor bind:schema={flowModule.suspend.resume_form.schema} />
			{/if}
		</div>
	{/if}
</Section>
