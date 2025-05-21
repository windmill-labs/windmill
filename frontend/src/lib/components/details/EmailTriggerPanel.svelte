<script lang="ts">
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import UserSettings from '../UserSettings.svelte'
	import { generateRandomString } from '$lib/utils'
	import HighlightTheme from '../HighlightTheme.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import { SettingService } from '$lib/gen'
	import Skeleton from '../common/skeleton/Skeleton.svelte'
	import TriggerTokens from '../triggers/TriggerTokens.svelte'
	import Description from '../Description.svelte'
	import Section from '../Section.svelte'
	import { createEventDispatcher } from 'svelte'
	import EmailTriggerConfigSection from './EmailTriggerConfigSection.svelte'

	let userSettings: UserSettings

	const dispatch = createEventDispatcher()

	export let token: string
	export let scopes: string[] = []
	export let isFlow: boolean = false
	export let hash: string | undefined = undefined
	export let path: string

	let emailDomain: string | null = null
	let triggerTokens: TriggerTokens | undefined = undefined

	let loading = true
	async function getEmailDomain() {
		emailDomain =
			((await SettingService.getGlobal({
				key: 'email_domain'
			})) as any) ?? 'mail.test.com'
		loading = false
	}

	getEmailDomain()

	$: emailDomain && dispatch('email-domain', emailDomain)
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

<Section label="Email trigger" class="flex flex-col gap-4">
	<Description link="https://www.windmill.dev/docs/advanced/email_triggers">
		Email triggers execute scripts and flows when emails are sent to specific addresses. Each
		trigger has its own unique email address that can be used to invoke the script or flow.
	</Description>
	{#if loading}
		<Skeleton layout={[[18]]} />
	{:else}
		{#if emailDomain}
			<EmailTriggerConfigSection {hash} {token} {path} {isFlow} {userSettings} {emailDomain} />
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
