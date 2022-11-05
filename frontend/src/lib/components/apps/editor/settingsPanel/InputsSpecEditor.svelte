<script lang="ts">
	import { Badge } from '$lib/components/common'
	import type { AppInputTransform } from '../../types'
	import StaticInputEditor from './StaticInputEditor.svelte'

	export let appInputTransform: AppInputTransform
	export let propKey: string
	export let componenId: string
</script>

{#if appInputTransform.type === 'static'}
	<StaticInputEditor {propKey} input={appInputTransform} {componenId} />
{:else if appInputTransform.type === 'output'}
	{#if appInputTransform.id && appInputTransform.name}
		<div class="flex justify-between w-full">
			<span class="text-xs font-bold">Component</span>
			<Badge color="indigo">{appInputTransform.id}</Badge>
		</div>
		<div class="flex justify-between w-full">
			<span class="text-xs font-bold">Field name</span>
			<Badge color="green">{appInputTransform.name}</Badge>
		</div>
	{:else}
		<span>Not connected</span>
	{/if}
{:else if appInputTransform.type === 'user'}
	<span>Not implemented</span>
{/if}
