<script lang="ts">
	import { Alert } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { Loader2 } from 'lucide-svelte'

	import Tooltip from '$lib/components/Tooltip.svelte'

	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import { untrack } from 'svelte'
	import { AppService, SettingService } from '$lib/gen'
	import Path from '$lib/components/Path.svelte'
	import { computeSecretUrl } from './appDeploy.svelte'
	import { base } from '$lib/base'
	import { isCloudHosted } from '$lib/cloud'
	import EEOnly from '$lib/components/EEOnly.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import OnBehalfOfSelector, {
		type OnBehalfOfChoice
	} from '$lib/components/OnBehalfOfSelector.svelte'

	const WM_DEPLOYERS_GROUP = 'wm_deployers'

	let {
		policy,
		setPublishState,
		appPath,
		customPath = $bindable(),
		onLatest,
		savedApp,
		summary = $bindable(),
		deploymentMsg = $bindable(),
		customPathError = $bindable(),
		pathError = $bindable(),
		newEditedPath = $bindable(),
		newPath,
		hideSecretUrl = false,
		preserveOnBehalfOf = $bindable(false)
	}: {
		policy: any
		setPublishState: () => void
		appPath: string
		customPath: string | undefined
		onLatest: boolean
		savedApp: any
		summary: string
		deploymentMsg: string | undefined
		customPathError: string
		pathError: string
		newEditedPath: string
		newPath: string
		hideSecretUrl?: boolean
		preserveOnBehalfOf?: boolean
	} = $props()

	let isDeployer = $derived($userStore?.groups?.includes(WM_DEPLOYERS_GROUP) ?? false)
	let canPreserve = $derived(
		!!$userStore?.is_admin || !!$userStore?.is_super_admin || isDeployer
	)
	let savedOnBehalfOfEmail = $derived(savedApp?.policy?.on_behalf_of_email)
	let onBehalfOfChoice: OnBehalfOfChoice = $state(undefined)
	let customOnBehalfOfEmail: string = $state('')
	let dirtyCustomPath = $state(false)
	let path: Path | undefined = $state(undefined)

	let dirtyPath = $state(false)

	async function appExists(customPath: string) {
		return await AppService.customPathExists({
			workspace: $workspaceStore!,
			customPath
		})
	}

	let globalWorkspacedRoute = $state(false)

	async function loadGlobalWorkspacedRouteSetting() {
		try {
			const setting = await SettingService.getGlobal({ key: 'app_workspaced_route' })
			globalWorkspacedRoute = (setting as boolean) ?? false
		} catch (error) {
			globalWorkspacedRoute = false
		}
	}

	loadGlobalWorkspacedRouteSetting()

	let secretUrl: string | undefined = $state(undefined)
	let secretUrlHref = $derived(secretUrl ? computeSecretUrl(secretUrl) : undefined)
	let fullCustomUrl = $derived(
		`${window.location.origin}${base}/a/${
			isCloudHosted() || globalWorkspacedRoute ? $workspaceStore + '/' : ''
		}${customPath}`
	)
	async function getSecretUrl() {
		secretUrl = await AppService.getPublicSecretOfApp({
			workspace: $workspaceStore!,
			path: appPath
		})
	}

	let validateTimeout: number | undefined = undefined
	async function validateCustomPath(customPath: string): Promise<void> {
		customPathError = ''
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(async () => {
			if (!/^[\w-]+(\/[\w-]+)*$/.test(customPath)) {
				customPathError = 'Invalid path'
			} else if (customPath !== savedApp?.custom_path && (await appExists(customPath))) {
				customPathError = 'Path already taken'
			} else {
				customPathError = ''
			}
			validateTimeout = undefined
		}, 500)
	}

	$effect(() => {
		;[customPath]
		untrack(() => customPath !== undefined && validateCustomPath(customPath))
	})

	$effect(() => {
		appPath && appPath != '' && secretUrl == undefined && untrack(() => getSecretUrl())
	})
</script>

{#if !onLatest}
	<Alert title="You're not on the latest app version. " type="warning">
		By deploying, you may overwrite changes made by other users. Press 'Deploy' to see diff.
	</Alert>
	<div class="py-2"></div>
{/if}
<label for="summary" class="text-emphasis text-xs font-semibold">Summary</label>
<div class="w-full pt-1">
	<!-- svelte-ignore a11y_autofocus -->
	<TextInput
		inputProps={{
			id: 'summary',
			autofocus: true,
			placeholder: 'App summary',
			onkeydown: (e) => {
				e.stopPropagation()
			},
			onkeyup: () => {
				if (appPath == '' && summary?.length > 0 && !dirtyPath) {
					path?.setName(
						summary
							.toLowerCase()
							.replace(/[^a-z0-9_]/g, '_')
							.replace(/-+/g, '_')
							.replace(/^-|-$/g, '')
					)
				}
			}
		}}
		bind:value={summary}
	/>
</div>
<div class="py-6"></div>
<label for="deploymentMsg" class="text-emphasis text-xs font-semibold">Deployment message</label>
<div class="w-full pt-1">
	<!-- svelte-ignore a11y_autofocus -->
	<TextInput
		inputProps={{
			id: 'deploymentMsg',
			placeholder: 'Optional deployment message'
		}}
		bind:value={deploymentMsg}
	/>
</div>
<div class="py-6"></div>
<label for="path" class="text-emphasis text-xs font-semibold">Path</label>
<Path
	bind:this={path}
	bind:dirty={dirtyPath}
	bind:error={pathError}
	bind:path={newEditedPath}
	initialPath={newPath}
	namePlaceholder="app"
	kind="app"
	autofocus={false}
/>

<div class="py-2"></div>
<Alert title="App executed on behalf of you">
	A viewer of the app will execute the runnables of the app on behalf of the publisher (you)
	<Tooltip>
		It ensures that all required resources/runnable visible for publisher but not for viewer at time
		of creating the app would prevent the execution of the app. To guarantee tight security, a
		policy is computed at time of deployment of the app which only allow the scripts/flows referred
		to in the app to be called on behalf of. Furthermore, static parameters are not overridable.
		Hence, users will only be able to use the app as intended by the publisher without risk for
		leaking resources not used in the app.
	</Tooltip>
	{#if canPreserve}
		<div class="mt-4">
			Because you are either an admin or part of the {WM_DEPLOYERS_GROUP} group, you can select another user to run this app on behalf of. Once deployed the app will be run on behalf of
			<OnBehalfOfSelector
				targetWorkspace={$workspaceStore ?? ''}
				targetEmail={savedOnBehalfOfEmail}
				selected={onBehalfOfChoice}
				onSelect={(choice, email, username) => {
					onBehalfOfChoice = choice
					if (choice === 'me') {
						policy.on_behalf_of_email = $userStore?.email
						policy.on_behalf_of = `u/${$userStore?.username}`
						customOnBehalfOfEmail = ''
						preserveOnBehalfOf = false
					} else if (choice === 'target') {
						policy.on_behalf_of_email = savedOnBehalfOfEmail
						customOnBehalfOfEmail = ''
						preserveOnBehalfOf = true
					} else if (choice === 'custom' && email) {
						policy.on_behalf_of_email = email
						policy.on_behalf_of = username ? `u/${username}` : undefined
						customOnBehalfOfEmail = email
						preserveOnBehalfOf = true
					}
				}}
				kind="app"
				{canPreserve}
				customEmail={customOnBehalfOfEmail}
				isDeployment={false}
			/>
		</div>
	{/if}
</Alert>


<div class="mt-10"></div>

{#if !hideSecretUrl}
	<h2>Public URL</h2>

	<div class="my-6">
		<div class="flex gap-2 items-center mb-2">
			<Toggle
				options={{
					left: `Require login and read-access`,
					right: `No login required`
				}}
				checked={policy.execution_mode == 'anonymous'}
				on:change={(e) => {
					policy.execution_mode = e.detail ? 'anonymous' : 'publisher'
					setPublishState()
				}}
				disabled={appPath == ''}
			/>
		</div>
		{#if appPath == ''}
			<ClipboardPanel content={`Save this app once to get the public secret URL`} size="md" />
		{:else if secretUrlHref}
			<ClipboardPanel content={secretUrlHref} size="md" />
		{:else}<Loader2 class="animate-spin" />
		{/if}
		<div class="text-xs text-secondary mt-1">
			Share this url directly or embed it using an iframe (if requiring login, top-level domain of
			embedding app must be the same as the one of Windmill)
		</div>

		<div class="mt-4">
			{#if !($userStore?.is_admin || $userStore?.is_super_admin)}
				<Alert type="warning" title="Admin only" size="xs">
					Custom path can only be set by workspace admins
				</Alert>
				<div class="mb-2"></div>
			{/if}
			<!-- svelte-ignore block_empty -->
			{#if !$enterpriseLicense}
				<EEOnly />
			{/if}
			<Toggle
				on:change={({ detail }) => {
					customPath = detail ? '' : undefined
					if (customPath === undefined) {
						customPathError = ''
					}
				}}
				checked={customPath !== undefined}
				options={{
					right: 'Use a custom URL'
				}}
				disabled={!$enterpriseLicense || !($userStore?.is_admin || $userStore?.is_super_admin)}
			/>

			{#if customPath !== undefined}
				<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
					<div>Custom path</div>
				</div>
				<input
					disabled={!($userStore?.is_admin || $userStore?.is_super_admin)}
					type="text"
					autocomplete="off"
					bind:value={customPath}
					class={customPathError === ''
						? ''
						: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
					oninput={() => {
						dirtyCustomPath = true
					}}
				/>
				<div class="text-secondary text-sm flex items-center gap-1 mt-2 w-full justify-between">
					<div>Custom public URL</div>
				</div>
				<ClipboardPanel content={fullCustomUrl} size="md" />

				<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5"
					>{dirtyCustomPath ? customPathError : ''}
				</div>
			{/if}
		</div>
	</div>
	<Alert type="info" title="Only latest deployed app is publicly available">
		You will still need to deploy the app to make visible the latest changes
	</Alert>

	<a
		href="https://www.windmill.dev/docs/advanced/external_auth_with_jwt#embed-public-apps-using-your-own-authentification"
		class="mt-4 text-2xs">Embed this app in your own product to be used by your own users</a
	>
{/if}
