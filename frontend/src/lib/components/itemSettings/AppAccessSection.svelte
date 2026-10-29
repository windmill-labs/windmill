<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Label from '$lib/components/Label.svelte'
	import Description from '$lib/components/Description.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import { Loader2, Pen, Plus, Trash } from 'lucide-svelte'
	import { AppService, SettingService, WorkspaceService } from '$lib/gen'
	import type { GuestUsage } from '$lib/gen'
	import { enterpriseLicense, userStore } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { canUserBypassRuleKind, protectionRulesState } from '$lib/workspaceProtectionRules.svelte'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import { customPathError, type AppAccessMode } from './appAccess'

	interface Props {
		workspace: string
		/** The deployed path, or the path the app will be created at. */
		appPath: string
		rawApp?: boolean
		mode: AppAccessMode
		onModeChange: (mode: AppAccessMode) => void | Promise<void>
		/** Links, embedding and the custom URL, which exist only for a deployed app. */
		deployed?: {
			customPath: string | undefined
			onCustomPathChange: (customPath: string | null) => Promise<void>
		}
		disabled?: boolean
	}

	let {
		workspace,
		appPath,
		rawApp = false,
		mode,
		onModeChange,
		deployed,
		disabled = false
	}: Props = $props()

	// The toggle group keeps the option it was clicked to, so a change the server refuses would
	// stay highlighted while `mode` never moved. Remounting it once each change settles makes it
	// show the stored mode either way.
	let toggleKey = $state(0)
	async function selectMode(next: AppAccessMode) {
		try {
			await onModeChange(next)
		} finally {
			toggleKey++
		}
	}

	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)
	let isAdmin = $derived(!!actingUser?.is_admin || !!actingUser?.is_super_admin)

	// Admins always pass the backend check. Everyone else fails closed while the workspace
	// protection rules load, so a toggle the rules will restrict is never briefly enabled.
	let rulesetsLoaded = $derived(protectionRulesState.rulesets !== undefined)
	let canSetAnonymous = $derived(
		isAdmin ||
			(rulesetsLoaded &&
				canUserBypassRuleKind('RestrictAnonymousAppDeployment', $userStore ?? undefined))
	)
	let canSetGuest = $derived(
		isAdmin ||
			(rulesetsLoaded &&
				canUserBypassRuleKind('RestrictGuestAppDeployment', $userStore ?? undefined))
	)

	let guestUsage = $state<GuestUsage | undefined>(undefined)
	// The backend decides; the hostname stands in until it answers, the shared cloud being the
	// only deployment where guests are unavailable.
	let guestsAvailable = $derived(guestUsage?.available ?? !isCloudHosted())

	let globalWorkspacedRoute = $state(false)
	let secretUrl: string | undefined = $state(undefined)

	function load() {
		WorkspaceService.getGuestUsage({ workspace })
			.then((u) => (guestUsage = u))
			.catch(() => (guestUsage = undefined))
		if (!deployed) return
		SettingService.getGlobal({ key: 'app_workspaced_route' })
			.then((s) => (globalWorkspacedRoute = (s as boolean) ?? false))
			.catch(() => (globalWorkspacedRoute = false))
		AppService.getPublicSecretOfApp({ workspace, path: appPath })
			.then((s) => (secretUrl = s))
			.catch(() => (secretUrl = undefined))
	}
	load()

	let embedMode = $state(false)
	let secretUrlHref = $derived(
		secretUrl ? `${window.location.origin}/public/${workspace}/${secretUrl}` : undefined
	)
	let appUrl = $derived(
		`${window.location.origin}/${rawApp ? 'apps_raw' : 'apps'}/get/${appPath}?workspace=${workspace}`
	)

	// Custom URL: at most one per app. Adding, editing and removing it each write at once.
	let editingCustomPath = $state(false)
	let customPathInput = $state('')
	let customPathErr: string | undefined = $state(undefined)
	let savingCustomPath = $state(false)
	// Shown to admins on EE; the dialog's own writes disable the controls rather than hide them.
	let mayEditCustomPath = $derived(!!$enterpriseLicense && isAdmin)
	let canSaveCustomPath = $derived(
		!disabled &&
			!savingCustomPath &&
			customPathInput !== '' &&
			customPathInput !== deployed?.customPath
	)
	// The Embed toggle only shows outside Members, so it must not linger on from another mode.
	let customUrlEmbed = $derived(embedMode && mode != 'publisher')
	let customUrlBase = $derived(
		`${window.location.origin}/a/${isCloudHosted() || globalWorkspacedRoute ? workspace + '/' : ''}`
	)
	let fullCustomUrl = $derived(customUrlBase + (deployed?.customPath ?? ''))
	// The guest JWT rides on the custom URL when set, else on the secret URL.
	let guestJwtBase = $derived(deployed?.customPath ? fullCustomUrl : secretUrlHref)

	function editCustomPath() {
		customPathInput = deployed?.customPath ?? ''
		customPathErr = undefined
		editingCustomPath = true
	}

	async function saveCustomPath(next: string | null) {
		if (!deployed || savingCustomPath) return
		savingCustomPath = true
		try {
			customPathErr =
				next === null ? undefined : await customPathError(workspace, next, deployed.customPath)
			if (customPathErr) return
			await deployed.onCustomPathChange(next)
		} catch (err) {
			customPathErr = `Could not check whether the path is free: ${err}`
		} finally {
			savingCustomPath = false
		}
		// A refused write leaves the deployed path as it was: keep the field open to retry.
		if ((deployed.customPath ?? null) === next) editingCustomPath = false
	}

	function toEmbedSnippet(url: string): string {
		return `<iframe src="${url}" title="Windmill app" width="100%" height="600"></iframe>`
	}
</script>

<div class="flex flex-col gap-4">
	{#if rulesetsLoaded && !canSetAnonymous && mode != 'anonymous'}
		<Alert type="warning" title="Restricted by a workspace protection rule" size="xs">
			Opening this app to anyone with the link is restricted to workspace admins and bypass users by
			a workspace protection rule
		</Alert>
	{/if}
	{#if rulesetsLoaded && !canSetGuest && mode != 'guest' && guestsAvailable}
		<Alert type="warning" title="Restricted by a workspace protection rule" size="xs">
			Opening this app to guests is restricted to workspace admins and bypass users by a workspace
			protection rule
		</Alert>
	{/if}
	<div class="flex flex-col gap-1">
		<div>
			{#key toggleKey}
				<ToggleButtonGroup
					selected={mode}
					{disabled}
					onSelected={(m) => {
						if (m && m !== mode) selectMode(m as AppAccessMode)
					}}
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
							disabled={mode != 'guest' && (!guestsAvailable || !canSetGuest)}
							tooltip={!guestsAvailable
								? 'Not available on Windmill Cloud. Guests require a self-hosted instance or a dedicated Windmill Cloud deployment.'
								: "Anyone your identity provider authenticates who has no Windmill account, plus workspace members. No membership, no seat up to the instance's allowance."}
							{item}
						/>
						<ToggleButton
							label="Public"
							value="anonymous"
							disabled={!canSetAnonymous && mode != 'anonymous'}
							tooltip="Anyone with the secret URL. No login."
							{item}
						/>
					{/snippet}
				</ToggleButtonGroup>
			{/key}
		</div>
		<div class="text-xs text-secondary">
			{#if mode == 'anonymous'}
				Anyone holding the secret URL can open this app without signing in.
			{:else if mode == 'guest'}
				{#if !guestsAvailable}
					Guests are not available on Windmill Cloud, so this app still admits members only. They
					require a self-hosted instance or a dedicated Windmill Cloud deployment.
				{:else}
					Anyone your identity provider authenticates can open this app without a Windmill account.
					They join no workspace. Members of this workspace can open it too.
					{#if guestUsage}
						{guestUsage.guest_count} of {guestUsage.free_allowance} free guests used across this instance
						in the last {guestUsage.window_days} days; beyond that,
						{guestUsage.metered
							? 'every four guests count as one seat'
							: 'new guests are refused until the count drops'}.
					{/if}
				{/if}
			{:else}
				The people with access above, signed in to Windmill.
			{/if}
		</div>
	</div>

	{#if deployed}
		<Label label={mode == 'publisher' ? 'Link' : 'Secret URL'}>
			{#snippet action()}
				{#if secretUrlHref && mode != 'publisher'}
					<Toggle
						size="xs"
						checked={embedMode}
						on:change={(e) => (embedMode = e.detail)}
						options={{ left: 'URL', right: 'Embed' }}
					/>
				{/if}
			{/snippet}
			{#if mode == 'publisher'}
				<ClipboardPanel content={appUrl} size="md" />
			{:else if secretUrlHref}
				<ClipboardPanel
					content={embedMode ? toEmbedSnippet(secretUrlHref) : secretUrlHref}
					size="md"
				/>
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
						An unguessable link. Switch to <b>Embed</b> for an iframe snippet. Only the latest deployed
						version is served.
					{/if}
				</div>
			{:else}
				<Loader2 size={16} class="animate-spin" />
			{/if}
		</Label>

		<Label
			label="Custom URL"
			for="customPath"
			eeOnly
			tooltip="Serve the app at a path you pick, on top of its link."
		>
			{#snippet action()}
				{#if !deployed.customPath && !editingCustomPath}
					<Button
						variant="default"
						unifiedSize="sm"
						startIcon={{ icon: Plus }}
						disabled={!mayEditCustomPath || disabled}
						onclick={editCustomPath}
					>
						Add custom URL
					</Button>
				{/if}
			{/snippet}
			{#if editingCustomPath}
				<div class="flex flex-row gap-2 items-start">
					<div class="grow min-w-0 flex flex-col gap-1">
						<TextInput
							inputProps={{
								id: 'customPath',
								autocomplete: 'off',
								placeholder: 'invoices',
								onkeydown: (e: KeyboardEvent) => {
									if (e.key === 'Enter' && canSaveCustomPath) saveCustomPath(customPathInput)
								}
							}}
							size="sm"
							bind:value={customPathInput}
							error={customPathErr}
						/>
						{#if customPathErr}
							<span class="text-2xs text-red-600 dark:text-red-400">{customPathErr}</span>
						{:else}
							<span class="text-2xs text-secondary truncate">
								{customUrlBase}{customPathInput || 'invoices'}
							</span>
						{/if}
					</div>
					<Button
						variant="default"
						unifiedSize="sm"
						disabled={!canSaveCustomPath}
						loading={savingCustomPath}
						onclick={() => saveCustomPath(customPathInput)}
					>
						Save
					</Button>
					<Button
						variant="subtle"
						unifiedSize="sm"
						disabled={savingCustomPath}
						onclick={() => (editingCustomPath = false)}
					>
						Cancel
					</Button>
				</div>
			{:else if deployed.customPath}
				<div class="flex flex-row gap-1 items-center">
					<div class="grow min-w-0">
						<ClipboardPanel
							content={customUrlEmbed ? toEmbedSnippet(fullCustomUrl) : fullCustomUrl}
							size="md"
						/>
					</div>
					{#if mayEditCustomPath}
						<Button
							variant="subtle"
							unifiedSize="md"
							iconOnly
							startIcon={{ icon: Pen }}
							aria-label="Edit custom URL"
							disabled={disabled || savingCustomPath}
							onclick={editCustomPath}
						/>
						<Button
							variant="subtle"
							destructive
							unifiedSize="md"
							iconOnly
							startIcon={{ icon: Trash }}
							aria-label="Remove custom URL"
							{disabled}
							loading={savingCustomPath}
							onclick={() => saveCustomPath(null)}
						/>
					{/if}
				</div>
			{/if}
			{#if $enterpriseLicense && !isAdmin}
				<span class="text-2xs text-secondary">Only workspace admins can set a custom URL.</span>
			{/if}
		</Label>

		{#if embedMode && mode == 'guest' && guestJwtBase && guestsAvailable}
			<Label label="Embed for your own authenticated users (guest JWT)">
				<div class="text-xs text-secondary">
					To open this app for a user your own product already authenticates, mint a short-lived JWT
					in your backend and append it to the app URL as <code>guest.&lt;jwt&gt;</code>. Each token
					is its own seatless guest, confined to this app. Unlike the plain secret URL above, there
					is no shared secret and no Windmill account.
				</div>
				<div class="text-xs text-secondary">
					Windmill verifies the token against the workspace's guest JWT key (Workspace settings →
					Guests): a PEM public key or a JWKS URL{#if !isCloudHosted()}, or the instance's
						configured issuer (<code>JWT_EXT_JWKS_URL</code>) when no workspace key is set{/if}. Set
					the <b>public</b> half there; in your backend, sign each token with the matching
					<b>private</b> key using RS256/384/512, PS256/384/512 or ES256/384 (symmetric HS* is
					refused), carrying <code>email</code>, <code>workspace_id</code> =
					<code>{workspace}</code>,
					<code>app_path</code> = <code>{appPath}</code> and <code>exp</code> (at most 24h ahead).
				</div>
				<ClipboardPanel
					content={toEmbedSnippet(`${guestJwtBase}/guest.YOUR_GUEST_JWT`)}
					size="md"
				/>
				<div class="text-2xs text-secondary">
					Replace <code>YOUR_GUEST_JWT</code> with the token your backend signs per user. Past the instance's
					free guest allowance a new guest email is refused (see the count above); guests already seen
					in the window keep working.
				</div>
			</Label>
		{/if}

		<Description
			link="https://www.windmill.dev/docs/advanced/external_auth_with_jwt#embed-public-apps-using-your-own-authentification"
			class="text-2xs text-hint"
		>
			You can embed this app in your own product, for your own users.
		</Description>
	{/if}
</div>
