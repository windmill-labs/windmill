<script lang="ts">
	import { Loader2, RefreshCw, TimerReset } from 'lucide-svelte'
	import Button from '../../common/button/Button.svelte'
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { classNames } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { createEventDispatcher } from 'svelte'

	export let componentNumber: number = 0
	export let interval: number | undefined = undefined
	export let refreshing: string[] = []
	export let progress: number = 100
	export let loading: boolean | undefined = false

	const dispatch = createEventDispatcher()

	let items = [
		{
			displayName: 'Once',
			action: () => dispatch('setInter', undefined)
		},
		...[1, 2, 3, 4, 5, 6].map((i) => ({
			displayName: `Every ${i * 5} seconds`,
			action: () => dispatch('setInter', i * 5000)
		}))
	]
</script>

<div class="border rounded-md overflow-hidden">
	<div class={twMerge('flex items-center')}>
		<Button
			disabled={componentNumber == 0}
			on:click
			color="light"
			size="xs"
			variant="border"
			btnClasses={twMerge(
				'!rounded-none text-tertiary !text-2xs !border-r border-y-0 border-l-0 group'
			)}
			title="Refresh {componentNumber} component{componentNumber > 1 ? 's' : ''} {interval
				? `every ${interval / 1000} seconds`
				: 'Once'} {refreshing?.length > 0 ? `(live: ${refreshing?.join(', ')}))` : ''}"
		>
			<div class="z-10 flex flex-row items-center gap-2">
				{#if !loading}
					<RefreshCw size={14} />
				{:else}
					<Loader2 class="animate-spin text-blue-500" size={14} />
				{/if}

				({componentNumber})
			</div>
		</Button>

		<ButtonDropdown hasPadding={false}>
			<slot:fragment slot="buttonReplacement">
				<div class="flex flex-row gap-2 text-xs hover:bg-surface-hover px-2 items-center h-7">
					{#if interval}
						<Badge color="blue" small>
							{interval ? `Every ${interval / 1000}s` : 'Once'}
						</Badge>
					{/if}

					<div class="flex justify-center items-center">
						<TimerReset size={14} />
					</div>
				</div>
			</slot:fragment>
			<svelte:fragment slot="label">
				<span
					class={twMerge('text-xs min-w-[2rem] ', interval ? 'text-blue-500' : 'text-tertiary')}
				>
					{interval ? `${interval / 1000}s` : 'Once'}
				</span>
			</svelte:fragment>
			<svelte:fragment slot="items">
				{#each items ?? [] as { }, index}
					<MenuItem
						on:click={() => {
							if (index === 0) {
								dispatch('setInter', undefined)
							} else {
								dispatch('setInter', index * 5000)
							}
						}}
					>
						<div
							class={classNames(
								'!text-tertiary text-left px-4 py-2 gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold'
							)}
						>
							{#if index === 0}
								Once
							{:else}
								{`Every ${index * 5} seconds`}
							{/if}
						</div>
					</MenuItem>
				{/each}
			</svelte:fragment>
		</ButtonDropdown>
	</div>
	{#if interval}
		<div class="w-full bg-gray-200 rounded-full h-0.5 dark:bg-gray-700">
			<div
				class="bg-blue-300 h-0.5 rounded-full dark:bg-blue-500 transition-all"
				style="width: {progress}%"
			/>
		</div>
	{/if}
</div>
