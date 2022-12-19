<script lang="ts">
	import type { ResultAppInput } from '$lib/components/apps/inputType'
	import { isScriptByNameDefined, isScriptByPathDefined } from '$lib/components/apps/utils'
	import RunnableSelector from '../mainInput/RunnableSelector.svelte'
	import SelectedRunnable from '../SelectedRunnable.svelte'

	export let appInput: ResultAppInput

	$: isRunnableSelected = isScriptByPathDefined(appInput) || isScriptByNameDefined(appInput)
</script>

{#if isRunnableSelected}
	<SelectedRunnable bind:appInput />
{:else}
	<RunnableSelector bind:appInput />
{/if}
