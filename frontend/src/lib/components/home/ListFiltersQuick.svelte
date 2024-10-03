<script lang="ts">
	import { Folder, User, Circle } from 'lucide-svelte'
	import { APP_TO_ICON_COMPONENT } from '../icons'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher } from 'svelte'

	export let filters: string[]
	export let selectedFilter:
		| { kind: 'owner' | 'integrations'; name: string | undefined }
		| undefined = undefined
	$: selectedAppFilter = selectedFilter?.kind === 'integrations' ? selectedFilter?.name : undefined

	export let resourceType = false

	function getIconComponent(name: string) {
		return APP_TO_ICON_COMPONENT[name] || APP_TO_ICON_COMPONENT[name.split('_')[0]]
	}

	const dispatch = createEventDispatcher()
</script>

{#if Array.isArray(filters) && filters.length > 0}
	{#each filters as filter (filter)}
		<div>
			<button
				class={twMerge(
					'w-full text-left py-2 px-3 hover:bg-surface-hover whitespace-nowrap flex flex-row gap-2 items-center rounded-md',
					filter === selectedAppFilter ? 'bg-surface-hover' : ''
				)}
				on:click={() => {
					selectedFilter =
						selectedAppFilter == filter ? undefined : { kind: 'integrations', name: filter }
					dispatch('selected')
				}}
			>
				<div class="flex justify-center flex-row items-center gap-2">
					{#if resourceType}
						{@const icon = getIconComponent(filter)}
						{#if icon}
							<svelte:component this={icon} height="14px" width="14px" />
						{:else}
							<div
								class="w-[14px] h-[14px] text-gray-400 flex flex-row items-center justify-center"
							>
								<Circle size="12" />
							</div>
						{/if}
					{:else if filter.startsWith('u/')}
						<User class="mr-0.5" size={14} />
					{:else if filter.startsWith('f/')}
						<Folder class="mr-0.5" size={14} />
					{/if}
					<span class="text-left text-2xs text-primary font-normal">{filter}</span>
				</div>
			</button>
		</div>
	{/each}
{/if}
