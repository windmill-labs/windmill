<script lang="ts">
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '$lib/components/apps/inputType'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import { findGridItem } from '../../appUtils'
	import TriggerBadgesList from './TriggerBadgesList.svelte'
	import { getDependencies } from './triggerListUtils'

	const { selectedComponent, app } = getContext<AppViewerContext>('AppViewerContext')
	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>

	$: gridItem = $selectedComponent ? findGridItem($app, $selectedComponent) : undefined

	$: onClick =
		gridItem?.data.type &&
		['buttoncomponent', 'formbuttoncomponent', 'formcomponent'].includes(gridItem?.data.type)

	$: if (gridItem) {
		console.log(gridItem?.data.configuration.triggerOnAppLoad)
	}

	$: onLoad =
		!onClick ||
		(gridItem?.data.configuration.triggerOnAppLoad.ctype === undefined &&
			// TODO: Type and value are not in sync.
			// @ts-ignore
			gridItem?.data.configuration.triggerOnAppLoad.value)
</script>

<TriggerBadgesList
	inputDependencies={getDependencies(fields)}
	{onLoad}
	id={$selectedComponent}
	{onClick}
/>
