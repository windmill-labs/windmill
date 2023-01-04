<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { Alert, Drawer, DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import ToggleButton from '$lib/components/common/toggleButton/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton/ToggleButtonGroup.svelte'
	import Path from '$lib/components/Path.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { AppService, Policy } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { faClipboard, faExternalLink, faSave } from '@fortawesome/free-solid-svg-icons'
	import {
		AlignHorizontalSpaceAround,
		Expand,
		Eye,
		Laptop2,
		Pencil,
		Smartphone
	} from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import { copyToClipboard, sendUserToast } from '../../../utils'
	import type { AppComponent, AppEditorContext } from '../types'
	import AppExportButton from './AppExportButton.svelte'

	async function hash(message) {
		const msgUint8 = new TextEncoder().encode(message) // encode as (utf-8) Uint8Array
		const hashBuffer = await crypto.subtle.digest('SHA-256', msgUint8) // hash the message
		const hashArray = Array.from(new Uint8Array(hashBuffer)) // convert buffer to byte array
		const hashHex = hashArray.map((b) => b.toString(16).padStart(2, '0')).join('') // convert bytes to hex string
		return hashHex
	}

	export let policy: Policy

	const { app, summary, mode, breakpoint, appPath } =
		getContext<AppEditorContext>('AppEditorContext')
	const loading = {
		publish: false,
		save: false
	}

	let newPath: string = ''
	let pathError: string | undefined = undefined

	let saveDrawerOpen = false
	let publishDrawerOpen = false

	function closeSaveDrawer() {
		saveDrawerOpen = false
	}

	async function computeTriggerables() {
		const allTriggers = await Promise.all(
			$app.grid.map(async (x) => {
				let c = x.data as AppComponent
				if (c.componentInput?.type == 'runnable') {
					const staticInputs = Object.fromEntries(
						Object.entries(c.componentInput.fields ?? {})
							.filter(([k, v]) => v.type == 'static')
							.map(([k, v]) => {
								return [k, v['value']]
							})
					)
					if (c.componentInput.runnable?.type == 'runnableByName') {
						let hex = await hash(c.componentInput.runnable.inlineScript?.content)
						return [`rawscript/${hex}`, staticInputs]
					} else if (c.componentInput.runnable?.type == 'runnableByPath') {
						return [
							`${c.componentInput.runnable.runType}/${c.componentInput.runnable.path}`,
							staticInputs
						]
					}
				}
				return []
			})
		)
		policy.triggerables = Object.fromEntries(allTriggers)
		policy.on_behalf_of = `u/${$userStore?.username}`
		policy.on_behalf_of_email = $userStore?.email
	}
	async function createApp(path: string) {
		await computeTriggerables()
		try {
			const appId = await AppService.createApp({
				workspace: $workspaceStore!,
				requestBody: {
					value: $app,
					path,
					summary: $summary,
					policy
				}
			})
			goto(`/apps/edit/${appId}`)
		} catch (e) {
			sendUserToast('Error creating app', e)
		}
	}

	let secretUrl: string | undefined = undefined

	$: secretUrl == undefined && policy.execution_mode == 'anonymous' && getSecretUrl()

	async function getSecretUrl() {
		secretUrl = await AppService.getPublicSecretOfApp({
			workspace: $workspaceStore!,
			path: appPath
		})
	}

	async function setPublishState() {
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path: appPath,
			requestBody: { policy }
		})
		if (policy.execution_mode == 'anonymous') {
			sendUserToast('App made visible publicly at the secret URL.')
		} else {
			sendUserToast('App made unaccessible publicly')
		}
	}

	async function save() {
		$dirtyStore = false
		if ($page.params.path == undefined) {
			saveDrawerOpen = true
			return
		}
		loading.save = true
		await computeTriggerables()
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path: $page.params.path,
			requestBody: {
				value: $app!,
				summary: $summary,
				policy
			}
		})
		loading.save = false
		sendUserToast('App saved')
	}
</script>

<Drawer bind:open={saveDrawerOpen} size="800px">
	<DrawerContent title="Create an App" on:close={() => closeSaveDrawer()}>
		<Path
			bind:error={pathError}
			bind:path={newPath}
			initialPath=""
			namePlaceholder="my_app"
			kind="app"
		/>

		<div slot="actions">
			<Button
				startIcon={{ icon: faSave }}
				disabled={pathError != ''}
				on:click={() => createApp(newPath)}>Create app</Button
			>
		</div>
	</DrawerContent>
</Drawer>

<Drawer bind:open={publishDrawerOpen} size="800px">
	<DrawerContent title="Publish an App" on:close={() => (publishDrawerOpen = false)}>
		{#if appPath == ''}
			<Alert title="Require saving" type="error">Save this app once before you can publish it</Alert
			>
		{:else}
			<Alert title="App executed on behalf of publisher">
				A viewer of the app will execute the runnables of the app on behalf of the publisher
				avoiding the risk that a resource or script would not be available to the viewer. To
				guarantee tight security, a policy is computed at time of saving of the app which only allow
				the scripts/flows referred to in the app to be called on behalf of. Furthermore, static
				parameters are not overridable. Hence, users will only be able to use the app as intended by
				the publisher without risk for leaking resources not used in the app.</Alert
			>
			<div class="mt-4" />
			<Toggle
				options={{
					left: `Require read-access`,
					right: `Publish publicly for anyone knowing the secret url`
				}}
				checked={policy.execution_mode == 'anonymous'}
				on:change={(e) => {
					policy.execution_mode = e.detail
						? Policy.execution_mode.ANONYMOUS
						: Policy.execution_mode.PUBLISHER
					setPublishState()
				}}
			/>

			{#if policy.execution_mode == 'anonymous' && secretUrl}
				{@const url = `${$page.url.hostname}/public/${$workspaceStore}/${secretUrl}`}
				{@const href = $page.url.protocol + '//' + url}
				<div class="my-6 box">
					Public url:
					<a
						on:click={(e) => {
							e.preventDefault()
							copyToClipboard(href)
						}}
						{href}
						class="whitespace-nowrap text-ellipsis overflow-hidden mr-1"
					>
						{url}
						<span class="text-gray-700 ml-2">
							<Icon data={faClipboard} />
						</span>
					</a>
				</div>

				<Alert type="info" title="Only show latest saved app">
					Once made public, you will still need to save the app to make visible the latest changes
				</Alert>
			{/if}
		{/if}

		<div />
	</DrawerContent>
</Drawer>

<div
	class="border-b flex flex-row justify-between py-1 gap-4 overflow-x-auto gap-y-2 px-4 items-center"
>
	<div class="min-w-64 w-64">
		<input type="text" placeholder="App summary" class="text-sm w-full" bind:value={$summary} />
	</div>
	<div class="flex gap-4 items-center grow justify-center">
		<div>
			<ToggleButtonGroup bind:selected={$mode}>
				<ToggleButton position="left" value="dnd" size="xs">
					<div class="inline-flex gap-1 items-center">
						<Pencil size={14} />
						Editor
					</div>
				</ToggleButton>
				<ToggleButton position="right" value="preview" size="xs">
					<div class="inline-flex gap-1 items-center"> <Eye size={14} /> Preview</div></ToggleButton
				>
			</ToggleButtonGroup>
		</div>
		<div>
			<ToggleButtonGroup bind:selected={$breakpoint}>
				<ToggleButton position="left" value="sm" size="xs">
					<Smartphone size={14} />
				</ToggleButton>
				<ToggleButton position="right" value="lg" size="xs"><Laptop2 size={14} /></ToggleButton>
			</ToggleButtonGroup>
		</div>

		<span class="hidden lg:block">
			<ToggleButtonGroup bind:selected={$app.fullscreen}>
				<ToggleButton position="left" value={false} size="xs"
					><AlignHorizontalSpaceAround size={14} /> &nbsp; <Tooltip
						>The max width is 1168px and the content stay centered instead of taking the full page
						width</Tooltip
					></ToggleButton
				>
				<ToggleButton position="right" value={true} size="xs"><Expand size={14} /></ToggleButton>
			</ToggleButtonGroup>
		</span>
	</div>
	<div class="flex flex-row grow gap-4 justify-end ">
		<span class="hidden lg:block">
			<AppExportButton app={$app} />
		</span>

		<Button
			on:click={() => (publishDrawerOpen = true)}
			color="dark"
			size="xs"
			variant="border"
			startIcon={{ icon: faExternalLink }}
		>
			Publish
		</Button>
		<Button
			loading={loading.save}
			startIcon={{ icon: faSave }}
			on:click={save}
			color="dark"
			size="xs">Save</Button
		>
	</div>
</div>
