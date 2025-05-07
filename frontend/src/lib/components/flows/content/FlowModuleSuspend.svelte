<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import InputTransformForm from '$lib/components/InputTransformForm.svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { getContext, tick } from 'svelte'

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
	import EditableSchemaDrawer from '$lib/components/schema/EditableSchemaDrawer.svelte'
	import AddProperty from '$lib/components/schema/AddProperty.svelte'

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
				enum: allUserGroups
			}
		}
	}

	$: {
		if ($workspaceStore && allUserGroups.length === 0) {
			loadGroups()
		}
	}

	let jsonView: boolean = false
</script>

<Section label="Suspend/Approval/Prompt" class="w-full">
	<svelte:fragment slot="action">
		<SuspendDrawer text="Approval/Prompt helpers" />
	</svelte:fragment>
	<svelte:fragment slot="header">
		<div class="flex flex-row items-center gap-2">
			<Tooltip documentationLink="https://www.windmill.dev/docs/flows/flow_approval">
				If defined, at the end of the step, the flow will be suspended until it receives external
				requests to be resumed or canceled. This is most useful to implement approval steps but can
				be used flexibly for other purposes.
			</Tooltip>
			<Toggle
				size="xs"
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
		</div>
	</svelte:fragment>

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

			<Toggle
				options={{
					right: 'Continue on disapproval/timeout',
					rightTooltip: `Instead of failing the flow and bubbling up the error, continue to the next step which would allow to put a branchone right after to handle both cases separately. 
						If any disapproval/timeout event is received, the resume payload will be similar to every error result in Windmill, an object containing an "error" field which you can use 
						to distinguish between approvals and disapproval/timeouts. 
						
						We recommend using the expr "resume?.error" to handle null payload values. 
						To filter timeout, use "resume?.error?.name === "SuspendedTimedOut" 
						To filter disapproval, use "resume?.error?.name === "SuspendedDisapproved"`
				}}
				checked={Boolean(flowModule.suspend?.continue_on_disapprove_timeout)}
				disabled={!Boolean(flowModule.suspend)}
				on:change={(e) => {
					if (flowModule.suspend) {
						flowModule.suspend.continue_on_disapprove_timeout = e.detail
					}
				}}
			/>
			{#if Boolean(flowModule.suspend?.continue_on_disapprove_timeout)}
				<Alert type="info" title="Continue on disapproval/timeout">
					We recommend using the expr <code>resume?.error</code> to handle null payload values.
					<br />
					To filter timeout, use <code>resume?.error?.name === "SuspendedTimedOut"</code>. <br />
					To filter disapproval, use <code>resume?.error?.name === "SuspendedDisapproved"</code>
				</Alert>
			{/if}
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

					<div class="mb-4"></div>

					{#if Boolean(flowModule.suspend.user_auth_required) && allUserGroups.length !== 0 && flowModule.suspend && schema.properties['groups']}
						<span class="text-xs font-bold"
							>Require approvers to be members of one of the following user groups (leave empty for
							any)
						</span>
						<div class="border">
							<PropPickerWrapper
								{result}
								noFlowPlugConnect
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
									bind:editor
								/>
							</PropPickerWrapper>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	{:else}
		<div class="grid grid-cols-4 mt-4 gap-8">
			<div class="col-span-2">
				{#if flowModule?.suspend?.resume_form}
					<EditableSchemaDrawer
						bind:schema={flowModule.suspend.resume_form.schema}
						on:change={(e) => {
							const schema = e.detail

							// If the schema is empty, remove the form
							if (Object.keys(schema?.properties ?? {}).length === 0) {
								tick().then(() => {
									if (!flowModule.suspend) return
									flowModule.suspend.resume_form = undefined
								})
							}
						}}
						{jsonView}
					/>
				{:else if emptyString($enterpriseLicense)}
					<Alert type="warning" title="Adding a form to the approval page is an EE feature" />
				{:else}
					<div class="flex flex-col items-end mb-2 w-full">
						<Toggle
							checked={false}
							label="JSON View"
							size="xs"
							options={{
								right: 'JSON editor',
								rightTooltip:
									'Arguments can be edited either using the wizard, or by editing their JSON Schema.'
							}}
							lightMode
							on:change={() => {
								if (flowModule.suspend) {
									flowModule.suspend.resume_form = {
										schema: emptySchema()
									}
								}
								jsonView = true
							}}
						/>
					</div>
					<AddProperty
						on:change={(e) => {
							jsonView = false
							if (flowModule.suspend) {
								flowModule.suspend.resume_form = {
									schema: e.detail
								}
							}
						}}
						schema={{}}
					/>
				{/if}
			</div>
			<div class="col-span-2 flex flex-col gap-4">
				{#if flowModule.suspend}
					{#if emptyString($enterpriseLicense)}
						<Alert type="warning" title="Adding a form to the approval page is an EE feature" />
					{/if}

					<div class="flex flex-col gap-2">
						<div class="flex">
							<SuspendDrawer text="Default args & Dynamic enums help" />
						</div>
					</div>
				{/if}
				{#if flowModule.suspend}
					<Toggle
						bind:checked={flowModule.suspend.hide_cancel}
						size="xs"
						options={{
							right: 'Hide cancel button on approval page'
						}}
						disabled={!Boolean(flowModule?.suspend?.resume_form)}
					/>
				{/if}
			</div>
		</div>
	{/if}
</Section>
