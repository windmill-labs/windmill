<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import type { GridItem, RichConfigurations } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'
	import GroupManagementDrawer from '../componentsPanel/GroupManagementDrawer.svelte'
	import { Plus } from 'lucide-svelte'
	import { Alert } from '$lib/components/common'

	interface Props {
		groupFields: RichConfigurations | undefined
		item: GridItem
	}

	let { groupFields = $bindable(), item }: Props = $props()

	let groupManagementDrawer: GroupManagementDrawer | undefined = $state(undefined)

	// const { app, runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	let fieldName: string = $state('')
	function addField(name: string) {
		if (name == '') return
		groupFields = {
			...(groupFields ?? {}),
			[name]: {
				type: 'static',
				value: '',
				fieldType: 'object'
			}
		}
	}
</script>

<div class="flex p-2 gap-1 items-center">
	<Toggle
		size="xs"
		checked={groupFields != undefined}
		on:change={(e) => {
			if (e.detail) {
				groupFields = {}
			} else {
				groupFields = undefined
			}
			console.log(groupFields)
		}}
		options={{
			right: 'container is a component group',
			rightTooltip: `Group fields allow inner components to depend on the group fields which make the container a
		group of component that is encapsulated. Inside the group, it is possible to retrieve the values
		using \`group.x\` where x is the group field name.

		Group fields are mutable by frontend scripts, so  \`group.x = 42\` can be used to set the value inside the group and
			 \`a.group.x = 42\` outside of it.`
		}}
	/>
</div>
{#if groupFields != undefined}
	<div class="p-2">
		<Button
			on:click={() => {
				groupManagementDrawer?.openDrawer()
			}}
			size="xs"
			color="light"
		>
			Save group to workspace
		</Button>
	</div>
	<PanelSection
		title={`Group Fields ${
			Object.keys(groupFields ?? {}).length > 0
				? `(${Object.keys(groupFields ?? {}).length ?? 0})`
				: ''
		}`}
	>
		{#if Object.keys(groupFields ?? {}).length == 0}
			<span class="text-xs text-tertiary">No group fields</span>
		{/if}
		<div class="w-full flex gap-2 flex-col mt-2">
			<InputsSpecsEditor
				on:delete={(e) => {
					if (!groupFields) {
						return
					}
					delete groupFields[e.detail]
					groupFields = groupFields
				}}
				id={item.id}
				shouldCapitalize={false}
				displayType
				deletable
				bind:inputSpecs={groupFields}
			/>
			<div class="flex flex-row gap-2 items-center relative">
				<input
					type="text"
					onkeydown={(event) => {
						event.stopPropagation()
						switch (event.key) {
							case 'Enter':
								event.preventDefault()
								addField(fieldName)
								break
						}
					}}
					placeholder="Group Field Name"
					bind:value={fieldName}
				/>

				<Button
					disabled={fieldName == ''}
					size="sm"
					color="light"
					variant="border"
					startIcon={{ icon: Plus }}
					on:click={() => addField(fieldName)}
					iconOnly
				/>
			</div>
		</div>

		<div class="mt-2"></div>
		<Alert size="xs" title="Group fields are mutable" type="info">
			You may set the value of a group field in a frontend script within the group using: <code
				>group.x = 42</code
			>
			and externally using <code>{item.id}.group.x = 42</code>
		</Alert>
	</PanelSection>
{/if}

<GroupManagementDrawer bind:this={groupManagementDrawer} {item} />
