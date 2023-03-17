<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import { findGridItem } from '../../appUtils'
	import TriggerBadgesList from './TriggerBadgesList.svelte'
	import { getDependencies, type DependencyBadge } from './triggerListUtils'

	const { selectedComponent, app } = getContext<AppViewerContext>('AppViewerContext')

	let badges: DependencyBadge = []
	$: gridItem = $selectedComponent ? findGridItem($app, $selectedComponent) : undefined

	let dependencies: Array<{ componentId: string; path: string }> | undefined = undefined

	$: if (gridItem && dependencies === undefined) {
		const fields =
			gridItem?.data?.componentInput?.type === 'runnable'
				? gridItem?.data?.componentInput?.fields
				: undefined

		if (!fields) {
			dependencies = []
		} else {
			dependencies = getDependencies(fields)
		}

		dependencies.forEach((dependency) => {
			badges.push({
				label: `${dependency.componentId} - ${dependency.path}`,
				color: 'red'
			})
		})
	}

	if (
		gridItem?.data.type === 'buttoncomponent' ||
		gridItem?.data.type === 'formbuttoncomponent' ||
		gridItem?.data.type === 'formcomponent'
	) {
		badges.push({
			label: 'On click',
			color: 'blue'
		})
	} else {
		badges.push({
			label: 'On load',
			color: 'green'
		})
	}
</script>

<TriggerBadgesList {badges} />
