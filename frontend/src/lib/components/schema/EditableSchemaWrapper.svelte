<script lang="ts">
	import type { Schema } from '$lib/common'
	import EditableSchemaForm from '../EditableSchemaForm.svelte'
	import AddProperty from './AddProperty.svelte'

	export let schema: Schema | undefined | any
	export let offset: number = 0
	export let uiOnly: boolean = false
	export let noPreview: boolean = false
	export let hideJsonToggle: boolean = false
	export let fullHeight: boolean = true

	let addProperty: AddProperty | undefined = undefined
</script>

<div class="border h-80 overflow-y-scroll">
	<div class="pt-6 px-4 border-b pb-4">
		<AddProperty isFlowInput bind:schema bind:this={addProperty} {hideJsonToggle} />
	</div>
	<EditableSchemaForm
		bind:schema
		isFlowInput
		on:edit={(e) => {
			addProperty?.openDrawer(e.detail)
		}}
		on:delete={(e) => {
			addProperty?.handleDeleteArgument([e.detail])
		}}
		{offset}
		{uiOnly}
		{noPreview}
		{fullHeight}
	/>
</div>
