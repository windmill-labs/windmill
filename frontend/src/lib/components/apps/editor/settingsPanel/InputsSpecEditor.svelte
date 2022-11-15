<script lang="ts">
	import type { AppInputTransform } from '../../types'
	import DynamicInputEditor from './DynamicInputEditor.svelte'
	import StaticInputEditor from './StaticInputEditor.svelte'

	export let appInputTransform: AppInputTransform
</script>

{#if appInputTransform.type === 'static'}
	<StaticInputEditor bind:input={appInputTransform} />
{:else if appInputTransform.type === 'output'}
	<DynamicInputEditor bind:input={appInputTransform} />
{/if}
