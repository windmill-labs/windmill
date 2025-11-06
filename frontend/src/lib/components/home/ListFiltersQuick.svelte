<script lang="ts">
	import { Folder, User, Circle } from 'lucide-svelte'
	import { APP_TO_ICON_COMPONENT } from '../icons'
	import { createEventDispatcher } from 'svelte'
	import { Button } from '../common'

	export let filters: string[]
	export let selectedFilter:
		| { kind: 'owner' | 'integrations'; name: string | undefined }
		| undefined = undefined
	$: selectedAppFilter = selectedFilter?.kind === 'integrations' ? selectedFilter?.name : undefined

	export let resourceType = false

	function getIconComponent(name: string, resourceType: boolean) {
		if (resourceType) {
			const icon = APP_TO_ICON_COMPONENT[name] || APP_TO_ICON_COMPONENT[name.split('_')[0]]
			if (icon) {
				return { icon: icon, props: { width: 13, height: 13 } }
			} else {
				return { icon: Circle, props: { class: 'text-gray-400' } }
			}
		} else if (name.startsWith('u/')) {
			return { icon: User }
		} else if (name.startsWith('f/')) {
			return { icon: Folder }
		}
		return { icon: undefined }
	}

	const dispatch = createEventDispatcher()
</script>

{#if Array.isArray(filters) && filters.length > 0}
	{#each filters as filter (filter)}
		{@const icon = getIconComponent(filter, resourceType)}
		<div>
			<Button
				selected={filter === selectedAppFilter}
				onClick={() => {
					selectedFilter =
						selectedAppFilter == filter ? undefined : { kind: 'integrations', name: filter }
					dispatch('selected')
				}}
				variant="subtle"
				startIcon={icon}
				unifiedSize="sm"
				btnClasses="justify-start"
			>
				{filter}
			</Button>
		</div>
	{/each}
{/if}
