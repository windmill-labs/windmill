<script lang="ts">
	import type {
		ConnectedAppInput,
		RowAppInput,
		RunnableByName,
		StaticAppInput,
		UserAppInput
	} from '$lib/components/apps/inputType'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import type { AppComponent } from '../../component'
	import TriggerBadgesList from './TriggerBadgesList.svelte'
	import { getDependencies } from './triggerListUtils'

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')
	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let appComponent: AppComponent
	export let runnable: RunnableByName

	const onClick = ['buttoncomponent', 'formbuttoncomponent', 'formcomponent'].includes(
		appComponent.type
	)

	$: onLoad =
		!onClick ||
		(appComponent?.configuration?.triggerOnAppLoad != undefined &&
			appComponent?.configuration?.triggerOnAppLoad?.ctype === undefined &&
			appComponent.configuration.triggerOnAppLoad.type == 'static' &&
			appComponent.configuration.triggerOnAppLoad.value)
</script>

<TriggerBadgesList
	inputDependencies={onClick ? [] : getDependencies(fields)}
	bind:inlineScript={runnable.inlineScript}
	{onLoad}
	id={$selectedComponent}
	{onClick}
/>
