<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import type { RichConfiguration } from '../../types'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import { slide } from 'svelte/transition'

	export let id: string
	export let field: RichConfiguration

	let disablable = field && !(field?.type === 'static' && field?.value === false)
</script>

<Toggle
	bind:checked={disablable}
	size="xs"
	options={{
		right: 'Can be disabled'
	}}
	on:change={() => {
		if (disablable) {
			field = {
				type: 'eval',
				expr: 'false',
				fieldType: 'boolean'
			}
		} else {
			field = {
				type: 'static',
				value: false,
				fieldType: 'boolean'
			}
		}
	}}
/>

{#if disablable}
	<div transition:slide|local>
		<InputsSpecEditor
			key={`Tab disabled`}
			bind:componentInput={field}
			{id}
			userInputEnabled={false}
			shouldCapitalize={true}
			resourceOnly={false}
			fieldType={field?.['fieldType']}
			subFieldType={field?.['subFieldType']}
			format={field?.['format']}
			selectOptions={field?.['selectOptions']}
			tooltip={field?.['tooltip']}
			fileUpload={field?.['fileUpload']}
			placeholder={field?.['placeholder']}
			customTitle={field?.['customTitle']}
			displayType={false}
		/>
	</div>
{/if}
