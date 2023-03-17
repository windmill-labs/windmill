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
	import { getDependencies, type DependencyBadge } from './triggerListUtils'

	const { selectedComponent, app } = getContext<AppViewerContext>('AppViewerContext')

	let badges: DependencyBadge = []
	$: gridItem = $selectedComponent ? findGridItem($app, $selectedComponent) : undefined
	$: fields =
		gridItem?.data?.componentInput?.type === 'runnable'
			? gridItem?.data?.componentInput?.fields
			: undefined

	let dependencies: Array<{ componentId: string; path: string }> | undefined = undefined

	function getBadges(
		fields:
			| Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
			| undefined
	) {
		if (!fields) {
			dependencies = []
		} else {
			dependencies = getDependencies(fields)
		}

		dependencies.forEach((dependency) => {
			badges.push({
				label: `${dependency.componentId} - ${dependency.path}`,
				color: 'orange'
			})
		})
	}

	$: onClick =
		gridItem?.data.type &&
		['buttoncomponent', 'formbuttoncomponent', 'formcomponent'].includes(gridItem?.data.type)

	$: getBadges(fields)
</script>

<TriggerBadgesList {badges} {onClick} onLoad={!onClick} />
