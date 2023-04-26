<script lang="ts">
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '$lib/components/apps/inputType'
	import type { InlineScript } from '$lib/components/apps/types'

	import TriggerBadgesList from './TriggerBadgesList.svelte'
	import { getDependencies } from './triggerListUtils'

	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let autoRefresh: boolean = false
	export let id: string
	export let inlineScript: InlineScript
	export let recomputeOnInputChanged: boolean | undefined = true
	export let doNotRecomputeOnInputChanged: undefined | boolean = undefined

	//TODO: remove after migration is done
	$: {
		if (doNotRecomputeOnInputChanged != undefined) {
			recomputeOnInputChanged = !doNotRecomputeOnInputChanged
			doNotRecomputeOnInputChanged = undefined
		}

		if (recomputeOnInputChanged == undefined) {
			recomputeOnInputChanged = true
		}
	}

	$: dependencies = getDependencies(fields)
</script>

<TriggerBadgesList
	bind:inlineScript
	{id}
	inputDependencies={dependencies}
	onLoad={autoRefresh}
	{recomputeOnInputChanged}
/>
