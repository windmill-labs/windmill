<script lang="ts">
	import { Alert, Section } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Badge from '$lib/components/Badge.svelte'
	import { enterpriseLicense, userStore } from '$lib/stores'
	import { useOperatorBuilderApps } from '$lib/operatorWriteRights'
	import { AlertTriangle, Loader2 } from 'lucide-svelte'

	import Tooltip from '$lib/components/Tooltip.svelte'

	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import { untrack } from 'svelte'
	import { AppService, SettingService, WorkspaceService } from '$lib/gen'
	import type { GuestUsage } from '$lib/gen'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Path from '$lib/components/Path.svelte'
	import { useFolderDefaultPermissionedAs } from '$lib/components/useFolderDefaultPermissionedAs.svelte'
	import { computeSecretUrl } from './appDeploy.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import EEOnly from '$lib/components/EEOnly.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import LabelsInput from '$lib/components/LabelsInput.svelte'
	import OnBehalfOfSelector, {
		type OnBehalfOfChoice
	} from '$lib/components/OnBehalfOfSelector.svelte'
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
	const operatorBuilderApps = useOperatorBuilderApps()

	const WM_DEPLOYERS_GROUP = 'wm_deployers'

	let {
		policy,
		setPublishState,
		appPath,
		customPath = $bindable(),
		onLatest,
		headUnknown = false,
		savedApp,
		summary = $bindable(),
		deploymentMsg = $bindable(),
		customPathError = $bindable(),
		pathError = $bindable(),
		newEditedPath = $bindable(),
		newPath,
		hideSecretUrl = false,
		labels = $bindable(),
		rawApp = false,
		newApp = false,
		operatingWorkspace = undefined
	}: {
		policy: any
		setPublishState: (message?: string) => void
		appPath: string
		customPath: string | undefined
		onLatest: boolean
		/** The comparison behind `onLatest` could not read the deployed head, so the warning
		 *  is caution rather than an observed newer version. */
		headUnknown?: boolean
		savedApp: any
		summary: string
		deploymentMsg: string | undefined
		customPathError: string
		pathError: string
		newEditedPath: string
		newPath: string
		hideSecretUrl?: boolean
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
	} = $props()

	const opWs = $derived(operatingWorkspace ?? $operatingWorkspaceStore)

	let isDeployer = $derived(actingUser?.groups?.includes(WM_DEPLOYERS_GROUP) ?? false)
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
		policy.execution_mode = mode
		// Same as sandbox: a not-yet-deployed app has no row to PATCH, so
		// `setPublishState` would 404. The mode is carried by the first deploy's
		// policy; persist incrementally only once the app exists.
		if (savedApp && !newApp) {
			setPublishState()
		}
	}
	let canPreserve = $derived(!!actingUser?.is_admin || !!actingUser?.is_super_admin || isDeployer)
	let savedOnBehalfOfEmail = $derived(savedApp?.policy?.on_behalf_of_email)
	let savedOnBehalfOf = $derived(savedApp?.policy?.on_behalf_of)
	let onBehalfOfChoice: OnBehalfOfChoice = $state(undefined)
	let customOnBehalfOfEmail: string = $state('')
	let myOnBehalfOf = $derived(actingUser?.username ? `u/${actingUser.username}` : undefined)
	// A new app starts on the folder rule its path matches, or on the deployer, and deploys
	// what is shown: naming nobody would let the server apply a rule this picker did not see.
	const folderDefault = useFolderDefaultPermissionedAs(
		() => (newApp ? newEditedPath : undefined),
		() => opWs
	)
	let defaultChosen = false
	$effect(() => {
		const rule = folderDefault.value
		if (!newApp || !canPreserve || folderDefault.loading) return
		untrack(() => {
			if (onBehalfOfChoice !== undefined && !defaultChosen) return
			defaultChosen = true
			onBehalfOfChoice = rule ? 'custom' : 'me'
			customOnBehalfOfEmail = rule ?? ''
			policy.on_behalf_of = rule ?? myOnBehalfOf
			policy.on_behalf_of_email = rule ? undefined : $userStore?.email
		})
	})
	// Without the right to preserve, the backend runs the redeployed app as the deployer whatever
	// the policy says, as it does for "me".
	let deployOnBehalfOf = $derived.by(() => {
		if (!canPreserve || onBehalfOfChoice === 'me') return myOnBehalfOf
		if (onBehalfOfChoice === 'custom') return policy.on_behalf_of
		return savedOnBehalfOf
	})
	let onBehalfOfWillChange = $derived(
		!newApp && !!savedOnBehalfOf && !!deployOnBehalfOf && deployOnBehalfOf !== savedOnBehalfOf
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
		!newApp &&
			appPath &&
			appPath != '' &&
			savedApp &&
			secretUrl == undefined &&
			untrack(() => getSecretUrl())
	})
</script>

<div class="flex flex-col gap-8">
	{#if !onLatest}
		<Alert
			title={headUnknown
				? 'Could not check the deployed version. '
				: "You're not on the latest app version. "}
			type="warning"
		>
			{headUnknown
				? "Deploying may overwrite changes made by other users. Press 'Deploy' to see diff."
				: "By deploying, you may overwrite changes made by other users. Press 'Deploy' to see diff."}
		</Alert>
	{/if}

	<div class="flex flex-col gap-6">
		<div class="flex flex-col gap-1">
			<label for="summary" class="text-emphasis text-xs font-semibold">Summary</label>
			<!-- svelte-ignore a11y_autofocus -->
			<TextInput
				inputProps={{
					id: 'summary',
					autofocus: true,
					placeholder: 'App summary',
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
			<LabelsInput bind:labels />
		</div>
		<div class="flex flex-col gap-1">
			<label for="deploymentMsg" class="text-emphasis text-xs font-semibold">
				Deployment message
			</label>
			<TextInput
				inputProps={{
					id: 'deploymentMsg',
					placeholder: 'Optional deployment message'
				}}
				bind:value={deploymentMsg}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="path" class="text-emphasis text-xs font-semibold">Path</label>
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
		</div>
	</div>

	<Section
		label="Run on behalf of"
		tooltip={`The app's runnables run as this user whoever opens the app, so viewers need no access of their own. To keep that safe, the app can call only the runnables it was deployed with, and viewers cannot override their static inputs. Runnables still get the viewer's email in WM_END_USER_EMAIL${rawApp ? ', and inline ones can be set to read data tables and object storage as the viewer' : ''}.`}
	>
		<div class="flex flex-col items-start gap-1">
			<div class="flex items-center gap-1.5">
				<OnBehalfOfSelector
					targetWorkspace={opWs ?? ''}
					targetValue={savedOnBehalfOfEmail}
					selected={onBehalfOfChoice}
					onSelect={(choice, details) => {
						defaultChosen = false
						onBehalfOfChoice = choice
						if (choice === 'me') {
							policy.on_behalf_of_email = $userStore?.email
							policy.on_behalf_of = `u/${$userStore?.username}`
							customOnBehalfOfEmail = ''
						} else if (choice === 'target') {
							policy.on_behalf_of_email = savedOnBehalfOfEmail
							policy.on_behalf_of = savedOnBehalfOf
							customOnBehalfOfEmail = ''
						} else if (choice === 'custom' && details) {
							policy.on_behalf_of_email = details.email
							policy.on_behalf_of = details.permissionedAs
							customOnBehalfOfEmail = details.email
						}
					}}
					kind="app"
					{canPreserve}
					customValue={customOnBehalfOfEmail}
					isDeployment={false}
					myPermissionedAs={myOnBehalfOf}
					readOnlyWithoutPreserve
					folderDefault={folderDefault.value}
				/>
				{#if onBehalfOfWillChange}
					<AlertTriangle class="w-3.5 h-3.5 text-yellow-500" />
					<span class="text-2xs text-yellow-600 dark:text-yellow-400"
						>will change from <strong>{savedOnBehalfOf}</strong> on deploy</span
					>
				{/if}
			</div>
			<div class="text-2xs text-secondary">
				{#if canPreserve}
					Viewers run the app's runnables as this user.
				{:else}
					Only workspace admins and members of {WM_DEPLOYERS_GROUP} can pick another user.
				{/if}
			</div>
		</div>
	</Section>

	<Section label="Sandbox isolation">
		<Toggle
			options={{
				right: "Isolate the app from the viewer's browser session",
				rightTooltip:
					"Controls what the app's browser-side code can reach in each viewer's browser, separately from who its runnables run as. A sandboxed app gets a narrowly scoped token on every surface, public URL and in-workspace. On by default for new apps. Turn it off only if the app needs IndexedDB, third-party sign-in or OAuth redirects."
			}}
			checked={$operatorBuilderApps || policy.sandbox == true}
			on:change={(e) => {
				policy.sandbox = e.detail || undefined
				// Counted where the toggle is flipped rather than where the policy is
				// persisted: a not-yet-deployed app only mutates it locally, and skipping
				// those would read as unused in the case where it is picked up front.
				logFeatureUsage('app_sandbox', 'toggled', {
					key: `${rawApp ? 'raw' : 'low_code'}:${e.detail ? 'on' : 'off'}`
				})
				// Frontend API access exists only for a sandboxed app, so turning
				// isolation off drops the declared scopes with it rather than leaving
				// them set but inert.
				if (!e.detail) {
					policy.frontend_sdk_scopes = undefined
				}
				// A not-yet-deployed app has no row to PATCH — `setPublishState` (POST
				// /apps/update) would 404. The flag rides along in the `policy` the first
				// deploy sends (createApp), so here we only mutate it locally. Persist
				// incrementally once the app exists.
				if (savedApp && !newApp) {
					setPublishState(e.detail ? 'Sandbox isolation enabled' : 'Sandbox isolation disabled')
				}
			}}
			disabled={!savedApp || $operatorBuilderApps}
		/>
		{#if $operatorBuilderApps || newApp}
			<div class="text-2xs text-secondary mt-1">
				{#if $operatorBuilderApps}
					Required for apps built by operators, and cannot be turned off.
				{/if}
				{#if newApp}
					Takes effect when you first deploy this app.
				{/if}
			</div>
		{/if}
		{#if !$operatorBuilderApps && policy.sandbox != true}
			<div class="mt-3">
				<Alert type="warning" title="Not isolated" size="xs">
					The app's frontend code runs with each viewer's full Windmill permissions. A future
					version will ask viewers to approve this. Keep isolation off only if the app needs it.
				</Alert>
			</div>
		{/if}

		{#if rawApp && policy.sandbox == true}
			<div class="mt-6">
				<Section
					label="Frontend API access"
					small
					collapsable
					tooltip="Each viewer approves these scopes before the app runs. The app's code, or an XSS bug in it, can use them as that viewer."
				>
					{#snippet badge()}
						{#if sdkScopesToConfirm(policy.frontend_sdk_scopes ?? []).length}
							<div class="flex grow min-w-0 flex-wrap gap-1 ps-2">
								{#each FRONTEND_SDK_SCOPES.filter( (s) => policy.frontend_sdk_scopes.includes(s.value) ) as scope (scope.value)}
									<Badge twBgColor="bg-surface-sunken" twTextColor="text-primary"
										>{scope.label}</Badge
									>
								{/each}
							</div>
						{/if}
					{/snippet}
					<div class="text-xs text-secondary mb-3">
						Let the app's frontend call the Windmill API as <b>the viewer</b> through the
						<code>windmill-client</code> SDK, which it lists as a dependency. Grant only what the app
						needs.
					</div>
					<div class="flex flex-col gap-2">
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
										// Same as sandbox: a not-yet-deployed app has no row to PATCH, so the
										// scopes ride along in the first deploy's policy instead.
										if (savedApp && !newApp) {
											setPublishState('Frontend API access updated')
										}
									}}
									disabled={!savedApp}
								/>
								<div class="text-2xs text-hint ml-9">{scope.description}</div>
							</div>
						{/each}
					</div>
					{#if sdkScopesToConfirm(policy.frontend_sdk_scopes ?? []).length}
						<div class="mt-3">
							<Alert type="info" title="Redeploy to use the SDK from a sandboxed app" size="xs">
								A sandboxed app calls the API cross-origin, which older <code>windmill-client</code>
								versions cannot do. An app bundled before this Windmill version fails with a CORS error
								until you deploy it again, which re-bundles it against a current client.
							</Alert>
						</div>
					{/if}
				</Section>
			</div>
		{/if}
	</Section>

	{#if !hideSecretUrl}
		<Section label="Access">
			<div class="flex flex-col gap-2">
				{#if rulesetsLoaded && !canSetAnonymous && policy.execution_mode != 'anonymous'}
					<Alert type="warning" title="Restricted by a workspace protection rule" size="xs">
						Opening this app to anyone with the link is restricted to workspace admins and bypass
						users by a workspace protection rule
					</Alert>
				{/if}
				{#if rulesetsLoaded && !canSetGuest && policy.execution_mode != 'guest' && guestsAvailable}
					<Alert type="warning" title="Restricted by a workspace protection rule" size="xs">
						Opening this app to guests is restricted to workspace admins and bypass users by a
						workspace protection rule
					</Alert>
				{/if}
				<div>
					<ToggleButtonGroup
						selected={accessMode}
						on:selected={(e) => onAccessModeChange(e.detail)}
						disabled={!savedApp}
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
								disabled={policy.execution_mode != 'guest' && (!guestsAvailable || !canSetGuest)}
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
							Guests are not available on Windmill Cloud, so this app still admits members only.
							They require a self-hosted instance or a dedicated Windmill Cloud deployment.
						{:else if guestUsage && !guestUsage.instance_enabled}
							A superadmin has turned guests off for this instance, so this app still admits members
							only.
						{:else if guestAccessEnabled === undefined}
							Checking whether this workspace allows guests…
						{:else if guestAccessEnabled === false}
							Guests are turned off for this workspace, so this app still admits members only. A
							workspace admin can turn them on in the workspace settings.
						{:else}
							Anyone your identity provider authenticates can open this app without a Windmill
							account. They join no workspace. Members of this workspace can open it too.
							{#if guestUsage}
								{guestUsage.guest_count} of {guestUsage.free_allowance} free guests used across this
								instance in the last {guestUsage.window_days} days; beyond that, {guestUsage.metered
									? 'every four guests count as one seat'
									: 'new guests are refused until the count drops'}.
							{/if}
						{/if}
					{:else}
						Only workspace members with read access on this app can open it.
					{/if}
				</div>
			</div>

			<div class="flex flex-col gap-1 mt-6">
				{#if !savedApp || newApp}
					<ClipboardPanel content={`Deploy this app once to get the public secret URL`} size="md" />
				{:else if secretUrlHref}
					<div class="flex justify-end">
						<Toggle
							size="xs"
							checked={embedMode}
							on:change={(e) => (embedMode = e.detail)}
							options={{ left: 'URL', right: 'Embed' }}
						/>
					</div>
					<ClipboardPanel
						content={embedMode ? toEmbedSnippet(secretUrlHref) : secretUrlHref}
						size="md"
					/>
				{:else}<Loader2 class="animate-spin" />
				{/if}
				<div class="text-2xs text-secondary">
					{#if embedMode}
						Paste this iframe snippet into another app.
						{#if rawApp}
							To embed it in a cross-origin isolated page, add <code>wm_coep=on</code>
							<Tooltip
								>Sets the cross-origin isolation headers (COEP) on the app, which such a page
								requires of its iframes. The app then only loads cross-origin resources that send
								CORS or a Cross-Origin-Resource-Policy header.</Tooltip
							> to the URL.
						{/if}
						(if requiring login, top-level domain of embedding app must be the same as the one of Windmill)
					{:else}
						Share this URL directly, or switch to <b>Embed</b> to get an iframe snippet. Only the latest
						deployed version is served.
					{/if}
				</div>
			</div>

			{#if embedMode && policy.execution_mode == 'guest' && guestAccessEnabled && guestJwtBase && guestsAvailable}
				<div class="mt-6 flex flex-col gap-2">
					<div class="text-xs font-semibold text-emphasis">
						Embed for your own authenticated users (guest JWT)
					</div>
					<div class="text-xs text-secondary">
						To open this app for a user your own product already authenticates, mint a short-lived
						JWT in your backend and append it to the app URL as <code>guest.&lt;jwt&gt;</code>. Each
						token is its own seatless guest, confined to this app. Unlike the plain secret URL
						above, there is no shared secret and no Windmill account.
					</div>
					<div class="text-xs text-secondary">
						Windmill verifies the token against the workspace's guest JWT key (Workspace settings →
						Guests): a PEM public key or a JWKS URL{#if !isCloudHosted()}, or the instance's
							configured issuer (<code>JWT_EXT_JWKS_URL</code>) when no workspace key is set{/if}.
						Set the <b>public</b> half there; in your backend, sign each token with the matching
						<b>private</b> key using RS256/384/512, PS256/384/512 or ES256/384 (symmetric HS* is
						refused), carrying <code>email</code>, <code>workspace_id</code> = <code>{opWs}</code>,
						<code>app_path</code> = <code>{appPath}</code> and <code>exp</code> (at most 24h ahead).
					</div>
					<ClipboardPanel
						content={toEmbedSnippet(`${guestJwtBase}/guest.YOUR_GUEST_JWT`)}
						size="md"
					/>
					<div class="text-2xs text-secondary">
						Replace <code>YOUR_GUEST_JWT</code> with the token your backend signs per user. Past the
						instance's free guest allowance a new guest email is refused (see the count above); guests
						already seen in the window keep working.
					</div>
				</div>
			{/if}

			<div class="flex flex-col gap-2 mt-6">
				{#if !(actingUser?.is_admin || actingUser?.is_super_admin)}
					<Alert type="warning" title="Admin only" size="xs">
						Custom path can only be set by workspace admins
					</Alert>
				{/if}
				<div class="flex items-center gap-2">
					<Toggle
						on:change={({ detail }) => {
							customPath = detail ? '' : undefined
							if (customPath === undefined) {
								customPathError = ''
							}
						}}
						checked={customPath !== undefined}
						options={{
							right: 'Use a custom URL'
						}}
						disabled={!$enterpriseLicense || !(actingUser?.is_admin || actingUser?.is_super_admin)}
					/>
					{#if !$enterpriseLicense}
						<EEOnly />
					{/if}
				</div>

				{#if customPath !== undefined}
					<div class="flex flex-col gap-1">
						<label for="customPath" class="text-emphasis text-xs font-semibold">Custom path</label>
						<TextInput
							inputProps={{
								id: 'customPath',
								autocomplete: 'off',
								disabled: !(actingUser?.is_admin || actingUser?.is_super_admin),
								oninput: () => {
									dirtyCustomPath = true
								}
							}}
							bind:value={customPath}
							error={dirtyCustomPath ? customPathError : undefined}
						/>
						{#if dirtyCustomPath && customPathError}
							<div class="text-2xs text-red-600 dark:text-red-400">{customPathError}</div>
						{/if}
					</div>
					<div class="flex flex-col gap-1">
						<div class="text-emphasis text-xs font-semibold">Custom public URL</div>
						<ClipboardPanel
							content={embedMode ? toEmbedSnippet(fullCustomUrl) : fullCustomUrl}
							size="md"
						/>
					</div>
				{/if}
			</div>

			<a
				href="https://www.windmill.dev/docs/advanced/external_auth_with_jwt#embed-public-apps-using-your-own-authentification"
				class="block mt-6 text-2xs"
				>Embed this app in your own product to be used by your own users</a
			>
		</Section>
	{/if}
</div>
