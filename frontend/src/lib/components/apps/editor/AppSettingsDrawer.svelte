<script lang="ts">
	import { Alert, Drawer, DrawerContent, Section } from '$lib/components/common'
	import { tick } from 'svelte'
	import { revealSetting } from '$lib/components/itemSettings/ActiveSettingChips.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Path from '$lib/components/Path.svelte'
	import { useRoleCapabilities } from '$lib/operatorWriteRights'
	import type { Policy } from '$lib/gen'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import LabelsInput from '$lib/components/LabelsInput.svelte'
	import RunOnBehalfOfSetting from '$lib/components/itemSettings/RunOnBehalfOfSetting.svelte'
	import { appOnBehalfOfTooltip } from '$lib/components/itemSettings/onBehalfOf'
	import SettingToggle from '$lib/components/itemSettings/SettingToggle.svelte'
	import SettingIcons from '$lib/components/itemSettings/SettingIcons.svelte'
	import { getAppMetadataBadges, getAppPermissionBadges } from './appSettingsBadges'
	import PathMoveNote from '$lib/components/itemSettings/PathMoveNote.svelte'
	import { FRONTEND_SDK_SCOPES, sdkScopesToConfirm } from '$lib/components/raw_apps/sdkScopes'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import AppAccessSection from '$lib/components/itemSettings/AppAccessSection.svelte'
	import DeployedAppAccess from '$lib/components/itemSettings/DeployedAppAccess.svelte'
	import { accessModeOf } from '$lib/components/itemSettings/appAccess'
	import { AppService, type AppWithLastVersion } from '$lib/gen'

	const operatingWorkspaceStore = useOperatingWorkspace()
	const roleCaps = useRoleCapabilities()
	// The server forces a composed app into sandbox isolation.
	const composeOnlyApps = $derived($roleCaps.apps === 'compose')

	let {
		open = $bindable(),
		policy = $bindable(),
		appPath,
		savedApp,
		summary = $bindable(),
		pathError = $bindable(),
		newEditedPath = $bindable(),
		newPath,
		labels = $bindable(),
		rawApp = false,
		newApp = false,
		operatingWorkspace = undefined,
		customPath = $bindable(),
		onAppChanged
	}: {
		open: boolean
		/** Mutated in place; deployed with the app, like every other setting here. */
		policy: Policy
		appPath: string
		savedApp: { path: string; policy: any; custom_path?: string } | undefined
		summary: string
		pathError: string
		newEditedPath: string
		newPath: string
		labels?: string[] | undefined
		// Only raw apps are offered frontend API access, which exists for sandboxed raw apps.
		rawApp?: boolean
		/** True while the editor is on a draft-only URL (`/edit/u/{user}/draft_{uuid}`
		 *  with no deployed row yet). */
		newApp?: boolean
		/** Workspace the app is deployed to. The path and on-behalf-of lookups target it;
		 *  defaults to the operating workspace. */
		operatingWorkspace?: string
		/** Taken by the first deploy. A deployed app's custom URL is written at once instead. */
		customPath?: string | undefined
		/** Called after a write to the deployed app row, with the row as it now is. */
		onAppChanged?: (row: AppWithLastVersion) => void
	} = $props()

	let body: HTMLElement | undefined = $state()

	/** Opens the drawer scrolled to the row whose `data-setting` is `setting`, or at the top. */
	export async function openAt(setting?: string) {
		open = true
		if (!setting) return
		await tick()
		revealSetting(body, setting)
	}

	const opWs = $derived(operatingWorkspace ?? $operatingWorkspaceStore)

	let path: Path | undefined = $state(undefined)
	let dirtyPath = $state(false)

	// Who can open a deployed app is not deployed with it: it is read from, and written to, the
	// deployed row, as in Share.
	let deployedPath = $derived(newApp ? undefined : savedApp?.path)
	let deployedApp: AppWithLastVersion | undefined = $state()
	async function loadDeployedApp() {
		const asked = deployedPath
		if (!asked || !opWs) return
		const row = await AppService.getAppLiteByPath({ workspace: opWs, path: asked })
		if (deployedPath === asked) deployedApp = row
	}
	$effect(() => {
		if (open && deployedPath) {
			deployedApp = undefined
			loadDeployedApp().catch(() => {})
		}
	})
</script>

{#snippet sandboxHint()}
	{#if composeOnlyApps}
		Required for apps composed by your role, and cannot be turned off.
	{:else}
		<Alert type="warning" title="Not isolated" size="xs">
			The app's frontend code runs with each viewer's full Windmill permissions. A future version
			will ask viewers to approve this. Keep isolation off only if the app needs it.
		</Alert>
	{/if}
{/snippet}

<Drawer bind:open size="800px">
	<DrawerContent noPadding title="Settings" on:close={() => (open = false)}>
		<div bind:this={body} class="h-full overflow-y-auto">
			<div class="flex flex-col gap-10 p-4 pb-8">
				<Section label="Metadata">
					{#snippet labelExtra()}
						<SettingIcons badges={getAppMetadataBadges(labels)} />
					{/snippet}
					<div class="flex flex-col gap-6">
						<Label label="Summary" for="app-summary">
							<!-- svelte-ignore a11y_autofocus -->
							<TextInput
								inputProps={{
									id: 'app-summary',
									autofocus: true,
									placeholder: 'Short summary to be displayed when listed',
									onkeydown: (e) => {
										e.stopPropagation()
									},
									onkeyup: () => {
										if (appPath == '' && summary?.length > 0 && !dirtyPath) {
											path?.setName(
												summary
													.toLowerCase()
													.replace(/[^a-z0-9_]/g, '_')
													.replace(/-+/g, '_')
													.replace(/^-|-$/g, '')
											)
										}
									}
								}}
								bind:value={summary}
							/>
						</Label>
						<div data-setting="labels" class="-mt-4"><LabelsInput bind:labels /></div>
						<Label label="Path">
							{#snippet header()}
								<Tooltip
									documentationLink="https://www.windmill.dev/docs/core_concepts/roles_and_permissions#path"
								>
									The unique identifier of the app in the workspace that defines permissions
								</Tooltip>
							{/snippet}
							<Path
								bind:this={path}
								bind:dirty={dirtyPath}
								bind:error={pathError}
								bind:path={newEditedPath}
								initialPath={newPath}
								namePlaceholder="app"
								kind="app"
								autofocus={false}
								workspaceOverride={operatingWorkspace}
							/>
							<PathMoveNote
								kind="app"
								deployedPath={newApp ? undefined : savedApp?.path}
								path={newEditedPath}
							/>
						</Label>
					</div>
				</Section>

				<Section label="Who can open the app">
					<div class="flex flex-col gap-2">
						{#if deployedPath}
							{#if deployedApp}
								<span class="text-xs text-secondary">
									Changes here apply to the deployed app right away.
								</span>
								<DeployedAppAccess
									workspace={opWs ?? ''}
									bind:app={deployedApp}
									reload={loadDeployedApp}
									{onAppChanged}
								/>
							{:else}
								<span class="text-xs text-secondary">Loading…</span>
							{/if}
						{:else}
							<AppAccessSection
								workspace={opWs ?? ''}
								appPath={newEditedPath}
								{rawApp}
								mode={accessModeOf(policy)}
								onModeChange={(m) => {
									policy.execution_mode = m
								}}
								{customPath}
								onCustomPathChange={(p) => {
									customPath = p ?? undefined
								}}
							/>
						{/if}
					</div>
				</Section>

				<Section label="Permissions">
					{#snippet labelExtra()}
						<SettingIcons
							badges={getAppPermissionBadges(policy, composeOnlyApps || policy.sandbox == true)}
						/>
					{/snippet}
					<div class="flex flex-col gap-6">
						<RunOnBehalfOfSetting
							kind="app"
							required
							tooltip={appOnBehalfOfTooltip(rawApp)}
							bind:email={policy.on_behalf_of_email}
							bind:permissionedAs={policy.on_behalf_of}
							savedEmail={savedApp?.policy?.on_behalf_of_email}
							savedPermissionedAs={savedApp?.policy?.on_behalf_of}
							targetWorkspace={opWs ?? ''}
							isNew={newApp}
						/>

						<SettingToggle
							setting="sandbox"
							label="Sandbox isolation"
							tooltip="Controls what the app's browser-side code can reach in each viewer's browser, separately from who its runnables run as. A sandboxed app gets a narrowly scoped token on every surface, public URL and in-workspace. On by default for new apps. Turn it off only if the app needs IndexedDB, third-party sign-in or OAuth redirects."
							checked={composeOnlyApps || policy.sandbox == true}
							disabled={composeOnlyApps}
							hint={composeOnlyApps || policy.sandbox != true ? sandboxHint : undefined}
							onchange={(on) => {
								policy.sandbox = on || undefined
								// Counted where the toggle is flipped rather than where the policy is
								// persisted, so a choice made before the first deploy counts too.
								logFeatureUsage('app_sandbox', 'toggled', {
									key: `${rawApp ? 'raw' : 'low_code'}:${on ? 'on' : 'off'}`
								})
								// Frontend API access exists only for a sandboxed app, so turning
								// isolation off drops the declared scopes with it rather than leaving
								// them set but inert.
								if (!on) {
									policy.frontend_sdk_scopes = undefined
								}
							}}
						>
							{#if rawApp}
								<Label
									label="Frontend API access"
									tooltip="Each viewer approves these scopes before the app runs. The app's code, or an XSS bug in it, can use them as that viewer."
								>
									<div class="text-xs text-secondary">
										Let the app's frontend call the Windmill API as <b>the viewer</b> through the
										<code>windmill-client</code> SDK, which it lists as a dependency. Grant only what
										the app needs.
									</div>
									<div class="flex flex-col gap-2 mt-2">
										{#each FRONTEND_SDK_SCOPES as scope (scope.value)}
											<div>
												<Toggle
													size="xs"
													options={{ right: scope.label }}
													checked={policy.frontend_sdk_scopes?.includes(scope.value) ?? false}
													on:change={(e) => {
														const current: string[] = policy.frontend_sdk_scopes ?? []
														const next = e.detail
															? [...current, scope.value]
															: current.filter((s) => s !== scope.value)
														// Keep the curated order so the consent banner and the stored
														// consent compare stably across deploys. A leftover implicit scope is
														// not in the list, so it drops here rather than keep minting a token
														// for an app whose toggles all read off.
														const ordered = FRONTEND_SDK_SCOPES.map((s) => s.value).filter((s) =>
															next.includes(s)
														)
														policy.frontend_sdk_scopes = ordered.length > 0 ? ordered : undefined
													}}
												/>
												<div class="text-2xs text-secondary pl-9">{scope.description}</div>
											</div>
										{/each}
									</div>
									{#if sdkScopesToConfirm(policy.frontend_sdk_scopes ?? []).length}
										<div class="mt-2">
											<Alert
												type="info"
												title="Redeploy to use the SDK from a sandboxed app"
												size="xs"
											>
												A sandboxed app calls the API cross-origin, which older <code
													>windmill-client</code
												>
												versions cannot do. An app bundled before this Windmill version fails with a
												CORS error until you deploy it again, which re-bundles it against a current client.
											</Alert>
										</div>
									{/if}
								</Label>
							{/if}
						</SettingToggle>
					</div>
				</Section>
			</div>
		</div>
	</DrawerContent>
</Drawer>
