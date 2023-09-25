<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import type { RichConfigurations } from '../../types'
	import type { AppComponent } from '../component'
	import PanelSection from './common/PanelSection.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'

	export let groupFields: RichConfigurations | undefined

	export let component: AppComponent

	// const { app, runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	let fieldName: string = ''
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
		using \`group.<x />\` where x is the group field name`
		}}
	/>
</div>
{#if groupFields != undefined}
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
				id={component.id}
				shouldCapitalize={false}
				displayType
				deletable
				bind:inputSpecs={groupFields}
			/>
			<div class="flex flex-row gap-2 items-center relative">
				<input
					type="text"
					on:keydown|stopPropagation={(event) => {
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
					startIcon={{ icon: faPlus }}
					on:click={() => addField(fieldName)}
					iconOnly
				/>
			</div>
		</div>
	</PanelSection>
{/if}
