<script lang="ts" module>
	export type AppSettingsTab = 'metadata' | 'permissions' | 'sharing'
</script>

<script lang="ts">
	import { Alert, Drawer, DrawerContent, Tab, TabContent, Tabs } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import { enterpriseLicense, userStore } from '$lib/stores'
	import { useRoleCapabilities } from '$lib/operatorWriteRights'
	import { Loader2 } from 'lucide-svelte'

	import Tooltip from '$lib/components/Tooltip.svelte'

	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import { untrack } from 'svelte'
	import { AppService, SettingService, WorkspaceService } from '$lib/gen'
	import type { GuestUsage, Policy } from '$lib/gen'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Path from '$lib/components/Path.svelte'
	import { computeSecretUrl } from './appDeploy.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import LabelsInput from '$lib/components/LabelsInput.svelte'
	import RunOnBehalfOfSetting from '$lib/components/itemSettings/RunOnBehalfOfSetting.svelte'
	import { appOnBehalfOfTooltip } from '$lib/components/itemSettings/onBehalfOf'
	import SettingToggle from '$lib/components/itemSettings/SettingToggle.svelte'
	import { canUserBypassRuleKind, protectionRulesState } from '$lib/workspaceProtectionRules.svelte'
	import { FRONTEND_SDK_SCOPES, sdkScopesToConfirm } from '$lib/components/raw_apps/sdkScopes'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import {
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspaceStore = useOperatingWorkspace()
	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)
	const roleCaps = useRoleCapabilities()
	// The server forces a composed app into sandbox isolation.
	const composeOnlyApps = $derived($roleCaps.apps === 'compose')

	let {
		open = $bindable(),
		tab = $bindable(),
		policy,
		appPath,
		customPath = $bindable(),
		savedApp,
		summary = $bindable(),
		customPathError = $bindable(),
		pathError = $bindable(),
		newEditedPath = $bindable(),
		newPath,
		preserveOnBehalfOf = $bindable(),
		labels = $bindable(),
		rawApp = false,
		newApp = false,
		operatingWorkspace = undefined,
		showErrors = false
	}: {
		open: boolean
		tab: AppSettingsTab
		/** Mutated in place; deployed with the app, like every other setting here. */
		policy: Policy
		appPath: string
		customPath: string | undefined
		savedApp: { path: string; policy: any; custom_path?: string } | undefined
		summary: string
		customPathError: string
		pathError: string
		newEditedPath: string
		newPath: string
		preserveOnBehalfOf: boolean | undefined
		labels?: string[] | undefined
		// Only raw apps are offered the `wm_coep` embedding hint: on a classic
		// (low-code) app COEP breaks no-CORP cross-origin subresources (external
		// images, {@html} embeds, CDN imports) that the app cannot opt out of.
		rawApp?: boolean
		/** True while the editor is on a draft-only URL (`/edit/u/{user}/draft_{uuid}`
		 *  with no deployed row yet). Suppresses the public-secret-URL fetch
		 *  (`/secret_of/...` 404s with no `app` row) and renders a placeholder
		 *  instead of the eternally-spinning link. */
		newApp?: boolean
		/** Workspace the app is deployed to. The secret-URL / custom-path / folder /
		 *  on-behalf-of lookups target it; defaults to the operating workspace. */
		operatingWorkspace?: string
		/** A deploy was refused over these errors: show them before the fields are edited. */
		showErrors?: boolean
	} = $props()

	const opWs = $derived(operatingWorkspace ?? $operatingWorkspaceStore)

	// Admins always pass the backend check. For everyone else, fail closed
	// while the workspace protection rules are still loading so the toggle
	// is never briefly enabled for a user the rules will end up restricting.
	let rulesetsLoaded = $derived(protectionRulesState.rulesets !== undefined)
	let canSetAnonymous = $derived(
		!!actingUser?.is_admin ||
			!!actingUser?.is_super_admin ||
			(rulesetsLoaded &&
				canUserBypassRuleKind('RestrictAnonymousAppDeployment', $userStore ?? undefined))
	)
	let canSetGuest = $derived(
		!!actingUser?.is_admin ||
			!!actingUser?.is_super_admin ||
			(rulesetsLoaded &&
				canUserBypassRuleKind('RestrictGuestAppDeployment', $userStore ?? undefined))
	)
	// The three rungs of the access control, widest last. `viewer` is a fourth
	// execution mode that this control never sets (it runs components as the viewer,
	// which a guest cannot be), so an app in it shows as members-only here.
	let accessMode = $derived(
		policy.execution_mode == 'anonymous'
			? 'anonymous'
			: policy.execution_mode == 'guest'
				? 'guest'
				: 'publisher'
	)
	// Undefined until loaded. An app can be set to `guest` while the workspace has
	// guests off, in which case the mode is stored but inert -- say so rather than
	// letting the publisher believe the app is open.
	let guestAccessEnabled: boolean | undefined = $state(undefined)
	let guestUsage: GuestUsage | undefined = $state(undefined)
	// Whether the deployment can have guests at all; off, the mode is not on offer. The
	// backend decides; the hostname stands in until it has answered, the shared cloud
	// being the only deployment where guests are unavailable.
	let guestsAvailable = $derived.by(() => guestUsage?.available ?? !isCloudHosted())

	$effect(() => {
		const ws = opWs
		if (ws === undefined) return
		untrack(() => {
			WorkspaceService.getPublicSettings({ workspace: ws })
				.then((s) => (guestAccessEnabled = s.guest_access_enabled))
				.catch(() => (guestAccessEnabled = undefined))
			WorkspaceService.getGuestUsage({ workspace: ws })
				.then((u) => (guestUsage = u))
				.catch(() => (guestUsage = undefined))
		})
	})

	function onAccessModeChange(mode: string | undefined) {
		if (mode === undefined || mode === accessMode) return
		policy.execution_mode = mode as Policy['execution_mode']
	}

	let isAdmin = $derived(!!actingUser?.is_admin || !!actingUser?.is_super_admin)

	// The URLs below only serve what is deployed, so a pending access change must not read as
	// already live.
	let sharingChanged = $derived(
		!newApp &&
			(policy.execution_mode != savedApp?.policy?.execution_mode ||
				(customPath ?? null) !== (savedApp?.custom_path ?? null))
	)
	let pathChanged = $derived(
		!newApp && !!savedApp?.path && !!newEditedPath && newEditedPath !== savedApp.path
	)

	let dirtyCustomPath = $state(false)
	let path: Path | undefined = $state(undefined)

	let dirtyPath = $state(false)

	async function appExists(customPath: string) {
		return await AppService.customPathExists({
			workspace: opWs!,
			customPath
		})
	}

	let globalWorkspacedRoute = $state(false)

	async function loadGlobalWorkspacedRouteSetting() {
		try {
			const setting = await SettingService.getGlobal({ key: 'app_workspaced_route' })
			globalWorkspacedRoute = (setting as boolean) ?? false
		} catch (error) {
			globalWorkspacedRoute = false
		}
	}

	loadGlobalWorkspacedRouteSetting()

	let secretUrl: string | undefined = $state(undefined)
	let secretUrlHref = $derived(secretUrl ? computeSecretUrl(secretUrl) : undefined)
	let fullCustomUrl = $derived(
		`${window.location.origin}/a/${
			isCloudHosted() || globalWorkspacedRoute ? opWs + '/' : ''
		}${customPath}`
	)

	// The app URL a guest JWT rides on: append `guest.<jwt>` and the viewer authenticates the
	// token as a seatless guest. Uses the custom URL when set, else the public secret URL.
	let guestJwtBase = $derived(customPath !== undefined ? fullCustomUrl : secretUrlHref)

	let embedMode = $state(false)
	function toEmbedSnippet(url: string): string {
		return `<iframe src="${url}" title="Windmill app" width="100%" height="600"></iframe>`
	}
	async function getSecretUrl() {
		secretUrl = await AppService.getPublicSecretOfApp({
			workspace: opWs!,
			path: appPath
		})
	}

	let validateTimeout: number | undefined = undefined
	async function validateCustomPath(customPath: string): Promise<void> {
		customPathError = ''
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(async () => {
			if (!/^[\w-]+(\/[\w-]+)*$/.test(customPath)) {
				customPathError = 'Invalid path'
			} else if (customPath !== savedApp?.custom_path && (await appExists(customPath))) {
				customPathError = 'Path already taken'
			} else {
				customPathError = ''
			}
			validateTimeout = undefined
		}, 500)
	}

	$effect(() => {
		;[customPath]
		untrack(() => customPath !== undefined && validateCustomPath(customPath))
	})

	$effect(() => {
		// Skip the secret URL fetch on draft-only items — `/secret_of/...`
		// has no `app` row to look up and would 404, leaving the UI
		// component spinning indefinitely.
		open &&
			tab === 'sharing' &&
			!newApp &&
			appPath &&
			appPath != '' &&
			savedApp &&
			secretUrl == undefined &&
			untrack(() => getSecretUrl())
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

{#snippet adminOnlyHint()}
	Only workspace admins can set a custom URL.
{/snippet}

<Drawer bind:open size="800px">
	<DrawerContent noPadding title="Settings" on:close={() => (open = false)}>
		<div class="flex flex-col h-full">
			<Tabs bind:selected={tab} wrapperClass="flex-none w-full">
				<Tab value="metadata" label="Metadata" />
				<Tab value="permissions" label="Permissions" />
				<Tab value="sharing" label="Sharing" />

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

						<TabContent value="permissions">
							<div class="flex flex-col gap-6 p-4 pb-8">
								<RunOnBehalfOfSetting
									kind="app"
									required
									tooltip={appOnBehalfOfTooltip(rawApp)}
									bind:email={policy.on_behalf_of_email}
									bind:permissionedAs={policy.on_behalf_of}
									bind:preserve={preserveOnBehalfOf}
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
							</div>
						</TabContent>

						<TabContent value="sharing">
							<div class="flex flex-col gap-6 p-4 pb-8">
								{#if sharingChanged}
									<Alert type="info" size="xs" title="Deploy the app to apply these changes." />
								{/if}
								<Label label="Access">
									{#if rulesetsLoaded && !canSetAnonymous && policy.execution_mode != 'anonymous'}
										<Alert
											type="warning"
											title="Restricted by a workspace protection rule"
											size="xs"
										>
											Opening this app to anyone with the link is restricted to workspace admins and
											bypass users by a workspace protection rule
										</Alert>
									{/if}
									{#if rulesetsLoaded && !canSetGuest && policy.execution_mode != 'guest' && guestsAvailable}
										<Alert
											type="warning"
											title="Restricted by a workspace protection rule"
											size="xs"
										>
											Opening this app to guests is restricted to workspace admins and bypass users
											by a workspace protection rule
										</Alert>
									{/if}
									<div>
										<ToggleButtonGroup
											selected={accessMode}
											on:selected={(e) => onAccessModeChange(e.detail)}
										>
											{#snippet children({ item })}
												<ToggleButton
													label="Members"
													value="publisher"
													tooltip="Workspace members with read access on this app."
													{item}
												/>
												<ToggleButton
													label="Guests"
													value="guest"
													disabled={policy.execution_mode != 'guest' &&
														(!guestsAvailable || !canSetGuest)}
													tooltip={!guestsAvailable
														? 'Not available on Windmill Cloud. Guests require a self-hosted instance or a dedicated Windmill Cloud deployment.'
														: "Anyone your identity provider authenticates who has no Windmill account, plus workspace members. No membership, no seat up to the instance's allowance."}
													{item}
												/>
												<ToggleButton
													label="Public"
													value="anonymous"
													disabled={!canSetAnonymous && policy.execution_mode != 'anonymous'}
													tooltip="Anyone with the secret URL. No login."
													{item}
												/>
											{/snippet}
										</ToggleButtonGroup>
									</div>
									<div class="text-xs text-secondary">
										{#if policy.execution_mode == 'anonymous'}
											Anyone holding the secret URL below can open this app without signing in.
										{:else if policy.execution_mode == 'guest'}
											{#if !guestsAvailable}
												Guests are not available on Windmill Cloud, so this app still admits members
												only. They require a self-hosted instance or a dedicated Windmill Cloud
												deployment.
											{:else if guestUsage && !guestUsage.instance_enabled}
												A superadmin has turned guests off for this instance, so this app still
												admits members only.
											{:else if guestAccessEnabled === undefined}
												Checking whether this workspace allows guests…
											{:else if guestAccessEnabled === false}
												Guests are turned off for this workspace, so this app still admits members
												only. A workspace admin can turn them on in the workspace settings.
											{:else}
												Anyone your identity provider authenticates can open this app without a
												Windmill account. They join no workspace. Members of this workspace can open
												it too.
												{#if guestUsage}
													{guestUsage.guest_count} of {guestUsage.free_allowance} free guests used across
													this instance in the last {guestUsage.window_days} days; beyond that,
													{guestUsage.metered
														? 'every four guests count as one seat'
														: 'new guests are refused until the count drops'}.
												{/if}
											{/if}
										{:else}
											Only workspace members with read access on this app can open it.
										{/if}
									</div>
								</Label>

								<Label
									label="Secret URL"
									tooltip="An unguessable link to the app. Who it lets in depends on the access above."
								>
									{#snippet action()}
										{#if savedApp && !newApp && secretUrlHref}
											<Toggle
												size="xs"
												checked={embedMode}
												on:change={(e) => (embedMode = e.detail)}
												options={{ left: 'URL', right: 'Embed' }}
											/>
										{/if}
									{/snippet}
									{#if !savedApp || newApp}
										<ClipboardPanel
											content={`Deploy this app once to get the public secret URL`}
											size="md"
										/>
									{:else if secretUrlHref}
										<ClipboardPanel
											content={embedMode ? toEmbedSnippet(secretUrlHref) : secretUrlHref}
											size="md"
										/>
									{:else}<Loader2 size={16} class="animate-spin" />
									{/if}
									<div class="text-2xs text-secondary">
										{#if embedMode}
											Paste this iframe snippet into another app.
											{#if rawApp}
												To embed it in a cross-origin isolated page, add <code>wm_coep=on</code>
												<Tooltip
													>Sets the cross-origin isolation headers (COEP) on the app, which such a
													page requires of its iframes. The app then only loads cross-origin
													resources that send CORS or a Cross-Origin-Resource-Policy header.</Tooltip
												> to the URL.
											{/if}
											(if requiring login, top-level domain of embedding app must be the same as the
											one of Windmill)
										{:else}
											Share this URL directly, or switch to <b>Embed</b> to get an iframe snippet. Only
											the latest deployed version is served.
										{/if}
									</div>
								</Label>

								{#if embedMode && policy.execution_mode == 'guest' && guestAccessEnabled && guestJwtBase && guestsAvailable}
									<Label label="Embed for your own authenticated users (guest JWT)">
										<div class="text-xs text-secondary">
											To open this app for a user your own product already authenticates, mint a
											short-lived JWT in your backend and append it to the app URL as <code
												>guest.&lt;jwt&gt;</code
											>. Each token is its own seatless guest, confined to this app. Unlike the
											plain secret URL above, there is no shared secret and no Windmill account.
										</div>
										<div class="text-xs text-secondary">
											Windmill verifies the token against the workspace's guest JWT key (Workspace
											settings → Guests): a PEM public key or a JWKS URL{#if !isCloudHosted()}, or
												the instance's configured issuer (<code>JWT_EXT_JWKS_URL</code>) when no
												workspace key is set{/if}. Set the <b>public</b> half there; in your
											backend, sign each token with the matching
											<b>private</b> key using RS256/384/512, PS256/384/512 or ES256/384 (symmetric
											HS* is refused), carrying <code>email</code>,
											<code>workspace_id</code>
											=
											<code>{opWs}</code>, <code>app_path</code> = <code>{appPath}</code> and
											<code>exp</code> (at most 24h ahead).
										</div>
										<ClipboardPanel
											content={toEmbedSnippet(`${guestJwtBase}/guest.YOUR_GUEST_JWT`)}
											size="md"
										/>
										<div class="text-2xs text-secondary">
											Replace <code>YOUR_GUEST_JWT</code> with the token your backend signs per user.
											Past the instance's free guest allowance a new guest email is refused (see the
											count above); guests already seen in the window keep working.
										</div>
									</Label>
								{/if}

								<SettingToggle
									setting="custom-url"
									label="Custom URL"
									tooltip="Serve the app at a path you pick, on top of its secret URL."
									eeOnly
									disabled={!$enterpriseLicense || !isAdmin}
									checked={customPath !== undefined}
									hint={!isAdmin ? adminOnlyHint : undefined}
									onchange={(on) => {
										customPath = on ? '' : undefined
										if (!on) {
											customPathError = ''
										}
									}}
								>
									<Label label="Custom path" for="customPath">
										<TextInput
											inputProps={{
												id: 'customPath',
												autocomplete: 'off',
												disabled: !isAdmin,
												oninput: () => {
													dirtyCustomPath = true
												}
											}}
											bind:value={customPath}
											error={dirtyCustomPath || showErrors ? customPathError : undefined}
										/>
										{#if (dirtyCustomPath || showErrors) && customPathError}
											<div class="text-2xs text-red-600 dark:text-red-400">{customPathError}</div>
										{/if}
									</Label>
									<Label label="Custom public URL">
										<ClipboardPanel
											content={embedMode ? toEmbedSnippet(fullCustomUrl) : fullCustomUrl}
											size="md"
										/>
									</Label>
								</SettingToggle>

								<a
									href="https://www.windmill.dev/docs/advanced/external_auth_with_jwt#embed-public-apps-using-your-own-authentification"
									class="text-2xs"
									>Embed this app in your own product to be used by your own users</a
								>
							</div>
						</TabContent>
					</div>
				{/snippet}
			</Tabs>
		</div>
	</DrawerContent>
</Drawer>
