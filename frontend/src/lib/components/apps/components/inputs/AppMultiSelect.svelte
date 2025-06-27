<script lang="ts">
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	export let id: string
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
