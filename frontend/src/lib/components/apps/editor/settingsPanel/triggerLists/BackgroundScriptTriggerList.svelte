<script lang="ts">
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '$lib/components/apps/inputType'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import { getAllRecomputeIdsForComponent } from '../../appUtils'
	import TriggerBadgesList from './TriggerBadgesList.svelte'
	import { getDependencies, type DependencyBadge } from './triggerListUtils'

	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let autoRefresh: boolean = false
	export let id: string
	export let refreshOn: { id: string; key: string }[] | undefined = undefined

	const badges: DependencyBadge = []
	const dependencies = getDependencies(fields)
	const { app, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	$: if ($app && $selectedComponent) {
		const recomputeIds = getAllRecomputeIdsForComponent($app, id)

		if ($selectedComponent && recomputeIds) {
			recomputeIds.forEach((x) => {
				badges.push({
					label: `Recomputed by: ${x}`,
					color: 'indigo'
				})
			})
		}
	}

	dependencies.forEach((dependency) => {
		badges.push({
			label: `${dependency.componentId} - ${dependency.path}`,
			color: 'orange'
		})
	})

	$: if (refreshOn) {
		refreshOn.forEach((x) => {
			badges.push({
				label: `Refresh on: ${x.id} - ${x.key}`,
				color: 'yellow'
			})
		})
	}
</script>

<TriggerBadgesList {badges} onLoad={autoRefresh} />
