<script lang="ts">
	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
	import Slider from '$lib/components/Slider.svelte'
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

	const { selectedId, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const result = $flowStateStore[$selectedId]?.previewResult ?? {}
	let editor: SimpleEditor | undefined = undefined

	export let flowModule: FlowModule
	export let previousModuleId: string | undefined

	let allUserGroups: string[] = []
	let suspendTabSelected: 'core' | 'form' | 'permissions' = 'core'

	let schema = emptySchema()

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
</script>

<h2 class="pb-4">
	Suspend/Approval
	<Tooltip documentationLink="https://www.windmill.dev/docs/flows/flow_approval">
		If defined, at the end of the step, the flow will be suspended until it receives external
		requests to be resumed or canceled. This is most useful to implement approval steps but can be
		used flexibly for other purpose.
	</Tooltip>
</h2>

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
		<span class="text-xs font-bold">Number of approvals/events required for resuming flow</span>

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

		<span class="text-xs font-bold">Timeout</span>

		{#if flowModule.suspend}
			<SecondsInput bind:seconds={flowModule.suspend.timeout} />
		{:else}
			<SecondsInput disabled />
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
							}
						}
					}}
				/>
				<div class="mb-4" />

				<span class="text-xs font-bold"
					>Require approvers to be members of one of the following user groups (leave empty for any)
				</span>
				{#if allUserGroups.length !== 0 && flowModule.suspend && schema.properties['groups']}
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
			<Toggle
				checked={Boolean(flowModule.suspend.resume_form)}
				options={{
					right: 'Add a form to the approval page'
				}}
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
			<div>
				<Slider size="xs" text="How to add dynamic default args">
					As one of the return key of this step, return an object `default_args` that contains the
					default arguments of the form arguments. e.g:
					<pre
						><code
							>{`return {
	endpoints,
	default_args: {
		foo: "foo",
		bar: true,
	},
}`}</code
						></pre
					>
				</Slider></div
			>
		{/if}
		{#if flowModule.suspend?.resume_form}
			<SchemaEditor bind:schema={flowModule.suspend.resume_form.schema} />
		{/if}
	</div>
{/if}
