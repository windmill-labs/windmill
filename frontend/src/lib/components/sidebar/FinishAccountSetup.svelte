<script lang="ts">
	import type { Component } from 'svelte'
	import { base } from '$lib/base'
	import { UserService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { GithubIcon, GitlabIcon, GoogleIcon } from '$lib/components/icons'
	import { OauthService } from '$lib/gen'

	// An account created through a pre-approved invite has no credentials yet
	// (`login_type` = pending_oauth): it is entered through single-use links. Either a
	// password or the first provider sign-in with the same address makes it a normal
	// account; the entry disappears with `login_type`, so nothing is stored here.
	let {
		open = $bindable(false),
		email,
		onDone
	}: { open?: boolean; email: string; onDone: () => void } = $props()

	const icons: Record<string, Component<any>> = {
		google: GoogleIcon,
		github: GithubIcon,
		gitlab: GitlabIcon
	}
	let logins = $state<{ type: string; displayName: string }[]>([])
	// The instance's SAML entry point, when SSO is configured: a SAML assertion adopts a
	// credential-less account the way an OAuth login does.
	let saml = $state<string | undefined>(undefined)
	// Where password login is off, a password would be a credential that cannot sign in;
	// the backend refuses it too, this just keeps the dead end off the screen.
	let passwordAllowed = $state(true)
	let password = $state('')
	let saving = $state(false)

	const labels: Record<string, string> = { github: 'GitHub', google: 'Google', gitlab: 'GitLab' }
	$effect(() => {
		if (!open) return
		UserService.isPasswordLoginDisabled()
			.then((disabled) => (passwordAllowed = !disabled))
			.catch(() => (passwordAllowed = true))
		OauthService.listOauthLogins()
			.then((r) => {
				logins = (r.oauth ?? []).map((l) => ({
					type: l.type,
					displayName:
						l.display_name || labels[l.type] || l.type.charAt(0).toUpperCase() + l.type.slice(1)
				}))
				saml = r.saml || undefined
			})
			.catch((err) => {
				console.warn('Could not list OAuth logins', err)
				logins = []
				saml = undefined
			})
	})

	async function setPassword() {
		if (password.length < 8) {
			sendUserToast('Use at least 8 characters', true)
			return
		}
		saving = true
		try {
			await UserService.setPassword({ requestBody: { password } })
			sendUserToast('Password set. You can sign in with it from now on.')
			open = false
			onDone()
		} catch (err) {
			sendUserToast(
				'Could not set the password: ' + (err instanceof Error ? err.message : err),
				true
			)
		} finally {
			saving = false
		}
	}
</script>

<Modal title="Finish setting up your account" bind:open cancelText="Later">
	<div class="flex flex-col gap-5">
		<p class="text-sm text-secondary">
			Your account <span class="font-medium text-primary">{email}</span> was created from an invite and
			has no sign-in method of its own yet. Pick one so you can come back any time.
		</p>

		{#if logins.length === 0 && !saml && !passwordAllowed}
			<p class="text-sm text-secondary">
				No sign-in method is available on this instance right now; ask an administrator.
			</p>
		{/if}
		{#if logins.length > 0 || saml}
			<div class="flex flex-col gap-2">
				<span class="text-xs font-semibold text-emphasis">Sign in with a provider</span>
				<div class="grid gap-2">
					{#each logins as login (login.type)}
						{@const Icon = icons[login.type]}
						<Button
							variant="default"
							unifiedSize="lg"
							startIcon={Icon ? { icon: Icon, classes: 'h-4' } : undefined}
							onClick={() => {
								// Read by the OAuth callback: a provider asserting another address is refused
								// rather than opening a second account under it (see oauth2_ee.rs).
								document.cookie = `finish_setup=${encodeURIComponent(email)}; path=/; max-age=600; SameSite=Lax`
								window.location.assign(`${base}/api/oauth/login/${login.type}`)
							}}
						>
							Continue with {login.displayName}
						</Button>
					{/each}
					{#if saml}
						<Button
							variant="default"
							unifiedSize="lg"
							onClick={() => {
								// The IdP posts to the ACS cross-site, where a SameSite cookie is not
								// sent, so the address rides in the RelayState the ACS lifts out. The
								// cookie stays as well: an IdP that drops RelayState but shares the
								// instance's site still delivers it, and either channel refuses.
								document.cookie = `finish_setup=${encodeURIComponent(email)}; path=/; max-age=600; SameSite=Lax`
								try {
									const url = new URL(saml!)
									url.searchParams.set('RelayState', `/?finish_setup=${encodeURIComponent(email)}`)
									window.location.assign(url.toString())
								} catch (e) {
									console.error('Could not set SAML RelayState', e)
									sendUserToast('Could not start sign-in, please try again.', true)
								}
							}}
						>
							Continue with SSO
						</Button>
					{/if}
				</div>
				<p class="text-2xs text-secondary">
					Sign in to the provider as {email}; a different address is refused and you stay signed in
					here.
				</p>
			</div>
			{#if passwordAllowed}
				<div class="flex items-center gap-3">
					<div class="h-px flex-1 bg-border-light"></div>
					<span class="text-2xs uppercase text-secondary">or</span>
					<div class="h-px flex-1 bg-border-light"></div>
				</div>
			{/if}
		{/if}

		{#if passwordAllowed}
			<div class="flex flex-col gap-2">
				<span class="text-xs font-semibold text-emphasis">Set a password</span>
				<div class="flex flex-row gap-2 items-center">
					<TextInput
						inputProps={{
							autocomplete: 'new-password',
							type: 'password',
							placeholder: 'At least 8 characters'
						}}
						bind:value={password}
					/>
					<Button variant="accent" unifiedSize="md" disabled={saving} onClick={setPassword}>
						Set password
					</Button>
				</div>
				<p class="text-2xs text-secondary"
					>A password account keeps signing in with the password only.</p
				>
			</div>
		{/if}
	</div>
</Modal>
