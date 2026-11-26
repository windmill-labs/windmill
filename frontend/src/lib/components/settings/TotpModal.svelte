<script module lang="ts">
	export type TotpAction = 'setup' | 'recovery_codes' | 'disable'
</script>

<script lang="ts">
	import { UserService } from '$lib/gen'
	import { Alert, Button, CopyButton } from '$lib/components/common'
	import Modal from '../common/modal/Modal.svelte'
	import Stepper from '../common/stepper/Stepper.svelte'
	import Checkbox from '../common/checkbox/Checkbox.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import CopyableCodeBlock from '../details/CopyableCodeBlock.svelte'
	import { plaintext } from 'svelte-highlight/languages'
	import { sendUserToast } from '$lib/toast'
	import { download } from '$lib/utils'
	import { renderSVG } from 'uqr'

	interface Props {
		action: TotpAction | undefined
		onClose: () => void
	}

	let { action = $bindable(), onClose }: Props = $props()

	const SETUP_STEPS = ['Password', 'Authenticator', 'Recovery codes']

	let step = $state(0)
	let password = $state('')
	let code = $state('')
	let setup = $state<{ secret: string; otpauth_url: string } | undefined>(undefined)
	let recoveryCodes = $state<string[] | undefined>(undefined)
	let saved = $state(false)
	let busy = $state(false)

	const title = $derived(
		action == 'setup'
			? 'Set up two-factor authentication'
			: action == 'recovery_codes'
				? 'New recovery codes'
				: 'Turn off two-factor authentication'
	)

	// Closing at any step drops the secret and the codes from the page.
	function close() {
		step = 0
		password = ''
		code = ''
		setup = undefined
		recoveryCodes = undefined
		saved = false
		action = undefined
		onClose()
	}

	const canContinue = $derived(
		recoveryCodes ? saved : action == 'setup' && step == 0 ? !!password : !!code
	)

	async function next() {
		if (!canContinue || busy) return
		busy = true
		try {
			if (recoveryCodes) {
				close()
			} else if (action == 'setup' && step == 0) {
				setup = await UserService.setupTotp({ requestBody: { password } })
				password = ''
				step = 1
			} else if (action == 'disable') {
				sendUserToast(await UserService.disableTotp({ requestBody: { code } }))
				close()
			} else {
				const requestBody = { code }
				recoveryCodes = (
					action == 'setup'
						? await UserService.enableTotp({ requestBody })
						: await UserService.regenerateTotpRecoveryCodes({ requestBody })
				).recovery_codes
				setup = undefined
				code = ''
				step = 2
			}
		} finally {
			busy = false
		}
	}

	const nextLabel = $derived(
		recoveryCodes
			? 'Done'
			: action == 'setup'
				? step == 0
					? 'Continue'
					: 'Turn on'
				: action == 'disable'
					? 'Turn off'
					: 'Create new codes'
	)
</script>

{#snippet primary()}
	<Button unifiedSize="sm" variant="accent" disabled={!canContinue} loading={busy} onClick={next}>
		{nextLabel}
	</Button>
{/snippet}

<Modal
	{title}
	bind:open={() => action != undefined, (v) => !v && action != undefined && close()}
	on:confirmed={next}
	kind={recoveryCodes ? 'X' : 'button'}
>
	<div class="flex flex-col gap-4">
		{#if action == 'setup'}
			<Stepper tabs={SETUP_STEPS} selectedIndex={step} maxReachedIndex={step} small />
		{/if}

		{#if recoveryCodes}
			<div class="flex flex-col gap-1">
				<span class="text-xs text-secondary font-normal">
					Each code logs you in once if you lose your authenticator.
				</span>
				<CopyableCodeBlock code={recoveryCodes.join('\n')} language={plaintext} />
				<div class="flex">
					<Button
						unifiedSize="sm"
						variant="default"
						onClick={() => download('windmill-recovery-codes.txt', recoveryCodes!.join('\n'))}
					>
						Download
					</Button>
				</div>
			</div>
			<Alert type="warning" title="Save these codes now" size="xs">
				They are shown only once. Keep them somewhere other than this device.
			</Alert>
			<label class="flex flex-row gap-2 items-center text-xs text-primary font-normal">
				<Checkbox checked={saved} onChange={() => (saved = !saved)} />
				I saved my recovery codes
			</label>
			<div class="flex justify-end">
				{@render primary()}
			</div>
		{:else if action == 'setup' && step == 0}
			<label class="flex flex-col gap-1">
				<span class="text-xs text-emphasis font-semibold">Password</span>
				<span class="text-xs text-secondary font-normal">
					Confirm it's you before adding an authenticator to this account.
				</span>
				<TextInput
					inputProps={{ type: 'password', autocomplete: 'current-password' }}
					bind:value={password}
				/>
			</label>
		{:else}
			{#if setup}
				<div class="flex flex-col gap-1">
					<span class="text-xs text-secondary font-normal">
						Scan this QR code with an authenticator app, or open it on this device.
					</span>
					<a
						href={setup.otpauth_url}
						aria-label="Add to an authenticator app"
						class="w-40 h-40 bg-white p-1 rounded-md"
					>
						{@html renderSVG(setup.otpauth_url)}
					</a>
					<div class="flex flex-row gap-1 items-center text-2xs text-secondary font-normal">
						Or enter this key manually:
						<span class="font-mono text-emphasis">{setup.secret}</span>
						<CopyButton value={setup.secret} />
					</div>
				</div>
			{/if}
			<label class="flex flex-col gap-1">
				<span class="text-xs text-emphasis font-semibold">Code</span>
				<span class="text-xs text-secondary font-normal">
					{setup
						? 'The 6-digit code your authenticator app shows'
						: 'A code from your authenticator app, or one of your recovery codes'}
				</span>
				<TextInput inputProps={{ autocomplete: 'one-time-code', name: 'totp' }} bind:value={code} />
			</label>
		{/if}
	</div>

	{#snippet actions()}
		{@render primary()}
	{/snippet}
</Modal>
