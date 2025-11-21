<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { AppService, OpenAPI, type AppWithLastVersion } from '$lib/gen'
	import { userStore } from '$lib/stores'

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

	function parseSecret(secret: string): { secret: string; jwt: string } {
		const parts = secret.split('/')
		return {
			secret: parts[0],
			jwt: parts[1]
		}
	}

	const parsedSecret = parseSecret(page.params.secret ?? '')

	async function loadApp() {
		try {
			app = await AppService.getPublicAppBySecret({
				workspace: page.params.workspace ?? '',
				path: parsedSecret.secret
			})
			noPermission = false
			notExists = false
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
		loadAll()
	}

	function loadAll() {
		console.log('loadAll')
		loadUser().then(() => {
			loadApp()
		})
	}

	async function loadUser() {
		if (parsedSecret.jwt) {
			const token = 'jwt_ext_' + parsedSecret.jwt
			OpenAPI.TOKEN = token
			setContext<{ token?: string }>('AuthToken', { token })
			jwtError = false
		}
		try {
			userStore.set(await getUserExt(page.params.workspace ?? ''))
			if (!$userStore && parsedSecret.jwt) {
				jwtError = true
				sendUserToast('Could not authentify user with jwt token', true)
			}
		} catch (e) {
			console.warn('Anonymous user')
		}
	}
</script>

<PublicApp
	{app}
	workspace={page.params.workspace}
	{notExists}
	{noPermission}
	{jwtError}
	onLoginSuccess={() => {
		loadAll()
	}}
></PublicApp>
