<script lang="ts">
	import type { AppInputTransform } from '../../types'
	import DynamicInputEditor from './DynamicInputEditor.svelte'
	import StaticInputEditor from './StaticInputEditor.svelte'

	export let appInputTransform: AppInputTransform
	export let propKey: string
	export let componenId: string
</script>

{#if appInputTransform.type === 'static'}
	<StaticInputEditor {propKey} bind:input={appInputTransform} {componenId} />
{:else if appInputTransform.type === 'output'}
	<DynamicInputEditor bind:input={appInputTransform} />
{:else if appInputTransform.type === 'user'}
	<span>Not implemented</span>
{/if}
