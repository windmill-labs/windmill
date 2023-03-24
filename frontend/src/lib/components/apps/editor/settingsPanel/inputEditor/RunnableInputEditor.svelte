<script lang="ts">
	import type { ResultAppInput } from '$lib/components/apps/inputType'
	import { isScriptByNameDefined, isScriptByPathDefined } from '$lib/components/apps/utils'
	import type { AppComponent } from '../../component'
	import RunnableSelector from '../mainInput/RunnableSelector.svelte'
	import SelectedRunnable from '../SelectedRunnable.svelte'

	export let appInput: ResultAppInput
	export let defaultUserInput = false
	export let appComponent: AppComponent
</script>

{#if isScriptByPathDefined(appInput) || isScriptByNameDefined(appInput)}
	<SelectedRunnable {appComponent} bind:appInput />
{:else if appInput !== undefined}
	<RunnableSelector id={appComponent.id} {defaultUserInput} bind:appInput />
{/if}
