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
	let password = $state(
		value && typeof value === 'string' && !value.startsWith('$var:') ? value : ''
	)

	let isGenerating = false

	let userPrefix = $derived(
		'u/' + ($userStore?.username ?? $userStore?.email)?.split('@')[0] + '/secret_arg/'
	)
	async function generateValue() {
		if (isGenerating) return
		isGenerating = true
		const mintWs = ws!
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
			path = npath
			mintedIn = mintWs
			console.log('generated', nvalue)
			value = nvalue
			debouncedUpdate()
		} finally {
			isGenerating = false
		}
	}

	async function updateValue() {
		try {
			await VariableService.updateVariable({
				workspace: mintedIn ?? ws!,
				path: path,
				requestBody: {
					value: password
				}
			})
		} catch (e) {
			generateValue()
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
			untrack(() => generateValue())
	})

	// The operating workspace can move after minting (a session forking, say), leaving the
	// variable behind where the job will not find it: mint a fresh one in the new workspace.
	// Bounded to a live instance: a field mounted onto an existing `$var:` holds neither the
	// plaintext nor the workspace it was minted in, so it can only be moved by retyping it.
	$effect(() => {
		const cur = ws
		if (!cur || path === '' || password === '' || mintedIn === cur) return
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
