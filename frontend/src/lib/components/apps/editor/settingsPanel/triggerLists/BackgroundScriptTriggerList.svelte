<script lang="ts">
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '$lib/components/apps/inputType'
	import type { InlineScript } from '$lib/components/apps/types'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import TriggerBadgesList from './TriggerBadgesList.svelte'
	import { getDependencies } from './triggerListUtils'

	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let autoRefresh: boolean = false
	export let id: string
	export let inlineScript: InlineScript
	export let doNotRecomputeOnInputChanged: boolean = false

	$: dependencies = getDependencies(fields)
</script>

{#if inlineScript.language !== 'frontend'}
	<div class="flex items-center px-1">
		<Toggle
			bind:checked={doNotRecomputeOnInputChanged}
			options={{ right: "Don't recompute on input changed" }}
		/>
		<Tooltip>Whenever an input is changed, the script will be re-run.</Tooltip>
	</div>
{/if}

<TriggerBadgesList
	bind:inlineScript
	{id}
	inputDependencies={dependencies}
	onLoad={autoRefresh}
	{doNotRecomputeOnInputChanged}
/>
