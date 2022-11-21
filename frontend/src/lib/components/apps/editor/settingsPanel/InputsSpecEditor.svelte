<script lang="ts">
	import type { AppInputTransform } from '../../types'
	import DynamicInputEditor from './DynamicInputEditor.svelte'
	import StaticInputEditor from './StaticInputEditor.svelte'

	export let appInputTransform: AppInputTransform
	export let canHide: boolean = false
</script>

{#if appInputTransform.type === 'static'}
	<StaticInputEditor bind:input={appInputTransform} {canHide} />
{:else if appInputTransform.type === 'output'}
	<DynamicInputEditor bind:input={appInputTransform} />
{/if}
