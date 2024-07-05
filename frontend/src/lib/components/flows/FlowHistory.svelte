<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import { classNames, displayDate, emptyString, sendUserToast } from '$lib/utils'
	import { type Flow, FlowService, type FlowVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Skeleton } from '$lib/components/common'
	import FlowViewer from '../FlowViewer.svelte'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Button from '../common/button/Button.svelte'
	import { ArrowRight, Pencil, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let path: string
	let drawer: Drawer
	let loading: boolean = false

	let versions: FlowVersion[] = []

	let selectedVersion: FlowVersion | undefined = undefined
	let selected: Flow | undefined = undefined
	let deploymentMsgUpdateMode = false
	let deploymentMsgUpdate: string | undefined = undefined

	export function open() {
		loadVersions()
		drawer.openDrawer()
	}

	async function loadFlow(version: number) {
		selected = await FlowService.getFlowVersion({
			workspace: $workspaceStore!,
			version,
			path
		})
	}

	async function loadVersions() {
		loading = true
		versions = await FlowService.getFlowHistory({
			workspace: $workspaceStore!,
			path: path
		})
		loading = false
	}

	async function updateDeploymentMsg(version: number | undefined) {
		if (
			selectedVersion === undefined ||
			version === undefined ||
			emptyString(deploymentMsgUpdate)
		) {
			return
		}
		await FlowService.updateFlowHistory({
			workspace: $workspaceStore!,
			version,
			path,
			requestBody: {
				deployment_msg: deploymentMsgUpdate!
			}
		})
		selectedVersion.deployment_msg = deploymentMsgUpdate
		deploymentMsgUpdateMode = false
		loadVersions()
	}

	const dispatch = createEventDispatcher()

	async function restoreVersion(flow: Flow | undefined) {
		if (!flow) return
		await FlowService.updateFlow({
			workspace: $workspaceStore!,
			requestBody: {
				...flow,
				path
			},
			path
		})
		dispatch('historyRestore')
		drawer?.closeDrawer()
		sendUserToast('Flow restored from previous deployment')
	}

	$: selectedVersion !== undefined && loadFlow(selectedVersion.id)
</script>

<Drawer bind:this={drawer} size="1200px">
	<DrawerContent
		on:close={() => {
			drawer?.closeDrawer()
		}}
	>
		<Splitpanes class="!overflow-visible">
			<Pane size={20}>
				<PanelSection title="Past Deployments">
					<div class="flex flex-col gap-2 w-full">
						{#if !loading}
							{#if versions.length > 0}
								<div class="flex gap-2 flex-col">
									{#each versions ?? [] as version}
										<!-- svelte-ignore a11y-click-events-have-key-events -->
										<div
											class={classNames(
												'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
												selectedVersion?.id == version.id ? 'bg-blue-100 text-blue-600' : ''
											)}
											role="button"
											tabindex="0"
											on:click={() => {
												selectedVersion = version
											}}
										>
											<span class="text-xs truncate">
												{#if emptyString(version.deployment_msg)}Version {version.id}{:else}{version.deployment_msg}{/if}
											</span>
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
						{#if selected}
							<div class="px-2 flex flex-col gap-2">
								<span class="flex flex-row text-sm p-1 text-tertiary">
									{#if deploymentMsgUpdateMode}
										<div class="flex w-full">
											<input
												type="text"
												bind:value={deploymentMsgUpdate}
												class="!w-auto grow"
												on:click|stopPropagation={() => {}}
												on:keydown|stopPropagation
												on:keypress|stopPropagation={({ key }) => {
													if (key === 'Enter') updateDeploymentMsg(selectedVersion?.id)
												}}
											/>
											<Button
												size="xs"
												color="blue"
												buttonType="button"
												btnClasses="!p-1 !w-[34px] !ml-1"
												aria-label="Save Deployment Message"
												on:click={() => {
													updateDeploymentMsg(selectedVersion?.id)
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
											Deployed {displayDate(selected.edited_at)} by {selected.edited_by}
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
								<div class="flex p-1 gap-2">
									<Button
										size="xs"
										on:click={() =>
											window.open(`/flows/add?template_id=${selectedVersion?.id}&template=${path}`)}
									>
										Restore as fork
									</Button>
									<Button size="xs" on:click={() => restoreVersion(selected)}
										>Redeploy with that version
									</Button>
								</div>
								<FlowViewer flow={selected} />
							</div>
						{:else}
							<Skeleton layout={[[40]]} />
						{/if}
					{:else}
						<div class="text-sm p-2 text-tertiary"
							>Select a deployment version to see its details</div
						>
					{/if}
				</div>
			</Pane>
		</Splitpanes>
	</DrawerContent>
</Drawer>
