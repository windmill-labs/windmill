<script lang="ts">
	import type { ComponentCustomCSS, RichConfigurations } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	// @ts-ignore
	import MultiSelect from 'svelte-multiselect'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'multiselectcomponent'> | undefined = undefined
	export let render: boolean
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
</script>

<InitializeComponent {id} />

<AlignWrapper {render} hFull {verticalAlignment}>
	<div
		>This version of the multiselect component is not supported anymore. Recreate the multiselect
		component to use this new version.</div
	>
</AlignWrapper>
