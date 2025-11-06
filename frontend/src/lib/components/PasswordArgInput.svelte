<script lang="ts">
	import { VariableService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { generateRandomString } from '$lib/utils'
	import { Button } from './common'
	import Password from './Password.svelte'
	import { untrack } from 'svelte'

	interface Props {
		value?: string | undefined
		disabled: boolean
	}

	let { value = $bindable(undefined), disabled }: Props = $props()

	let path = $state('')
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
		try {
			let npath = userPrefix + generateRandomString(12)
			let nvalue = '$var:' + npath
			await VariableService.createVariable({
				workspace: $workspaceStore!,
				requestBody: {
					value: password,
					is_secret: true,
					path: npath,
					description: 'Ephemeral secret variable',
					expires_at: new Date(Date.now() + 1000 * 60 * 60 * 24 * 7).toISOString()
				}
			})
			path = npath
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
				workspace: $workspaceStore!,
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
		$workspaceStore &&
			($userStore?.username || $userStore?.email) &&
			path == '' &&
			password != '' &&
			untrack(() => generateValue())
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
	<Password {disabled} bind:password />
{/if}
