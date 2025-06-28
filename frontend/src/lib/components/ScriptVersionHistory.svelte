<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { classNames, emptyString } from '$lib/utils'
	import { ScriptService, type ScriptHistory } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Skeleton } from '$lib/components/common'
	import FlowModuleScript from './flows/content/FlowModuleScript.svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from './common/button/Button.svelte'
	import { ExternalLink, Pencil, ArrowRight, X, Diff, Code } from 'lucide-svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'

	const dispatch = createEventDispatcher()

	let { openDetails = false, scriptPath }: { openDetails?: boolean; scriptPath: string } = $props()

	let deploymentMsgUpdateMode = $state(false)
	let deploymentMsgUpdate: string | undefined = $state()

	let selectedVersion: ScriptHistory | undefined = $state()
	let selectedVersionIndex: number | undefined = $state()
	let versions: ScriptHistory[] | undefined = $state()
	let loading: boolean = $state(false)

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

	let showDiff: boolean = $state(false)
	let previousHash: string | undefined = $state()
</script>

<Splitpanes class="!overflow-visible">
	<Pane size={20}>
		<div class="flex flex-col gap-2 px-2 pt-2 w-full">
			{#if !loading}
				{#if versions && versions.length > 0}
					<div class="flex gap-2 flex-col">
						{#each versions ?? [] as version, versionIndex}
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class={classNames(
									'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer ',
									selectedVersion?.script_hash == version.script_hash ? 'bg-surface-selected' : '',
									'hover:bg-surface-hover'
								)}
								onclick={() => {
									selectedVersion = version
									selectedVersionIndex = versionIndex

									if (showDiff && versions && selectedVersionIndex === versions.length - 1) {
										showDiff = false
									}

									const availableVersions = versions?.slice(selectedVersionIndex + 1)

									if (
										previousHash &&
										!availableVersions?.find((v) => v.script_hash === previousHash)
									) {
										previousHash = availableVersions?.[0]?.script_hash
									}

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
	</Pane>
	<Pane size={80}>
		<div class="h-full w-full overflow-auto">
			{#if selectedVersion}
				{#key selectedVersion}
					<div class="flex flex-col min-h-full">
						<span class="flex flex-row text-sm p-2 text-tertiary">
							{#if deploymentMsgUpdateMode}
								<div class="flex w-full">
									<input
										type="text"
										bind:value={deploymentMsgUpdate}
										class="!w-auto grow"
										onclick={(e) => {
											e.stopPropagation()
										}}
										onkeydown={(e) => {
											e.stopPropagation()
										}}
										onkeypress={(e) => {
											e.stopPropagation()
											if (e.key === 'Enter') updateDeploymentMsg(selectedVersion?.script_hash)
										}}
									/>
									<Button
										size="xs"
										color="blue"
										buttonType="button"
										btnClasses="!p-1 !w-[34px] !ml-1"
										aria-label="Save deployment message"
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
									onclick={() => {
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

						{#if selectedVersionIndex !== undefined && versions?.slice(selectedVersionIndex + 1).length}
							<div class="p-2 flex flex-row items-center gap-2 h-8">
								<div class="w-min">
									<ToggleButtonGroup
										selected={showDiff ? 'diff' : 'code'}
										on:selected={({ detail }) => {
											showDiff = detail === 'diff'
										}}
									>
										{#snippet children({ item })}
											<ToggleButton light small value="code" label="Code" icon={Code} {item} />
											<ToggleButton light small value="diff" label="Diff" icon={Diff} {item} />
										{/snippet}
									</ToggleButtonGroup>
								</div>

								{#if showDiff}
									<div class="text-xs">Versions:</div>
									<select bind:value={previousHash} class="!text-xs !w-40">
										{#each versions?.slice(selectedVersionIndex + 1) ?? [] as version}
											<option
												value={version.script_hash}
												selected={version.script_hash === selectedVersion.script_hash}
												class="!text-xs"
											>
												{version.deployment_msg ?? version.script_hash}
											</option>
										{/each}
									</select>
								{/if}
							</div>
						{:else}
							<div class="p-2 text-xs text-secondary"> No previous version found </div>
						{/if}
						<FlowModuleScript
							showDate
							path={scriptPath}
							hash={selectedVersion.script_hash}
							{previousHash}
							{showDiff}
						/>
					</div>
				{/key}
			{:else}
				<div class="text-sm p-2 text-tertiary">Select a deployment version to see its details</div>
			{/if}
		</div>
	</Pane>
</Splitpanes>
