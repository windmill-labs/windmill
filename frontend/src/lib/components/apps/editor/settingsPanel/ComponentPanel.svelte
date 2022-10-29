<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppComponent, AppEditorContext } from '../../types'

	export let component: AppComponent

	const { staticOutputs, worldStore } = getContext<AppEditorContext>('AppEditorContext')

	function deleteComponent() {
		component = undefined
	}
</script>

{#if component}
	{JSON.stringify($staticOutputs[component.id])}
	{JSON.stringify($worldStore?.outputsById[component.id])}

	<Button size="sm" color="red" startIcon={{ icon: faTrashAlt }} on:click={() => deleteComponent()}>
		Delete component
	</Button>
{:else}
	Empty component
{/if}
