<script lang="ts" module>
	export type AppSettingsTab = 'metadata' | 'runtime'
</script>

<script lang="ts">
	import { Alert, Drawer, DrawerContent, Tab, TabContent, Tabs } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import Section from '$lib/components/Section.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import Path from '$lib/components/Path.svelte'
	import { useRoleCapabilities } from '$lib/operatorWriteRights'
	import type { Policy } from '$lib/gen'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import LabelsInput from '$lib/components/LabelsInput.svelte'
	import RunOnBehalfOfSetting from '$lib/components/itemSettings/RunOnBehalfOfSetting.svelte'
	import SettingToggle from '$lib/components/itemSettings/SettingToggle.svelte'
	import { FRONTEND_SDK_SCOPES, sdkScopesToConfirm } from '$lib/components/raw_apps/sdkScopes'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspaceStore = useOperatingWorkspace()
	const roleCaps = useRoleCapabilities()
	// The server forces a composed app into sandbox isolation.
	const composeOnlyApps = $derived($roleCaps.apps === 'compose')

	let {
		open = $bindable(),
		tab = $bindable(),
		policy,
		appPath,
		savedApp,
		summary = $bindable(),
		pathError = $bindable(),
		newEditedPath = $bindable(),
		newPath,
		preserveOnBehalfOf = $bindable(),
		labels = $bindable(),
		rawApp = false,
		newApp = false,
		operatingWorkspace = undefined
	}: {
		open: boolean
		tab: AppSettingsTab
		/** Mutated in place; deployed with the app, like every other setting here. */
		policy: Policy
		appPath: string
		savedApp: { path: string; policy: any; custom_path?: string } | undefined
		summary: string
		pathError: string
		newEditedPath: string
		newPath: string
		preserveOnBehalfOf: boolean | undefined
		labels?: string[] | undefined
		// Only raw apps are offered frontend API access, which exists for sandboxed raw apps.
		rawApp?: boolean
		/** True while the editor is on a draft-only URL (`/edit/u/{user}/draft_{uuid}`
		 *  with no deployed row yet). */
		newApp?: boolean
		/** Workspace the app is deployed to. The path and on-behalf-of lookups target it;
		 *  defaults to the operating workspace. */
		operatingWorkspace?: string
	} = $props()

	const opWs = $derived(operatingWorkspace ?? $operatingWorkspaceStore)

	let pathChanged = $derived(
		!newApp && !!savedApp?.path && !!newEditedPath && newEditedPath !== savedApp.path
	)
	let path: Path | undefined = $state(undefined)
	let dirtyPath = $state(false)
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
		<div class="flex flex-col h-full">
			<Tabs bind:selected={tab} wrapperClass="flex-none w-full">
				<Tab value="metadata" label="Metadata" />
				<Tab value="runtime" label="Runtime" />

				{#snippet content()}
					<div class="min-h-0 grow overflow-y-auto">
						<TabContent value="metadata">
							<div class="flex flex-col gap-6 p-4 pb-8">
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
								<LabelsInput bind:labels class="-mt-4" />
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
									{#if pathChanged}
										<Alert
											type="info"
											size="xs"
											title="Deploy the app to make the path change effective."
											class="mt-2"
										/>
									{/if}
								</Label>
							</div>
						</TabContent>

						<TabContent value="runtime">
							<div class="flex flex-col gap-8 p-4 pb-8">
								<Section label="Run as" class="flex flex-col gap-6">
									<RunOnBehalfOfSetting
										kind="app"
										required
										tooltip={`The app's runnables run as this user whoever opens the app, so viewers need no access of their own. To keep that safe, the app can call only the runnables it was deployed with, and viewers cannot override their static inputs. Runnables still get the viewer's email in WM_END_USER_EMAIL${rawApp ? ', and inline ones can be set to read data tables and object storage as the viewer' : ''}.`}
										bind:email={policy.on_behalf_of_email}
										bind:permissionedAs={policy.on_behalf_of}
										bind:preserve={preserveOnBehalfOf}
										savedEmail={savedApp?.policy?.on_behalf_of_email}
										savedPermissionedAs={savedApp?.policy?.on_behalf_of}
										targetWorkspace={opWs ?? ''}
										isNew={newApp}
									/>
								</Section>

								<Section label="Isolation" class="flex flex-col gap-6">
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
													Let the app's frontend call the Windmill API as <b>the viewer</b> through
													the
													<code>windmill-client</code> SDK, which it lists as a dependency. Grant only
													what the app needs.
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
																	const ordered = FRONTEND_SDK_SCOPES.map((s) => s.value).filter(
																		(s) => next.includes(s)
																	)
																	policy.frontend_sdk_scopes =
																		ordered.length > 0 ? ordered : undefined
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
															versions cannot do. An app bundled before this Windmill version fails with
															a CORS error until you deploy it again, which re-bundles it against a current
															client.
														</Alert>
													</div>
												{/if}
											</Label>
										{/if}
									</SettingToggle>
								</Section>
							</div>
						</TabContent>
					</div>
				{/snippet}
			</Tabs>
		</div>
	</DrawerContent>
</Drawer>
