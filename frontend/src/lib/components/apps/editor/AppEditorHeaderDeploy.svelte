<script lang="ts">
	import { Alert } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { Loader2 } from 'lucide-svelte'

	import Tooltip from '$lib/components/Tooltip.svelte'

	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import { untrack } from 'svelte'
	import { AppService, SettingService } from '$lib/gen'
	import Path from '$lib/components/Path.svelte'
	import { computeSecretUrl } from './appDeploy.svelte'
	import { base } from '$lib/base'
	import { isCloudHosted } from '$lib/cloud'
	import EEOnly from '$lib/components/EEOnly.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import LabelsInput from '$lib/components/LabelsInput.svelte'
	import OnBehalfOfSelector, {
		type OnBehalfOfChoice
	} from '$lib/components/OnBehalfOfSelector.svelte'
	import { canUserBypassRuleKind, protectionRulesState } from '$lib/workspaceProtectionRules.svelte'

	const WM_DEPLOYERS_GROUP = 'wm_deployers'

	let {
		policy,
		setPublishState,
		appPath,
		customPath = $bindable(),
		onLatest,
		savedApp,
		summary = $bindable(),
		deploymentMsg = $bindable(),
		customPathError = $bindable(),
		pathError = $bindable(),
		newEditedPath = $bindable(),
		newPath,
		hideSecretUrl = false,
		preserveOnBehalfOf = $bindable(false),
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
		savedApp: any
		summary: string
		deploymentMsg: string | undefined
		customPathError: string
		pathError: string
		newEditedPath: string
		newPath: string
		hideSecretUrl?: boolean
		preserveOnBehalfOf?: boolean
		labels?: string[] | undefined
		// Raw apps need cross-origin isolation (wm_coep) to be embeddable. Classic
		// (low-code) apps must NOT get the flag — it would force COEP on the
		// document and break no-CORP cross-origin subresources (external images,
		// {@html} embeds, CDN imports).
		rawApp?: boolean
		/** True while the editor is on a draft-only URL (`/edit/u/{user}/draft_{uuid}`
		 *  with no deployed row yet). Suppresses the public-secret-URL fetch
		 *  (`/secret_of/...` 404s with no `app` row) and renders a placeholder
		 *  instead of the eternally-spinning link. */
		newApp?: boolean
		/** Workspace the app is deployed to — the session's acting workspace when
		 *  embedded in a session preview, else the navigation `$workspaceStore`.
		 *  The secret-URL / custom-path / folder / on-behalf-of lookups must target
		 *  it, not `$workspaceStore` (which stays on the nav workspace in a session). */
		operatingWorkspace?: string
	} = $props()

	const opWs = $derived(operatingWorkspace ?? $workspaceStore)

	let isDeployer = $derived($userStore?.groups?.includes(WM_DEPLOYERS_GROUP) ?? false)
	// Admins always pass the backend check. For everyone else, fail closed
	// while the workspace protection rules are still loading so the toggle
	// is never briefly enabled for a user the rules will end up restricting.
	let rulesetsLoaded = $derived(protectionRulesState.rulesets !== undefined)
	let canSetAnonymous = $derived(
		!!$userStore?.is_admin ||
			!!$userStore?.is_super_admin ||
			(rulesetsLoaded &&
				canUserBypassRuleKind('RestrictAnonymousAppDeployment', $userStore ?? undefined))
	)
	let canPreserve = $derived(!!$userStore?.is_admin || !!$userStore?.is_super_admin || isDeployer)
	let savedOnBehalfOfEmail = $derived(savedApp?.policy?.on_behalf_of_email)
	let savedOnBehalfOf = $derived(savedApp?.policy?.on_behalf_of)
	let onBehalfOfChoice: OnBehalfOfChoice = $state(undefined)
	let customOnBehalfOfEmail: string = $state('')
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
		`${window.location.origin}${base}/a/${
			isCloudHosted() || globalWorkspacedRoute ? opWs + '/' : ''
		}${customPath}`
	)

	// When embedding a raw app in an iframe inside another Windmill app (or any
	// cross-origin-isolated page), the embedded document must set COEP. The
	// `wm_coep` flag opts the public app into the cross-origin isolation headers.
	// Only raw apps get it — for classic (low-code) apps COEP would break
	// no-CORP cross-origin subresources, so their snippet stays a plain iframe.
	let embedMode = $state(false)
	function toEmbedSnippet(url: string): string {
		const finalUrl = rawApp ? `${url}${url.includes('?') ? '&' : '?'}wm_coep=on` : url
		return `<iframe src="${finalUrl}" title="Windmill app" width="100%" height="600"></iframe>`
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

{#if !onLatest}
	<Alert title="You're not on the latest app version. " type="warning">
		By deploying, you may overwrite changes made by other users. Press 'Deploy' to see diff.
	</Alert>
	<div class="py-2"></div>
{/if}
<label for="summary" class="text-emphasis text-xs font-semibold">Summary</label>
<div class="w-full pt-1">
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
</div>
<div class="pt-3"></div>
<LabelsInput bind:labels class="-mt-4" />
<div class="py-6"></div>
<label for="deploymentMsg" class="text-emphasis text-xs font-semibold">Deployment message</label>
<div class="w-full pt-1">
	<!-- svelte-ignore a11y_autofocus -->
	<TextInput
		inputProps={{
			id: 'deploymentMsg',
			placeholder: 'Optional deployment message'
		}}
		bind:value={deploymentMsg}
	/>
</div>
<div class="py-6"></div>
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

<div class="py-2"></div>
<Alert title="App executed on behalf of you">
	A viewer of the app will execute the runnables of the app on behalf of the publisher (you)
	<Tooltip>
		It ensures that all required resources/runnable visible for publisher but not for viewer at time
		of creating the app would prevent the execution of the app. To guarantee tight security, a
		policy is computed at time of deployment of the app which only allow the scripts/flows referred
		to in the app to be called on behalf of. Furthermore, static parameters are not overridable.
		Hence, users will only be able to use the app as intended by the publisher without risk for
		leaking resources not used in the app.
	</Tooltip>
	{#if canPreserve}
		<div class="mt-4">
			Because you are either an admin or part of the {WM_DEPLOYERS_GROUP} group, you can select another
			user to run this app on behalf of. Once deployed the app will be run on behalf of
			<OnBehalfOfSelector
				targetWorkspace={opWs ?? ''}
				targetValue={savedOnBehalfOfEmail}
				selected={onBehalfOfChoice}
				onSelect={(choice, details) => {
					onBehalfOfChoice = choice
					if (choice === 'me') {
						policy.on_behalf_of_email = $userStore?.email
						policy.on_behalf_of = `u/${$userStore?.username}`
						customOnBehalfOfEmail = ''
						preserveOnBehalfOf = false
					} else if (choice === 'target') {
						policy.on_behalf_of_email = savedOnBehalfOfEmail
						policy.on_behalf_of = savedOnBehalfOf
						customOnBehalfOfEmail = ''
						preserveOnBehalfOf = true
					} else if (choice === 'custom' && details) {
						policy.on_behalf_of_email = details.email
						policy.on_behalf_of = details.permissionedAs
						customOnBehalfOfEmail = details.email
						preserveOnBehalfOf = true
					}
				}}
				kind="app"
				{canPreserve}
				customValue={customOnBehalfOfEmail}
				isDeployment={false}
			/>
		</div>
	{/if}
</Alert>

<div class="mt-10"></div>

<div class="flex items-center gap-2">
	<h2>Sandbox isolation</h2>
	<Badge color="yellow">Alpha</Badge>
</div>
<div class="my-6">
	<Toggle
		options={{ right: "Isolate the app from the viewer's browser session" }}
		checked={policy.sandbox == true}
		on:change={(e) => {
			policy.sandbox = e.detail || undefined
			// A not-yet-deployed app has no row to PATCH — `setPublishState` (POST
			// /apps/update) would 404. The flag rides along in the `policy` the first
			// deploy sends (createApp), so here we only mutate it locally. Persist
			// incrementally once the app exists.
			if (savedApp && !newApp) {
				setPublishState(e.detail ? 'Sandbox isolation enabled' : 'Sandbox isolation disabled')
			}
		}}
		disabled={!savedApp}
	/>
	<div class="text-xs text-secondary mt-1">
		Controls what the app's browser-side code can reach in each viewer's browser — distinct from the
		on-behalf-of model above (which sets who its runnables run as). Off by default, the app's code
		uses the viewer's own session; enable it to confine the app to a narrowly-scoped token instead,
		on every surface (public URL and in-workspace). Leave it off if the app needs full browser
		features (IndexedDB, third-party auth/SDKs, OAuth redirects).
	</div>
	{#if newApp}
		<div class="text-xs text-tertiary mt-1">Takes effect when you first deploy this app.</div>
	{/if}
	{#if policy.sandbox == true}
		<div class="mt-2">
			<Alert type="warning" title="Alpha feature" size="xs">
				Sandbox isolation is in alpha. After enabling, open the app from its public URL to confirm
				it still works, and report any broken behavior.
			</Alert>
		</div>
	{/if}
</div>

{#if !hideSecretUrl}
	<h2>Public URL</h2>

	<div class="my-6">
		{#if rulesetsLoaded && !canSetAnonymous}
			<Alert type="warning" title="Restricted by a workspace protection rule" size="xs">
				Making this app publicly accessible without login is restricted to workspace admins and
				bypass users by a workspace protection rule
			</Alert>
			<div class="mb-2"></div>
		{/if}
		<div class="flex gap-2 items-center mb-2">
			<Toggle
				options={{
					left: `Require login and read-access`,
					right: `No login required`
				}}
				checked={policy.execution_mode == 'anonymous'}
				on:change={(e) => {
					policy.execution_mode = e.detail ? 'anonymous' : 'publisher'
					// Same as sandbox: a not-yet-deployed app has no row to PATCH, so
					// `setPublishState` would 404. The mode is carried by the first
					// deploy's policy; persist incrementally only once the app exists.
					if (savedApp && !newApp) {
						setPublishState()
					}
				}}
				disabled={!savedApp || (!canSetAnonymous && policy.execution_mode != 'anonymous')}
			/>
		</div>
		{#if !savedApp || newApp}
			<ClipboardPanel content={`Deploy this app once to get the public secret URL`} size="md" />
		{:else if secretUrlHref}
			<div class="flex justify-end mb-1">
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
		<div class="text-xs text-secondary mt-1">
			{#if embedMode}
				Paste this iframe snippet into another app.
				{#if rawApp}
					The <code>wm_coep</code> flag <Tooltip
						>Sets the cross-origin isolation headers (COEP) so the app can be embedded inside
						another Windmill app or any cross-origin-isolated page. Without it the browser blocks
						the iframe.</Tooltip
					> lets it load inside a cross-origin-isolated page.
				{/if}
				(if requiring login, top-level domain of embedding app must be the same as the one of Windmill)
			{:else}
				Share this url directly, or switch to <b>Embed</b> to get an iframe snippet.
			{/if}
		</div>

		<div class="mt-4">
			{#if !($userStore?.is_admin || $userStore?.is_super_admin)}
				<Alert type="warning" title="Admin only" size="xs">
					Custom path can only be set by workspace admins
				</Alert>
				<div class="mb-2"></div>
			{/if}
			<!-- svelte-ignore block_empty -->
			{#if !$enterpriseLicense}
				<EEOnly />
			{/if}
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
				disabled={!$enterpriseLicense || !($userStore?.is_admin || $userStore?.is_super_admin)}
			/>

			{#if customPath !== undefined}
				<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
					<div>Custom path</div>
				</div>
				<input
					disabled={!($userStore?.is_admin || $userStore?.is_super_admin)}
					type="text"
					autocomplete="off"
					bind:value={customPath}
					class={customPathError === ''
						? ''
						: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
					oninput={() => {
						dirtyCustomPath = true
					}}
				/>
				<div class="text-secondary text-sm flex items-center gap-1 mt-2 w-full justify-between">
					<div>Custom public URL</div>
				</div>
				<ClipboardPanel
					content={embedMode ? toEmbedSnippet(fullCustomUrl) : fullCustomUrl}
					size="md"
				/>

				<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5"
					>{dirtyCustomPath ? customPathError : ''}
				</div>
			{/if}
		</div>
	</div>
	<Alert type="info" title="Only latest deployed app is publicly available">
		You will still need to deploy the app to make visible the latest changes
	</Alert>

	<a
		href="https://www.windmill.dev/docs/advanced/external_auth_with_jwt#embed-public-apps-using-your-own-authentification"
		class="mt-4 text-2xs">Embed this app in your own product to be used by your own users</a
	>
{/if}
