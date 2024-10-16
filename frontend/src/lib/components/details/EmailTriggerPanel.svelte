<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { Button } from '$lib/components/common'
	import { SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON } from '$lib/consts'
	import UserSettings from '../UserSettings.svelte'
	import ClipboardPanel from './ClipboardPanel.svelte'
	import { generateRandomString } from '$lib/utils'
	import HighlightTheme from '../HighlightTheme.svelte'
	import Alert from '../common/alert/Alert.svelte'
	import { SettingService } from '$lib/gen'
	import { base32 } from 'rfc4648'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { AlertTriangle } from 'lucide-svelte'
	import Skeleton from '../common/skeleton/Skeleton.svelte'
	import Label from '$lib/components/Label.svelte'
	import TriggerTokens from '../triggers/TriggerTokens.svelte'
	let userSettings: UserSettings

	export let token: string
	export let scopes: string[] = []
	export let isFlow: boolean = false
	export let hash: string | undefined = undefined
	export let path: string

	let emailDomain: string | null = null

	let loading = true
	async function getEmailDomain() {
		emailDomain =
			((await SettingService.getGlobal({
				key: 'email_domain'
			})) as any) ?? null
		loading = false
	}

	getEmailDomain()

	let requestType: 'hash' | 'path' = 'path'

	function emailAddress() {
		const pathOrHash = requestType === 'hash' ? hash : path.replaceAll('/', '.')
		const plainPrefix = `${$workspaceStore}+${
			(requestType === 'hash' ? 'hash.' : isFlow ? 'flow.' : '') + pathOrHash
		}+${token}`
		const encodedPrefix = base32
			.stringify(new TextEncoder().encode(plainPrefix), {
				pad: false
			})
			.toLowerCase()
		return `${pathOrHash}+${encodedPrefix}@${emailDomain}`
	}

	export let email: string = ''

	$: email = emailAddress()

	let triggerTokens: TriggerTokens | undefined = undefined
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

<div class="flex flex-col w-full gap-4">
	{#if loading}
		<Skeleton layout={[[18]]} />
	{:else}
		{#if emailDomain}
			{#if SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON}
				<Label label="Token">
					<div class="flex flex-row justify-between gap-2">
						<input
							bind:value={token}
							placeholder="paste your token here once created to alter examples below"
							class="!text-xs"
						/>
						<Button size="xs" color="light" variant="border" on:click={userSettings.openDrawer}>
							Create an Email-specific Token
							<Tooltip light>
								The token will have a scope such that it can only be used to trigger this script. It
								is safe to share as it cannot be used to impersonate you.
							</Tooltip>
						</Button>
					</div>
					{#if token === 'TOKEN_TO_CREATE'}
						<div class="flex flex-row gap-1 text-xs text-red-500 items-center mt-1">
							<AlertTriangle size="12" />
							Create/input a valid token before copying the email address below
						</div>
					{/if}
				</Label>
			{/if}

			{#if !isFlow}
				<div class="flex flex-col gap-2">
					<div class="flex flex-row justify-between">
						<div class="text-xs font-semibold flex flex-row items-center">Call method</div>
						<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={requestType}>
							<ToggleButton label="By path" value="path" />
							<ToggleButton label="By hash" value="hash" />
						</ToggleButtonGroup>
					</div>
				</div>
			{/if}

			{#key requestType}
				{#key token}
					<Label label="Email address">
						<ClipboardPanel content={email} />
					</Label>
				{/key}
			{/key}
			<Alert title="Email trigger" size="xs">
				To trigger the job by email, send an email to the address above. The job will receive two
				arguments: `raw_email` containing the raw email as string, and `parsed_email` containing the
				parsed email as an object.
			</Alert>
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
</div>
