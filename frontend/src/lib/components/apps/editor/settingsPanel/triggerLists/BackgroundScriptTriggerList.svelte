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

	const badges: DependencyBadge = []
	const dependencies = getDependencies(fields)

	dependencies.forEach((dependency) => {
		badges.push({
			label: `${dependency.componentId} - ${dependency.path}`,
			color: 'red'
		})
	})
</script>

<TriggerBadgesList {badges} />
