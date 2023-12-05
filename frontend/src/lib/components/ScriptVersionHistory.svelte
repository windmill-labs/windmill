<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import PanelSection from './apps/editor/settingsPanel/common/PanelSection.svelte'
	import { classNames, emptyString } from '$lib/utils'
	import { ScriptService, type ScriptHistory } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Skeleton } from '$lib/components/common'
	import FlowModuleScript from './flows/content/FlowModuleScript.svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from './common/button/Button.svelte'
	import { ExternalLink, Pencil, ArrowRight, X } from 'lucide-svelte'

	const dispatch = createEventDispatcher()

	export let openDetails: boolean = false
	export let scriptPath: string

	let deploymentMsgUpdateMode = false
	let deploymentMsgUpdate: string | undefined = undefined

	let selectedVersion: ScriptHistory | undefined = undefined
	let versions: ScriptHistory[] | undefined = undefined
	let loading: boolean = false

	async function loadVersions() {
		loading = true
		versions = await ScriptService.getScriptHistoryByPath({
			workspace: $workspaceStore!,
			path: scriptPath
		})
		loading = false
	}

	async function updateDeploymentMsg(scriptHash: string | undefined) {
		if (
			selectedVersion === undefined ||
			scriptHash === undefined ||
			emptyString(deploymentMsgUpdate)
		) {
			return
		}
		await ScriptService.updateScriptHistory({
			workspace: $workspaceStore!,
			path: scriptPath,
			hash: scriptHash,
			requestBody: {
				deployment_msg: deploymentMsgUpdate!
			}
		})
		selectedVersion.deployment_msg = deploymentMsgUpdate
		deploymentMsgUpdateMode = false
		loadVersions()
	}

	loadVersions()
</script>

<Splitpanes class="!overflow-visible">
	<Pane size={20}>
		<PanelSection title="Past Versions">
			<div class="flex flex-col gap-2 w-full">
				{#if !loading}
					{#if versions && versions.length > 0}
						<div class="flex gap-2 flex-col">
							{#each versions ?? [] as version}
								<!-- svelte-ignore a11y-click-events-have-key-events -->
								<div
									class={classNames(
										'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
										selectedVersion?.script_hash == version.script_hash
											? 'bg-blue-100 text-blue-600'
											: ''
									)}
									on:click={() => {
										selectedVersion = version
										deploymentMsgUpdate = undefined
										deploymentMsgUpdateMode = false
									}}
								>
									<span class="text-xs truncate">
										{#if emptyString(version.deployment_msg)}Version {version.script_hash}{:else}{version.deployment_msg}{/if}
									</span>
									{#if openDetails}
										<Button
											on:click={() => {
												dispatch('openDetails', { version: version.script_hash })
											}}
											class="ml-2 inline-flex gap-1 text-xs items-center"
											size="xs"
											color="light"
											variant="border"
										>
											Run page<ExternalLink size={14} />
										</Button>
									{/if}
								</div>
							{/each}
						</div>
					{:else}
						<div class="text-sm text-tertiary">No items</div>
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
					<div class="flex flex-col">
						<span class="flex flex-row text-sm p-2 text-tertiary">
							{#if deploymentMsgUpdateMode}
								<div class="flex w-full">
									<input
										type="text"
										bind:value={deploymentMsgUpdate}
										class="!w-auto grow"
										on:click|stopPropagation={() => {}}
										on:keydown|stopPropagation
										on:keypress|stopPropagation={({ key }) => {
											if (key === 'Enter') updateDeploymentMsg(selectedVersion?.script_hash)
										}}
									/>
									<Button
										size="xs"
										color="blue"
										buttonType="button"
										btnClasses="!p-1 !w-[34px] !ml-1"
										aria-label="Save Deployment Message"
										on:click={() => {
											updateDeploymentMsg(selectedVersion?.script_hash)
										}}
									>
										<ArrowRight size={14} />
									</Button>
									<Button
										size="xs"
										color="light"
										buttonType="button"
										btnClasses="!p-1 !w-[34px] !ml-1"
										aria-label="Abort"
										on:click={() => {
											deploymentMsgUpdateMode = false
											deploymentMsgUpdate = undefined
										}}
									>
										<X size={14} />
									</Button>
								</div>
							{:else}
								{#if selectedVersion.deployment_msg}
									{selectedVersion.deployment_msg}
								{:else}
									Version {selectedVersion.script_hash}
								{/if}
								<button
									on:click={() => {
										deploymentMsgUpdate = selectedVersion?.deployment_msg
										deploymentMsgUpdateMode = true
									}}
									title="Update commit message"
									class="flex items-center px-1 rounded-sm hover:text-primary text-secondary h-5"
									aria-label="Update commit message"
								>
									<Pencil size={14} />
								</button>
							{/if}
						</span>
						<FlowModuleScript path={scriptPath} hash={selectedVersion.script_hash} />
					</div>
				{/key}
			{:else}
				<div class="text-sm p-2 text-tertiary">Select a deployment version to see its details</div>
			{/if}
		</div>
	</Pane>
</Splitpanes>
