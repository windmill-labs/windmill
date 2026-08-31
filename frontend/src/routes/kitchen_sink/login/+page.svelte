<script lang="ts">
	// Every state the login page can be in, at /kitchen_sink/login.
	//
	// Each frame renders the real <Login> card inside the real page chrome, fed a fixed
	// instance configuration through `preview` so nothing here talks to the API: signing in
	// always fails with the credentials error, and the provider buttons don't navigate.
	import Login, { type LoginPreview } from '$lib/components/Login.svelte'
	import LoginHeading from '$lib/components/LoginHeading.svelte'
	import { WindmillIcon } from '$lib/components/icons'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { goto } from '$lib/navigation'
	import { onMount } from 'svelte'

	// A pixel-accurate login card that silently drops what is typed into it has no business on
	// a deployed instance, where browsers would offer to autofill real credentials into it.
	let enabled = $state(false)
	onMount(() => {
		enabled = import.meta.env.DEV
		if (!enabled) {
			goto('/')
		}
	})

	// The width the login page ships with today is max-w-sm; the others are for comparison.
	const widths = [
		{ label: 'xs — 320px', value: 'sm:max-w-xs' },
		{ label: 'sm — 384px (current)', value: 'sm:max-w-sm' },
		{ label: 'md — 448px', value: 'sm:max-w-md' },
		{ label: 'lg — 512px', value: 'sm:max-w-lg' }
	]
	let width = $state('sm:max-w-sm')

	const google = { type: 'google', displayName: 'Google' }
	const github = { type: 'github', displayName: 'GitHub' }
	const microsoft = { type: 'microsoft', displayName: 'Microsoft' }
	const okta = { type: 'okta', displayName: 'Okta' }
	const auth0 = { type: 'auth0', displayName: 'Auth0' }
	const gitlab = { type: 'gitlab', displayName: 'GitLab' }
	const nextcloud = { type: 'nextcloud', displayName: 'Nextcloud' }
	const pocketid = { type: 'pocketid', displayName: 'Pocket ID' }
	const custom = { type: 'keycloak', displayName: 'Keycloak' }

	type Variant = {
		title: string
		note: string
		preview: LoginPreview
		firstTime?: boolean
		email?: string
	}

	const variants: Variant[] = [
		{
			title: 'Self-hosted, password only',
			note: 'No OAuth provider and no SAML configured: the form is the whole page.',
			preview: {}
		},
		{
			title: 'Password only, SMTP configured',
			note: 'Adds the "Forgot password?" link — it only shows when the instance can send email.',
			preview: { smtpConfigured: true }
		},
		{
			title: 'First-time setup',
			note: 'Fresh instance: default credentials prefilled with the welcome line above them.',
			preview: { smtpConfigured: false },
			firstTime: true
		},
		{
			title: 'One provider',
			note: 'Password form is collapsed behind "Log in without third-party".',
			preview: { logins: [google] }
		},
		{
			title: 'Three providers',
			note: 'One button per row, whatever the count.',
			preview: { logins: [google, github, microsoft], smtpConfigured: true }
		},
		{
			title: 'Four providers',
			note: 'Four buttons, still stacked; the card grows by a row per provider.',
			preview: { logins: [google, github, microsoft, okta], smtpConfigured: true }
		},
		{
			title: 'Everything at once',
			note: '8 known providers + a custom one + SAML: the tallest the card ever gets.',
			preview: {
				logins: [google, github, microsoft, okta, auth0, gitlab, nextcloud, pocketid, custom],
				saml: true,
				smtpConfigured: true
			}
		},
		{
			title: 'Deep link with the email prefilled',
			note: '/user/login?email=… opens the form even though providers are configured.',
			preview: { logins: [google, github], smtpConfigured: true },
			email: 'someone@windmill.dev'
		},
		{
			title: 'Last used: Google',
			note: 'Badged and moved to the top of the list; the rest keep their order.',
			preview: {
				logins: [github, google, microsoft],
				smtpConfigured: true,
				lastUsed: { kind: 'oauth', provider: 'google' }
			}
		},
		{
			title: 'Last used: email and password',
			note: 'The form leads the card, already open, with the providers under the divider.',
			preview: {
				logins: [github, google],
				smtpConfigured: true,
				lastUsed: { kind: 'password' }
			}
		},
		{
			title: 'Long provider name, last used',
			note: 'A custom provider with a long display name, badged: the worst case for the label.',
			preview: {
				logins: [
					{ type: 'keycloak', displayName: 'Acme Corporation Single Sign-On' },
					github,
					{ type: 'authentik', displayName: 'Authentik (staging)' }
				],
				smtpConfigured: true,
				lastUsed: { kind: 'oauth', provider: 'keycloak' }
			}
		},
		{
			title: 'Last used: SSO',
			note: 'SAML leads the list when it is what worked last, ahead of the OAuth buttons.',
			preview: {
				logins: [google, github],
				saml: true,
				smtpConfigured: true,
				lastUsed: { kind: 'saml' }
			}
		},
		{
			title: 'SAML only',
			note: 'A single SSO button, password login still reachable underneath.',
			preview: { saml: true }
		},
		{
			title: 'SSO only, password login disabled',
			note: 'No password form and no "Log in without third-party" escape hatch.',
			preview: { logins: [okta], saml: true, disablePasswordLogin: true }
		},
		{
			title: 'Cloud',
			note: 'isCloudHosted(): adds the contact line above the form and the terms/privacy footer.',
			preview: { logins: [google, github, microsoft], cloud: true, smtpConfigured: true }
		},
		{
			title: 'Auto-login redirecting',
			note: 'auto_login is set: everything is hidden behind "Signing you in…" while the redirect happens.',
			preview: { logins: [okta], autoRedirecting: true }
		}
	]
</script>

{#if enabled}
	<div class="p-4 bg-surface-secondary min-h-screen">
		<div class="flex flex-wrap items-end gap-4 mb-4">
			<div>
				<h1 class="text-lg font-semibold text-emphasis">Login page states</h1>
				<p class="text-xs text-secondary">
					Real cards, fixed instance config, no API calls. Sign in always fails so the error state
					is one click away (click twice for the shake).
				</p>
			</div>
			<div class="w-56">
				<div class="text-xs font-semibold text-emphasis mb-1">Card width</div>
				<Select items={widths} bind:value={width} />
			</div>
			<DarkModeToggle forcedDarkMode={false} />
		</div>

		<div class="grid grid-cols-1 2xl:grid-cols-2 gap-4">
			{#each variants as variant (variant.title)}
				{@const hasThirdParty = (variant.preview.logins?.length ?? 0) > 0 || !!variant.preview.saml}
				<div class="border rounded-lg overflow-hidden bg-surface-secondary">
					<div class="px-4 py-2 border-b bg-surface">
						<div class="text-sm font-semibold text-emphasis">{variant.title}</div>
						<div class="text-xs text-secondary">{variant.note}</div>
					</div>
					<div class="py-10 px-4">
						<div class="sm:mx-auto sm:w-full {width}">
							<div class="mx-auto flex justify-center">
								<WindmillIcon height="48px" width="48px" spin="slow" />
							</div>
							<div class="mt-6">
								<LoginHeading {hasThirdParty} />
							</div>
						</div>
						<div class="mt-6 sm:mx-auto sm:w-full {width}">
							<Login
								preview={variant.preview}
								firstTime={variant.firstTime ?? false}
								email={variant.email}
								autoRedirect={false}
							/>
						</div>
					</div>
				</div>
			{/each}
		</div>
	</div>
{/if}
