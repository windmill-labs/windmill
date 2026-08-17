<script lang="ts">
	import { Folder, User, Circle } from 'lucide-svelte'
	import { appIconComponent } from '../icons'
	import { createEventDispatcher } from 'svelte'
	import { Button } from '../common'

	interface Props {
		filters: string[]
		selectedFilter?: { kind: 'owner' | 'integrations'; name: string | undefined } | undefined
		resourceType?: boolean
	}

	let { filters, selectedFilter = $bindable(undefined), resourceType = false }: Props = $props()

	function getIconComponent(name: string, resourceType: boolean) {
		if (resourceType) {
			const icon = appIconComponent(name)
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
	let selectedAppFilter = $derived(
		selectedFilter?.kind === 'integrations' ? selectedFilter?.name : undefined
	)
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
				title={filter}
			>
				<span class="truncate">{filter}</span>
			</Button>
		</div>
	{/each}
{/if}
