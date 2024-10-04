<script lang="ts">
	import { Alert, Badge, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { HttpTriggerService } from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Save } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { page } from '$app/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { base } from '$lib/base'

	let is_flow: boolean = false
	let initialPath = ''
	let edit = true

	let itemKind: 'flow' | 'script' = 'script'

	$: is_flow = itemKind === 'flow'

	let script_path = ''
	let initialScriptPath = ''
	let fixedScriptPath = ''

	let drawerLoading = true
	export async function openEdit(ePath: string, isFlow: boolean) {
		drawerLoading = true
		try {
			drawer?.openDrawer()
			initialPath = ePath
			itemKind = isFlow ? 'flow' : 'script'
			edit = true
			dirtyPath = false
			await loadTrigger()
		} catch (err) {
			sendUserToast(`Could not load route: ${err}`, true)
		} finally {
			drawerLoading = false
		}
	}

	export async function openNew(nis_flow: boolean, fixedScriptPath_?: string) {
		drawerLoading = true
		try {
			drawer?.openDrawer()
			is_flow = nis_flow
			edit = false
			itemKind = nis_flow ? 'flow' : 'script'
			is_async = false
			requires_auth = false
			initialRoutePath = ''
			route_path = ''
			http_method = 'post'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = ''
			initialPath = ''
			dirtyPath = false
		} finally {
			drawerLoading = false
		}
	}

	let path: string = ''
	let pathError = ''
	let routeError = ''
	let is_async = false
	let requires_auth = false
	let initialRoutePath = ''
	let route_path = ''
	let http_method: 'get' | 'post' | 'put' | 'patch' | 'delete' = 'post'

	const dispatch = createEventDispatcher()

	let can_write = true
	async function loadTrigger(): Promise<void> {
		const s = await HttpTriggerService.getHttpTrigger({
			workspace: $workspaceStore!,
			path: initialPath
		})
		script_path = s.script_path
		initialScriptPath = s.script_path

		is_flow = s.is_flow
		path = s.path
		route_path = s.route_path
		initialRoutePath = s.route_path
		http_method = s.http_method ?? 'post'
		is_async = s.is_async
		requires_auth = s.requires_auth

		can_write = canWrite(s.path, s.extra_perms, $userStore)
	}

	async function triggerScript(): Promise<void> {
		if (edit) {
			await HttpTriggerService.updateHttpTrigger({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					path,
					script_path,
					is_flow,
					is_async,
					requires_auth,
					route_path: $userStore?.is_admin || $userStore?.is_super_admin ? route_path : undefined,
					http_method
				}
			})
			sendUserToast(`Route ${path} updated`)
		} else {
			await HttpTriggerService.createHttpTrigger({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					script_path,
					is_flow,
					is_async,
					requires_auth,
					route_path,
					http_method
				}
			})
			sendUserToast(`Route ${path} created`)
		}
		if (!$usedTriggerKinds.includes('http')) {
			$usedTriggerKinds = [...$usedTriggerKinds, 'http']
		}
		dispatch('update')
		drawer.closeDrawer()
	}

	let drawer: Drawer

	let dirtyPath = false

	$: fullRoute = `${$page.url.origin}${base}/api/r/${
		isCloudHosted() ? $workspaceStore + '/' : ''
	}${route_path}`

	async function routeExists(route_path: string, method: typeof http_method) {
		return await HttpTriggerService.existsRoute({
			workspace: $workspaceStore!,
			requestBody: {
				route_path,
				http_method
			}
		})
	}

	let validateTimeout: NodeJS.Timeout | undefined = undefined
	async function validateRoute(path: string, method: typeof http_method): Promise<void> {
		routeError = ''
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(async () => {
			if (!/^[\w-:]+(\/[\w-:]+)*$/.test(path)) {
				routeError = 'Endpoint not valid'
			} else if (initialRoutePath !== path && (await routeExists(path, method))) {
				routeError = 'Endpoint already taken'
			} else {
				routeError = ''
			}
			validateTimeout = undefined
		}, 500)
	}

	$: validateRoute(route_path, http_method)
</script>

<Drawer size="700px" bind:this={drawer}>
	<DrawerContent
		title={edit ? (can_write ? `Edit route ${initialPath}` : `Route ${initialPath}`) : 'New route'}
		on:close={drawer.closeDrawer}
	>
		<svelte:fragment slot="actions">
			{#if !drawerLoading && can_write}
				<Button
					startIcon={{ icon: Save }}
					disabled={pathError != '' || routeError != '' || emptyString(script_path) || !can_write}
					on:click={triggerScript}
				>
					Save
				</Button>
			{/if}
		</svelte:fragment>
		{#if drawerLoading}
			<Loader2 class="animate-spin" />
		{:else}
			<div class="flex flex-col gap-12">
				<div class="flex flex-col gap-4">
					<Label label="Path">
						<Path
							bind:dirty={dirtyPath}
							bind:error={pathError}
							bind:path
							{initialPath}
							checkInitialPathExistence={!edit}
							namePlaceholder="route"
							kind="http_trigger"
							hideUser
							disabled={!can_write}
						/>
					</Label>
				</div>

				<Section label="HTTP">
					{#if !($userStore?.is_admin || $userStore?.is_super_admin)}
						<Alert type="info" title="Admin only" collapsible>
							Route endpoints can only be edited by workspace admins
						</Alert>
						<div class="my-2" />
					{/if}
					<div class="flex flex-col w-full gap-4">
						<label class="block grow w-full">
							<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
								<div>
									Path
									<Required required={true} />
								</div>
								<div class="text-2xs text-tertiary"> ':myparam' for path params </div>
							</div>
							<!-- svelte-ignore a11y-autofocus -->
							<input
								type="text"
								autocomplete="off"
								bind:value={route_path}
								disabled={!($userStore?.is_admin || $userStore?.is_super_admin) || !can_write}
								class={routeError === ''
									? ''
									: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
							/>
						</label>

						<ToggleButtonGroup
							class="w-auto"
							bind:selected={http_method}
							disabled={!($userStore?.is_admin || $userStore?.is_super_admin) || !can_write}
						>
							<ToggleButton label="GET" value="get" />
							<ToggleButton label="POST" value="post" />
							<ToggleButton label="PUT" value="put" />
							<ToggleButton label="PATCH" value="patch" />
							<ToggleButton label="DELETE" value="delete" />
						</ToggleButtonGroup>
						<div class="flex flex-col w-full mt-2">
							<div class="flex justify-start w-full">
								<Badge
									color="gray"
									class="center-center !bg-surface-secondary !text-tertiary !w-[90px] !h-[24px] rounded-r-none border"
								>
									Full endpoint
								</Badge>
								<input
									type="text"
									readonly
									value={fullRoute}
									size={fullRoute.length || 50}
									class="font-mono !text-xs max-w-[calc(100%-70px)] !w-auto !h-[24px] !py-0 !border-l-0 !rounded-l-none"
									on:focus={({ currentTarget }) => {
										currentTarget.select()
									}}
								/>
							</div>
							<div class="text-red-600 dark:text-red-400 text-2xs">{routeError}</div>
						</div>
					</div>
				</Section>

				<Section label="Runnable">
					<p class="text-xs mb-1 text-tertiary">
						Pick a script or flow to be triggered<Required required={true} /><br />
						To handle headers, query or path parameters, add a preprocessor to your runnable.
					</p>
					<div class="flex flex-row mb-2">
						<ScriptPicker
							disabled={fixedScriptPath != '' || !can_write}
							initialPath={fixedScriptPath || initialScriptPath}
							kinds={['script']}
							allowFlow={true}
							bind:itemKind
							bind:scriptPath={script_path}
							allowRefresh
						/>

						{#if script_path === undefined}
							<Button
								btnClasses="ml-4 mt-2"
								color="dark"
								size="xs"
								href={itemKind === 'flow'
									? '/flows/add?hub=55'
									: '/scripts/add?hub=hub%2F9088%2Fwindmill%2FHTTP%20route%20script%20with%20preprocessor%20template'}
								target="_blank">Create from template</Button
							>
						{/if}
					</div>
				</Section>
				<Section label="Settings">
					<div class="flex flex-col gap-4">
						<div class="flex flex-row justify-between">
							<div class="text-sm font-semibold flex flex-row items-center">Request type</div>
							<ToggleButtonGroup class="w-auto" bind:selected={is_async} disabled={!can_write}>
								<ToggleButton
									label="Async"
									value={true}
									tooltip="The returning value is the uuid of the job assigned to execute the job."
								/>
								<ToggleButton
									label="Sync"
									value={false}
									tooltip="Triggers the execution, wait for the job to complete and return it as a response."
								/>
							</ToggleButtonGroup>
						</div>
						<div class="flex flex-row justify-between">
							<div class="text-sm font-semibold flex flex-row items-center">Authentication</div>
							<ToggleButtonGroup class="w-auto" bind:selected={requires_auth} disabled={!can_write}>
								<ToggleButton label="None" value={false} />
								<ToggleButton
									label="Required"
									value={true}
									tooltip="Requires authentication with read access on the route"
								/>
							</ToggleButtonGroup>
						</div>
					</div>
				</Section>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
