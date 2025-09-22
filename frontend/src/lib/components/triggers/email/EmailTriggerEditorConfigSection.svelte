<script lang="ts">
	import { Alert } from '$lib/components/common'
	import Required from '$lib/components/Required.svelte'
	import Section from '$lib/components/Section.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	// import { page } from '$app/stores'
	import { getEmailAddress, getEmailDomain } from './utils'
	import { isCloudHosted } from '$lib/cloud'
	import Toggle from '$lib/components/Toggle.svelte'
	import TestingBadge from '../testingBadge.svelte'
	import { untrack } from 'svelte'
	import { EmailTriggerService } from '$lib/gen'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	interface Props {
		initialTriggerPath?: string | undefined
		dirtyLocalPart?: boolean
		local_part: string | undefined
		can_write?: boolean
		headless?: boolean
		workspaced_local_part?: boolean
		isValid?: boolean
		isDraftOnly?: boolean
		showTestingBadge?: boolean
	}

	let {
		initialTriggerPath = undefined,
		dirtyLocalPart = $bindable(false),
		local_part = $bindable(),
		can_write = false,
		headless = false,
		workspaced_local_part = $bindable(false),
		isValid = $bindable(false),
		isDraftOnly = true,
		showTestingBadge = false
	}: Props = $props()

	let validateTimeout: number | undefined = undefined

	let addressError: string = $state('')
	async function validateEmailAddress(
		localPart: string | undefined,
		workspaced_local_part: boolean
	): Promise<void> {
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(async () => {
			if (!localPart || !/^[a-z0-9._]{1,64}$/.test(localPart)) {
				addressError =
					'Local part not valid, only accepts lowercase alphanumeric characters, dots and underscores, and must be between 1 and 64 characters'
			} else if (await emailTriggerExists(localPart, workspaced_local_part)) {
				addressError = 'Email address already taken'
			} else {
				addressError = ''
			}
			validateTimeout = undefined
		}, 500)
	}
	async function emailTriggerExists(local_part: string, workspaced_local_part: boolean) {
		return await EmailTriggerService.existsEmailLocalPart({
			workspace: $workspaceStore!,
			requestBody: {
				local_part,
				trigger_path: initialTriggerPath,
				workspaced_local_part: workspaced_local_part
			}
		})
	}

	$effect.pre(() => {
		;[local_part, workspaced_local_part]
		untrack(() => {
			validateEmailAddress(local_part, workspaced_local_part)
		})
	})

	$effect.pre(() => {
		isValid = addressError === ''
	})

	let emailDomain: string | null = $state(null)
	getEmailDomain().then((domain) => {
		emailDomain = domain
	})

	let fullEmailAddress = $derived(
		getEmailAddress(local_part, workspaced_local_part, $workspaceStore ?? '', emailDomain ?? '')
	)

	$effect.pre(() => {
		local_part === undefined && (local_part = '')
	})

	let userIsAdmin = $derived($userStore?.is_admin || $userStore?.is_super_admin)
	let userCanEditConfig = $derived(userIsAdmin || isDraftOnly) // User can edit config if they are admin or if the trigger is a draft which will not be saved
</script>

<div>
	<Section label="Email" {headless}>
		{#snippet header()}
			{#if showTestingBadge}
				<TestingBadge />
			{/if}
		{/snippet}
		{#if !userCanEditConfig && isDraftOnly}
			<Alert type="info" title="Admin only" collapsible size="xs">
				Email triggers can only be edited by workspace admins
			</Alert>
			<div class="my-2"></div>
		{/if}
		<div class="flex flex-col w-full gap-4">
			<label class="block grow w-full">
				<div class="flex flex-col gap-1">
					<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
						<div>
							Local part
							<Required required={true} />
						</div>
					</div>
					<!-- svelte-ignore a11y_autofocus -->
					<input
						type="text"
						autocomplete="off"
						bind:value={local_part}
						disabled={!userCanEditConfig || !can_write}
						class={addressError === ''
							? ''
							: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
						oninput={() => {
							dirtyLocalPart = true
						}}
					/>
				</div>
			</label>

			<div class="flex flex-col w-full">
				<ClipboardPanel content={fullEmailAddress} />

				<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5"
					>{dirtyLocalPart ? addressError : ''}</div
				>
				{#if !isCloudHosted()}
					<div class="mt-1">
						<Toggle
							size="sm"
							checked={workspaced_local_part}
							disabled={!can_write || !userCanEditConfig}
							on:change={() => {
								workspaced_local_part = !workspaced_local_part
								dirtyLocalPart = true
							}}
							options={{
								right: 'Prefix with workspace',
								rightTooltip:
									'Prefixes the email address with the workspace ID (e.g., ${workspace_id}-${local_part}@). Note: deploying the email trigger to another workspace updates the email address workspace prefix accordingly.',
								rightDocumentationLink:
									'https://www.windmill.dev/docs/advanced/email_triggers#workspace-prefix'
							}}
						/>
					</div>
				{/if}
			</div>
		</div>
	</Section>
</div>
