<script lang="ts">
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import UserSettings from '../../UserSettings.svelte'
	import { generateRandomString } from '$lib/utils'
	import HighlightTheme from '../../HighlightTheme.svelte'
	import Alert from '../../common/alert/Alert.svelte'
	import Skeleton from '../../common/skeleton/Skeleton.svelte'
	import TriggerTokens from '../TriggerTokens.svelte'
	import Description from '../../Description.svelte'
	import Section from '../../Section.svelte'
	import DefaultEmailConfigSection from './DefaultEmailConfigSection.svelte'
	import { getEmailDomain } from './utils'

	let userSettings: UserSettings | undefined = $state(undefined)
	interface Props {
		token: string
		scopes?: string[]
		isFlow?: boolean
		runnableVersion?: string | undefined
		path: string
		onEmailDomain: (domain: string) => void
	}

	let {
		token = $bindable(),
		scopes = [],
		isFlow = false,
		runnableVersion = undefined,
		path,
		onEmailDomain
	}: Props = $props()

	let emailDomain: string | null = $state(null)
	let triggerTokens: TriggerTokens | undefined = $state(undefined)

	let loading = $state(true)

	getEmailDomain().then((domain) => {
		emailDomain = domain
		loading = false
	})

	$effect(() => {
		if (emailDomain) {
			onEmailDomain(emailDomain)
		}
	})
</script>

<HighlightTheme />

<UserSettings
	bind:this={userSettings}
	on:tokenCreated={(e) => {
		token = e.detail
		triggerTokens?.listTokens()
	}}
	newTokenWorkspace={$workspaceStore}
	newTokenLabel={`email-${$userStore?.username ?? 'superadmin'}-${generateRandomString(4)}`}
	{scopes}
/>

<Section label="Default email trigger" class="flex flex-col gap-6">
	<Description link="https://www.windmill.dev/docs/advanced/email_triggers">
		Default email trigger is a partially fixed email address that can be used to trigger a script or
		flow. The email address is composed of the encoded workspace and script or flow path as well as
		the token.
	</Description>
	{#if loading}
		<Skeleton layout={[[18]]} />
	{:else}
		{#if emailDomain}
			<DefaultEmailConfigSection {runnableVersion} {token} {path} {isFlow} {userSettings} {emailDomain} />
		{:else}
			<div>
				<Alert title="Email triggers are disabled" size="xs" type="warning">
					Ask an instance superadmin to setup the instance for email triggering (<a
						target="_blank"
						href="https://windmill.dev/docs/advanced/email_triggers">docs</a
					>) and to set the email domain in the instance settings.
				</Alert>
			</div>
		{/if}

		{#if !$enterpriseLicense}
			<Alert title="Community Edition limitations" type="warning" size="xs">
				Email triggers on Windmill Community Edition are limited to 100 emails per day.
			</Alert>
		{/if}

		<TriggerTokens bind:this={triggerTokens} {isFlow} {path} labelPrefix="email" />
	{/if}
</Section>
