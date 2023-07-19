<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import PanelSection from './settingsPanel/common/PanelSection.svelte'
	import { classNames, displayDate } from '$lib/utils'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import AppPreview from './AppPreview.svelte'
	import { Skeleton } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { createEventDispatcher } from 'svelte'

	export let versions: number[]

	let selectedVersion: number | undefined = undefined
	let selected: AppWithLastVersion | undefined = undefined

	$: selectedVersion !== undefined && loadValue(selectedVersion)

	async function loadValue(version: number) {
		let app = await AppService.getAppByVersion({ workspace: $workspaceStore!, id: version })

		selected = app
	}

	$: reversedVersions = versions.slice().reverse()

	const dispatch = createEventDispatcher()
</script>

<Splitpanes class="!overflow-visible">
	<Pane size={20}>
		<PanelSection title="Past Deployments">
			<div class="flex flex-col gap-2 w-full">
				{#if versions.length > 0}
					<div class="flex gap-2 flex-col">
						{#each reversedVersions ?? [] as version}
							<!-- svelte-ignore a11y-click-events-have-key-events -->
							<div
								class={classNames(
									'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
									selectedVersion == version ? 'bg-blue-100 text-blue-600' : ''
								)}
								on:click={() => (selectedVersion = version)}
							>
								<span class="text-xs truncate">{version}</span>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-sm text-gray-500">No items</div>
				{/if}
			</div>
		</PanelSection>
	</Pane>
	<Pane size={80}>
		<div class="h-full w-full overflow-auto">
			{#if selectedVersion}
				{#if selected}
					<div class="flex justify-between">
						<h3 class="p-1">Deployed {displayDate(selected.created_at)} by {selected.created_by}</h3
						>
						<div class="flex gap-2">
							<Button on:click={() => window.open(`/apps/add?template_id=${selectedVersion}`)}
								>Restore as fork</Button
							>
							<Button on:click={() => dispatch('restore', selected)}>Restore here</Button>
						</div>
					</div>
					<AppPreview noBackend app={selected.value} context={{}} />
				{:else}
					<Skeleton layout={[[40]]} />
				{/if}
			{:else}
				<div class="text-sm p-2 text-gray-500">Select a deployment version to see its details</div>
			{/if}
		</div>
	</Pane>
</Splitpanes>
