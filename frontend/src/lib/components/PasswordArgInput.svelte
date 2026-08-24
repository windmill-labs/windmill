<script lang="ts">
	import { VariableService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { generateRandomString } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { Button } from './common'
	import Password from './Password.svelte'
	import { untrack } from 'svelte'

	interface Props {
		value?: string | undefined
		disabled: boolean
		minRows?: number
		/** Workspace the ephemeral secret is minted in; defaults to the nav workspace.
		 * Session editors pass their acting workspace. */
		workspace?: string | undefined
	}

	let { value = $bindable(undefined), disabled, minRows, workspace }: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	let path = $state('')
	// Workspace the variable at `path` actually lives in; `ws` can move away from it.
	let mintedIn = $state<string | undefined>(undefined)
	// What the field mints from: an argument already holding a `$var:` ref has nothing to mint.
	function plaintextOf(v: unknown): string {
		return typeof v === 'string' && v !== '' && !v.startsWith('$var:') ? v : ''
	}
	let password = $state(plaintextOf(value))

	// The argument no longer holds what this field would mint from — a parent can replace the whole
	// args object without remounting it (previewing a saved input, say). Minting now would describe a
	// secret the argument does not point at, and binding it would discard the replacement.
	let argReplaced = $derived(path !== '' && value !== '$var:' + path)

	let isGenerating = false

	let userPrefix = $derived(
		'u/' + ($userStore?.username ?? $userStore?.email)?.split('@')[0] + '/secret_arg/'
	)
	async function generateValue() {
		if (isGenerating || argReplaced) return
		isGenerating = true
		const mintWs = ws!
		const boundBefore = value
		try {
			let npath = userPrefix + generateRandomString(12)
			let nvalue = '$var:' + npath
			await VariableService.createVariable({
				workspace: mintWs,
				requestBody: {
					value: password,
					is_secret: true,
					path: npath,
					description: 'Ephemeral secret variable',
					expires_at: new Date(Date.now() + 1000 * 60 * 60 * 24 * 7).toISOString()
				}
			})
			// The arg can be replaced the same way while the create is in flight. Nothing ever
			// referenced the variable just minted, so delete it; it expires on its own if that fails.
			if (value !== boundBefore) {
				VariableService.deleteVariable({ workspace: mintWs, path: npath }).catch(() => {})
				return
			}
			path = npath
			mintedIn = mintWs
			console.log('generated', nvalue)
			value = nvalue
			debouncedUpdate()
		} finally {
			// Ended without binding: discarded just above, or the create failed after the argument
			// moved. The field would otherwise keep showing a secret the argument does not hold, and
			// the mint effect tracks `ws` — a workspace move would bind that stale plaintext over the
			// replacement. Re-seeding leaves the field describing the argument again.
			if (path === '' && value !== boundBefore) {
				password = plaintextOf(value)
			}
			isGenerating = false
		}
	}

	async function updateValue() {
		// The first keystroke queues an update before anything is minted: letting it run would 404 and
		// retry the mint, binding over an argument that was replaced while the first mint was in flight.
		if (path === '') return
		const updating = path
		try {
			await VariableService.updateVariable({
				workspace: mintedIn ?? ws!,
				path: path,
				requestBody: {
					value: password
				}
			})
		} catch (e) {
			// A re-mint can bind a fresh variable while this update is in flight; recovering then
			// would orphan the one it just bound.
			if (path !== updating) return
			generateValue().catch((e) =>
				sendUserToast(`Could not create the secret: ${e?.body ?? e?.message ?? e}`, true)
			)
		}
	}

	let timeout: number | undefined = undefined
	function debouncedUpdate() {
		timeout && clearTimeout(timeout)
		timeout = setTimeout(updateValue, 500)
	}

	$effect(() => {
		password && untrack(() => debouncedUpdate())
	})

	$effect(() => {
		ws &&
			($userStore?.username || $userStore?.email) &&
			path == '' &&
			password != '' &&
			untrack(() =>
				// A failed mint leaves the plaintext bound to nothing and the argument empty. Only a
				// further keystroke re-runs this, so say so rather than submitting the job without it.
				generateValue().catch((e) =>
					sendUserToast(`Could not create the secret: ${e?.body ?? e?.message ?? e}`, true)
				)
			)
	})

	// The operating workspace can move after minting (a session forking, say), leaving the
	// variable behind where the job will not find it: mint a fresh one in the new workspace.
	// Bounded to a live instance: a field mounted onto an existing `$var:` holds neither the
	// plaintext nor the workspace it was minted in, so it can only be moved by retyping it.
	$effect(() => {
		const cur = ws
		if (!cur || path === '' || password === '' || mintedIn === cur || argReplaced) return
		untrack(() =>
			generateValue().catch((e) =>
				sendUserToast(`Could not create the secret in ${cur}: ${e?.body ?? e?.message ?? e}`, true)
			)
		)
	})
</script>

{#if value?.startsWith('$var:') && !value.startsWith('$var:' + userPrefix)}
	<div class="flex items-center gap-2 text-sm text-primary">
		Linked to static variable
		<Button
			size="xs"
			variant="default"
			onclick={() => {
				value = ''
			}}
		>
			Reset variable link
		</Button>
	</div>
{:else}
	<Password {disabled} {minRows} bind:password />
{/if}
