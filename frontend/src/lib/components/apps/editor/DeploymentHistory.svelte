<script lang="ts">
	import { stopPropagation, createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import PanelSection from './settingsPanel/common/PanelSection.svelte'
	import { classNames, displayDate, emptyString } from '$lib/utils'
	import { AppService, type AppWithLastVersion, type AppHistory } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Skeleton } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { createEventDispatcher, untrack } from 'svelte'
	import { Pencil, ArrowRight, X, Loader2 } from 'lucide-svelte'

	interface Props {
		appPath: string | undefined
	}

	let { appPath }: Props = $props()
	let loading: boolean = $state(false)

	let versions: AppHistory[] = $state([])

	let selectedVersion: AppHistory | undefined = $state(undefined)
	let selected: (AppWithLastVersion & { value: any }) | undefined = $state(undefined)

	let deploymentMsgUpdateMode = $state(false)
	let deploymentMsgUpdate: string | undefined = $state(undefined)

	async function loadVersions() {
		if (appPath === undefined) {
			return
		}

		loading = true
		versions = await AppService.getAppHistoryByPath({
			workspace: $workspaceStore!,
			path: appPath
		})
		loading = false
	}

	async function loadValue(version: number) {
		let app = await AppService.getAppByVersion({ workspace: $workspaceStore!, id: version })

		selected = app
	}

	async function updateDeploymentMsg(appId: number | undefined, appVersion: number | undefined) {
		if (
			selectedVersion === undefined ||
			appId === undefined ||
			appVersion === undefined ||
			emptyString(deploymentMsgUpdate)
		) {
			return
		}
		await AppService.updateAppHistory({
			workspace: $workspaceStore!,
			id: appId,
			version: appVersion,
			requestBody: {
				deployment_msg: deploymentMsgUpdate!
			}
		})
		selectedVersion.deployment_msg = deploymentMsgUpdate
		deploymentMsgUpdateMode = false
		loadVersions()
	}

	const dispatch = createEventDispatcher()
	loadVersions()
	$effect(() => {
		selectedVersion?.version !== undefined &&
			untrack(() => {
				selectedVersion && loadValue(selectedVersion.version!)
			})
	})
</script>

<Splitpanes class="!overflow-visible">
	<Pane size={20}>
		<PanelSection title="Past Deployments">
			<div class="flex flex-col gap-2 w-full">
				{#if !loading}
					{#if versions.length > 0}
						<div class="flex gap-2 flex-col">
							{#each versions ?? [] as version}
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class={classNames(
										'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
										selectedVersion?.version == version.version ? 'bg-blue-100 text-blue-600' : ''
									)}
									onclick={() => {
										selectedVersion = version
										deploymentMsgUpdateMode = false
										deploymentMsgUpdate = undefined
									}}
								>
									<span class="text-xs truncate">
										{#if emptyString(version.deployment_msg)}Version {version.version}{:else}{version.deployment_msg}{/if}
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
					<div class="flex flex-col justify-between">
						<span class="flex flex-row text-sm p-1 text-tertiary">
							{#if deploymentMsgUpdateMode}
								<div class="flex w-full">
									<input
										type="text"
										bind:value={deploymentMsgUpdate}
										class="!w-auto grow"
										onclick={stopPropagation(() => {})}
										onkeydown={stopPropagation(bubble('keydown'))}
										onkeypress={(e) => {
											e.stopPropagation()
											if (e.key === 'Enter')
												updateDeploymentMsg(selected?.id, selectedVersion?.version)
										}}
									/>
									<Button
										size="xs"
										color="blue"
										buttonType="button"
										btnClasses="!p-1 !w-[34px] !ml-1"
										aria-label="Save deployment message"
										on:click={() => {
											updateDeploymentMsg(selected?.id, selectedVersion?.version)
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
									Deployed {displayDate(selected.created_at)} by {selected.created_by}
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
						<div class="flex p-1 gap-2">
							<Button
								size="xs"
								on:click={() => window.open(`/apps/add?template_id=${selectedVersion?.version}`)}
							>
								Restore as fork
							</Button>
							<Button size="xs" on:click={() => dispatch('restore', selected)}
								>Redeploy with that version
							</Button>
						</div>
					</div>

					{#await import('$lib/components/apps/editor/AppPreview.svelte')}
						<Loader2 class="animate-spin" />
					{:then Module}
						<Module.default noBackend app={selected.value} context={{}} />
					{/await}
				{:else}
					<Skeleton layout={[[40]]} />
				{/if}
			{:else}
				<div class="text-sm p-2 text-tertiary">Select a deployment version to see its details</div>
			{/if}
		</div>
	</Pane>
</Splitpanes>
