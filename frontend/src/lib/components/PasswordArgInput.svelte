<script lang="ts">
	import { VariableService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { generateRandomString } from '$lib/utils'
	import Password from './Password.svelte'

	export let value: string | undefined = undefined
	export let disabled: boolean

	let path = ''
	let password = value && typeof value === 'string' && !value.startsWith('$var:') ? value : ''

	let isGenerating = false
	async function generateValue() {
		if (isGenerating) return
		isGenerating = true
		try {
			let npath =
				'u/' +
				($userStore?.username ?? $userStore?.email)?.split('@')[0] +
				'/secret_arg/' +
				generateRandomString(12)
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

	let timeout: NodeJS.Timeout | undefined = undefined
	function debouncedUpdate() {
		timeout && clearTimeout(timeout)
		timeout = setTimeout(updateValue, 500)
	}

	$: password && debouncedUpdate()

	$: $workspaceStore &&
		($userStore?.username || $userStore?.email) &&
		path == '' &&
		password != '' &&
		generateValue()
</script>

<Password {disabled} bind:password />
