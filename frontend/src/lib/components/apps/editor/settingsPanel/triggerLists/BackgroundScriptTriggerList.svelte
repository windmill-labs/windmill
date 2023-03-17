<script lang="ts">
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '$lib/components/apps/inputType'

	import TriggerBadgesList from './TriggerBadgesList.svelte'
	import { getDependencies, type DependencyBadge } from './triggerListUtils'

	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let autoRefresh: boolean = false
	export let id: string
	export let refreshOn: { id: string; key: string }[] | undefined = undefined

	const badges: DependencyBadge = []
	const dependencies = getDependencies(fields)

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

<TriggerBadgesList valuesChangeBadges={badges} {id} onLoad={autoRefresh} />
