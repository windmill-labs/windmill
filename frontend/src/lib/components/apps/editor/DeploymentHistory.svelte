<script lang="ts">
	import { stopPropagation, createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import PanelSection from './settingsPanel/common/PanelSection.svelte'
	import {
		classNames,
		cleanValueProperties,
		displayDate,
		emptyString,
		orderedYamlStringify,
		replaceFalseWithUndefined,
		type Value
	} from '$lib/utils'
	import { AppService, type AppWithLastVersion, type AppHistory } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Skeleton } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { createEventDispatcher, untrack } from 'svelte'
	import { Pencil, ArrowRight, X, Loader2 } from 'lucide-svelte'
	import RawAppPreview from '$lib/components/raw_apps/RawAppPreview.svelte'
	import type { Runnable } from '$lib/components/raw_apps/rawAppPolicy'
	import Select from '$lib/components/select/Select.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'

	interface Props {
		appPath: string | undefined
	}

	type HistoryApp = AppWithLastVersion & { value: any }
	type HistoryTab = 'preview' | 'diff'

	let { appPath }: Props = $props()
	let loading: boolean = $state(false)

	let versions: AppHistory[] = $state([])

	let selectedVersion: AppHistory | undefined = $state(undefined)
	let selected: HistoryApp | undefined = $state(undefined)
	let previousVersion: HistoryApp | undefined = $state(undefined)
	let selectedVersionIndex: number | undefined = $state(undefined)
	let previousVersionId: number | undefined = $state(undefined)
	let selectedTab: HistoryTab = $state('preview')
	let versionCache: Record<number, HistoryApp> = $state({})

	let deploymentMsgUpdateMode = $state(false)
	let deploymentMsgUpdate: string | undefined = $state(undefined)

	async function getVersionApp(version: number): Promise<HistoryApp> {
		const cached = versionCache[version]
		if (cached) {
			return cached
		}

		const app = await AppService.getAppByVersion({ workspace: $workspaceStore!, id: version })
		versionCache[version] = app
		return app
	}

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

	async function loadSelectedVersion(version: number) {
		const app = await getVersionApp(version)
		if (selectedVersion?.version === version) {
			selected = app
		}
	}

	async function loadPreviousVersion(version: number) {
		const app = await getVersionApp(version)
		if (previousVersionId === version) {
			previousVersion = app
		}
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

	function toComparableVersionValue(app: HistoryApp): Value {
		return {
			summary: app.summary,
			value: app.value,
			path: app.path,
			policy: app.policy,
			custom_path: app.custom_path
		}
	}

	function toVersionLabel(version: AppHistory): string {
		return emptyString(version.deployment_msg) ? `Version ${version.version}` : version.deployment_msg!
	}

	let availableVersions = $derived(
		selectedVersionIndex !== undefined ? versions.slice(selectedVersionIndex + 1) : []
	)

	let compareVersionItems = $derived(
		availableVersions.map((version) => ({
			label: toVersionLabel(version),
			value: version.version
		}))
	)

	let selectedVersionYaml = $derived.by(() => {
		if (!selected) return undefined
		return orderedYamlStringify(
			replaceFalseWithUndefined(cleanValueProperties(toComparableVersionValue(selected)))
		)
	})

	let previousVersionYaml = $derived.by(() => {
		if (!previousVersion) return undefined
		return orderedYamlStringify(
			replaceFalseWithUndefined(cleanValueProperties(toComparableVersionValue(previousVersion)))
		)
	})

	const dispatch = createEventDispatcher()
	loadVersions()
	$effect(() => {
		selectedVersion?.version !== undefined &&
			untrack(() => {
				selectedVersion && loadSelectedVersion(selectedVersion.version!)
			})
	})
	$effect(() => {
		if (previousVersionId === undefined) {
			previousVersion = undefined
			return
		}

		const version = previousVersionId
		untrack(() => {
			loadPreviousVersion(version)
		})
	})
	$effect(() => {
		if (availableVersions.length === 0) {
			previousVersionId = undefined
			if (selectedTab === 'diff') {
				selectedTab = 'preview'
			}
			return
		}

		if (!availableVersions.some((version) => version.version === previousVersionId)) {
			previousVersionId = availableVersions[0]?.version
		}
	})
</script>

<Splitpanes class="!overflow-visible">
	<Pane size={20}>
		<PanelSection title="Past Deployments">
			<div class="flex flex-col gap-2 w-full">
				{#if !loading}
					{#if versions.length > 0}
						<div class="flex gap-2 flex-col">
							{#each versions ?? [] as version, versionIndex}
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class={classNames(
										'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
										selectedVersion?.version == version.version ? 'bg-blue-100 text-blue-600' : ''
									)}
									onclick={() => {
										selectedVersion = version
										selectedVersionIndex = versionIndex
										previousVersionId = versions[versionIndex + 1]?.version
										deploymentMsgUpdateMode = false
										deploymentMsgUpdate = undefined
									}}
								>
									<span class="text-xs truncate">
										{toVersionLabel(version)}
									</span>
								</div>
							{/each}
						</div>
					{:else}
						<div class="text-sm text-primary">No items</div>
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
					{@const currentSelected = selected}
					<div class="flex flex-col justify-between">
						<span class="flex flex-row text-sm p-1 text-primary">
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
								on:click={() =>
									window.open(
										`${selected?.raw_app ? '/apps_raw' : '/apps'}/add?template_id=${selectedVersion?.version}`
									)}
							>
								Restore as fork
							</Button>
							<Button size="xs" on:click={() => dispatch('restore', selected)}
								>Redeploy with that version
							</Button>
						</div>
					</div>

					<Tabs bind:selected={selectedTab}>
						{#if availableVersions.length > 0}
							<Tab value="diff" label="Diff" />
						{/if}
						<Tab value="preview" label="Preview" />

						{#snippet content()}
							{#if availableVersions.length > 0}
								<TabContent value="diff">
									<div class="flex flex-col gap-2">
										<div class="flex flex-row items-center gap-2 py-2">
											<div class="text-xs">Compare with:</div>
											<Select
												items={compareVersionItems}
												bind:value={previousVersionId}
												class="w-56"
												size="sm"
											/>
										</div>

										<div class="h-[calc(100vh-260px)] min-h-[420px]">
											{#if previousVersionYaml && selectedVersionYaml}
												{#key `${previousVersionId ?? 'none'}:${selectedVersion?.version ?? 'current'}`}
													{#await import('$lib/components/DiffEditor.svelte')}
														<Loader2 class="animate-spin" />
													{:then Module}
														<Module.default
															open={true}
															automaticLayout
															className="h-full"
															defaultLang="yaml"
															defaultOriginal={previousVersionYaml}
															defaultModified={selectedVersionYaml}
															readOnly
														/>
													{/await}
												{/key}
											{:else}
												<Loader2 class="animate-spin" />
											{/if}
										</div>
									</div>
								</TabContent>
							{/if}

							<TabContent value="preview">
								{#if currentSelected.raw_app}
									{#if currentSelected.bundle_secret}
										<RawAppPreview
											workspace={$workspaceStore!}
											user={$userStore}
											secret={currentSelected.bundle_secret}
											path={currentSelected.path}
											runnables={(currentSelected.value?.runnables ?? {}) as Record<string, Runnable>}
										/>
									{:else}
										<div class="text-sm p-2 text-primary">
											This raw app version has no preview bundle.
										</div>
									{/if}
								{:else}
									{#await import('$lib/components/apps/editor/AppPreview.svelte')}
										<Loader2 class="animate-spin" />
									{:then Module}
										<Module.default noBackend app={currentSelected.value} context={{}} />
									{/await}
								{/if}
							</TabContent>
						{/snippet}
					</Tabs>
				{:else}
					<Skeleton layout={[[40]]} />
				{/if}
			{:else}
				<div class="text-sm p-2 text-primary">Select a deployment version to see its details</div>
			{/if}
		</div>
	</Pane>
</Splitpanes>
