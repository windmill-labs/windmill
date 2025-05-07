<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { AlertTriangle } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import { SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON } from '$lib/consts'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ClipboardPanel from './ClipboardPanel.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { base32 } from 'rfc4648'
	import CaptureSection, { type CaptureInfo } from '../triggers/CaptureSection.svelte'
	import CaptureTable from '../triggers/CaptureTable.svelte'

	export let token: string = ''
	export let isFlow: boolean = false
	export let hash: string | undefined = undefined
	export let path: string
	export let userSettings: any
	export let emailDomain: string | null = null
	export let showCapture: boolean = false
	export let captureInfo: CaptureInfo | undefined = undefined
	export let captureTable: CaptureTable | undefined = undefined

	let requestType: 'hash' | 'path' = 'path'

	function getCaptureEmail() {
		const cleanedPath = path.replaceAll('/', '.')
		const plainPrefix = `capture+${$workspaceStore}+${(isFlow ? 'flow.' : '') + cleanedPath}`
		const encodedPrefix = base32
			.stringify(new TextEncoder().encode(plainPrefix), {
				pad: false
			})
			.toLowerCase()
		return `${encodedPrefix}@${emailDomain}`
	}

	$: captureEmail = getCaptureEmail()

	function emailAddress(
		requestType: 'hash' | 'path',
		path: string,
		hash: string | undefined,
		isFlow: boolean,
		token: string
	) {
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

	$: email = emailAddress(requestType, path, hash, isFlow, token)
</script>

<div>
	{#if showCapture && captureInfo}
		<CaptureSection
			bind:captureTable
			captureType="email"
			disabled={false}
			{captureInfo}
			on:captureToggle
			on:applyArgs
			on:updateSchema
			on:addPreprocessor
			on:testWithArgs
		>
			<Label label="Email address">
				<ClipboardPanel content={captureEmail} disabled={!captureInfo.active} />
			</Label>
		</CaptureSection>
	{/if}
	<div class="flex flex-col gap-4">
		{#if SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON}
			<Label label="Token">
				<div class="flex flex-row justify-between gap-2">
					<input
						bind:value={token}
						placeholder="paste your token here once created to alter examples below"
						class="!text-xs"
					/>
					<Button
						size="xs"
						color="light"
						variant="border"
						on:click={() => userSettings.openDrawer()}
					>
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
					<ToggleButtonGroup class="h-[30px] w-auto" bind:selected={requestType} let:item>
						<ToggleButton label="By path" value="path" {item} />
						<ToggleButton label="By hash" value="hash" {item} />
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
	</div>
</div>
