<script lang="ts">
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '$lib/components/apps/inputType'
	import type { InlineScript } from '$lib/components/apps/types'
	import Toggle from '$lib/components/Toggle.svelte'

	import TriggerBadgesList from './TriggerBadgesList.svelte'
	import { getDependencies } from './triggerListUtils'

	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let autoRefresh: boolean = false
	export let id: string
	export let inlineScript: InlineScript
	export let recomputeOnInputChanged: boolean | undefined = true
	export let doNotRecomputeOnInputChanged: undefined | boolean = undefined

	if (doNotRecomputeOnInputChanged == true) {
		recomputeOnInputChanged = false
	}

	if (recomputeOnInputChanged == undefined) {
		recomputeOnInputChanged = true
	}

	$: dependencies = getDependencies(fields)
</script>

{#if inlineScript.language !== 'frontend'}
	<div class="flex items-center px-1">
		<Toggle
			size="xs"
			bind:checked={recomputeOnInputChanged}
			options={{ right: 'recompute on any input changes' }}
		/>
	</div>
{/if}

<TriggerBadgesList
	bind:inlineScript
	{id}
	inputDependencies={dependencies}
	onLoad={autoRefresh}
	{recomputeOnInputChanged}
/>
