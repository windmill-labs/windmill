<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { AppService, OpenAPI, type AppWithLastVersion } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'

	import { setContext } from 'svelte'
	import { setLicense } from '$lib/enterpriseUtils'

	import { getUserExt } from '$lib/user'
	import { sendUserToast } from '$lib/toast'
	import { page } from '$app/state'
	import PublicApp from '$lib/components/apps/editor/PublicApp.svelte'

	let app: (AppWithLastVersion & { value: any }) | undefined = $state(undefined)
	let notExists = $state(false)
	let noPermission = $state(false)
	let jwtError = $state(false)

	function isJwt(t: string) {
		// simply check that the first part is a valid base64 encoded json
		try {
			const parts = t.split('.')
			const header = atob(parts[0])
			JSON.parse(header)
			return true
		} catch (e) {
			return false
		}
	}

	function parseCustomPath(customPath: string): { path: string; jwt: string | undefined } {
		const parts = customPath.split('/')
		if (parts.length > 1 && isJwt(parts[parts.length - 1])) {
			return {
				path: parts.slice(0, -1).join('/'),
				jwt: parts[parts.length - 1]
			}
		} else {
			return {
				path: customPath,
				jwt: undefined
			}
		}
	}

	let workspace: string | undefined = $state(undefined)
	async function loadApp() {
		const parsedCustomPath = parseCustomPath(page.params.path ?? '')

		if (parsedCustomPath.jwt) {
			const token = 'jwt_ext_' + parsedCustomPath.jwt
			OpenAPI.TOKEN = token
			setContext<{ token?: string }>('AuthToken', { token })
			jwtError = false
		}
		try {
			app = await AppService.getPublicAppByCustomPath({
				customPath: parsedCustomPath.path
			})
			workspace = app.workspace_id
			workspaceStore.set(app.workspace_id)
			noPermission = false
			notExists = false

			try {
				userStore.set(await getUserExt(app.workspace_id))
				if (!$userStore && parsedCustomPath.jwt) {
					jwtError = true
					sendUserToast('Could not authentify user with jwt token', true)
				}
			} catch (e) {
				console.warn('Anonymous user')
			}
		} catch (e) {
			if (e.status == 401) {
				noPermission = true
			} else {
				notExists = true
			}
		}
	}

	if (BROWSER) {
		setLicense()
		loadApp()
	}
</script>

<PublicApp
	{workspace}
	{notExists}
	{noPermission}
	{jwtError}
	{app}
	onLoginSuccess={() => {
		loadApp()
	}}
></PublicApp>
