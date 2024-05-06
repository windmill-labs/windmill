<script lang="ts">
	import { VariableService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { generateRandomString } from '$lib/utils'
	import Password from './Password.svelte'

	export let value: string | undefined = undefined
	export let disabled: boolean

	let path = ''
	let password = ''

	async function generateValue() {
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
				description: ''
			}
		})
		path = npath
		value = nvalue
	}

	async function updateValue() {
		await VariableService.updateVariable({
			workspace: $workspaceStore!,
			path: path,
			requestBody: {
				value: password
			}
		})
	}

	let timeout: NodeJS.Timeout | undefined = undefined
	function debouncedUpdate() {
		timeout && clearTimeout(timeout)
		setTimeout(updateValue, 500)
	}

	$: password && debouncedUpdate()

	$: $workspaceStore &&
		($userStore?.username || $userStore?.email) &&
		path == '' &&
		password != '' &&
		generateValue()
</script>

<Password {disabled} bind:password />
