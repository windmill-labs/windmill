<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import PanelSection from './apps/editor/settingsPanel/common/PanelSection.svelte'
	import { classNames } from '$lib/utils'
	import { ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Skeleton } from '$lib/components/common'
	import FlowModuleScript from './flows/content/FlowModuleScript.svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from './common/button/Button.svelte'

	const dispatch = createEventDispatcher()

	export let openDetails: boolean = false
	export let scriptPath: string

	let selectedVersion: string | undefined = undefined
	let versions: string[] | undefined = undefined

	async function loadVersions() {
		versions = (
			await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: scriptPath })
		).parent_hashes
	}

	loadVersions()
</script>

<Splitpanes class="!overflow-visible">
	<Pane size={20}>
		<PanelSection title="Past Versions">
			<div class="flex flex-col gap-2 w-full">
				{#if versions}
					{#if versions.length > 0}
						<div class="flex gap-2 flex-col">
							{#each versions ?? [] as version}
								<!-- svelte-ignore a11y-click-events-have-key-events -->
								<div
									class={classNames(
										'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
										selectedVersion == version ? 'bg-blue-100 text-blue-600' : ''
									)}
									on:click={() => (selectedVersion = version)}
								>
									<span class="text-xs truncate">{version}</span>
									{#if openDetails}
										<Button
											on:click={() => {
												dispatch('openDetails', { version })
											}}
											size="xs2"
											color="light"
											variant="border"
										>
											Open details
										</Button>
									{/if}
								</div>
							{/each}
						</div>
					{:else}
						<div class="text-sm text-gray-500">No items</div>
					{/if}
				{:else}
					<Skeleton layout={[[40], [40], [40], [40], [40]]} />
				{/if}
			</div>
		</PanelSection>
	</Pane>
	<Pane size={80}>
		<div class="h-full w-full overflow-auto">
			{#if selectedVersion}
				{#key selectedVersion}
					<FlowModuleScript path={scriptPath} hash={selectedVersion} />
				{/key}
			{:else}
				<div class="text-sm p-2 text-gray-500">Select a deployment version to see its details</div>
			{/if}
		</div>
	</Pane>
</Splitpanes>
