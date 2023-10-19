<script lang="ts">
	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
	import Slider from '$lib/components/Slider.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import { Alert, Tab, Tabs } from '$lib/components/common'
	import { GroupService, type FlowModule } from '$lib/gen'
	import { emptySchema, emptyString } from '$lib/utils'
	import { enterpriseLicense, workspaceStore } from '$lib/stores.js'
	import { SecondsInput } from '../../common'
	import Multiselect from 'svelte-multiselect'

	export let flowModule: FlowModule

	export let allUserGroups: string[] = []
	let selectedUserGroups: string[] | undefined
	let suspendTabSelected: 'core' | 'form' | 'permissions' = 'core'

	$: isSuspendEnabled = Boolean(flowModule.suspend)

	async function loadGroups(): Promise<void> {
		allUserGroups = await GroupService.listGroupNames({ workspace: $workspaceStore! })
	}

	$: {
		if ($workspaceStore) {
			loadGroups()
		}
		switch (flowModule.suspend?.user_groups_required?.type) {
			case 'static':
				if (flowModule.suspend?.user_groups_required.value === undefined) {
					selectedUserGroups = []
				} else {
					selectedUserGroups = flowModule.suspend?.user_groups_required.value
				}
				break
			case 'javascript':
				console.warn('javascript input transform not supported yet')
				break
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
						}
					}}
				/>

				<div class="mb-4" />

				<span class="text-xs font-bold"
					>Require approvers to be members of one of the following user groups (leave empty for any)
				</span>
				{#if allUserGroups.length !== 0}
					<Multiselect
						disabled={emptyString($enterpriseLicense) || !flowModule.suspend.user_auth_required}
						on:change={(e) => {
							if (flowModule.suspend) {
								flowModule.suspend.user_groups_required = {
									value: selectedUserGroups,
									type: 'static'
								}
							}
						}}
						bind:selected={selectedUserGroups}
						options={allUserGroups}
						selectedOptionsDraggable={false}
						placeholder="Authorized user groups"
						ulOptionsClass={'!bg-surface-secondary'}
					/>
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
