<script lang="ts">
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	interface Props {
		id: string
		render: boolean
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
	}

	let { id, render, verticalAlignment = undefined }: Props = $props()
</script>

<InitializeComponent {id} />

<AlignWrapper {render} hFull {verticalAlignment}>
	<div
		>This version of the multiselect component is not supported anymore. Recreate the multiselect
		component to use this new version.</div
	>
</AlignWrapper>
