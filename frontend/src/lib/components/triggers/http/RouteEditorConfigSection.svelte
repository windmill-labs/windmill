<script lang="ts">
	import { Alert, Url } from '$lib/components/common'
	import Required from '$lib/components/Required.svelte'
	import Section from '$lib/components/Section.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { HttpTriggerService } from '$lib/gen'
	// import { page } from '$app/stores'
	import { getHttpRoute } from './utils'
	import { isCloudHosted } from '$lib/cloud'
	import Toggle from '$lib/components/Toggle.svelte'
	import TestingBadge from '../testingBadge.svelte'

	export let initialTriggerPath: string | undefined = undefined
	export let dirtyRoutePath: boolean = false
	export let route_path: string | undefined
	export let http_method: 'get' | 'post' | 'put' | 'patch' | 'delete' | undefined
	export let can_write: boolean = false
	export let static_asset_config: { s3: string; storage?: string; filename?: string } | undefined =
		undefined
	export let headless: boolean = false
	export let workspaced_route: boolean = false
	export let isValid = false
	export let isDraftOnly: boolean = true
	export let showTestingBadge: boolean = false

	let validateTimeout: NodeJS.Timeout | undefined = undefined

	let routeError: string = ''
	async function validateRoute(
		routePath: string | undefined,
		method: typeof http_method,
		workspaced_route: boolean
	): Promise<void> {
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(async () => {
			if (!routePath || !method || !/^:?[-\w]+(\/:?[-\w]+)*$/.test(routePath)) {
				routeError = 'Endpoint not valid'
			} else if (await routeExists(routePath, method, workspaced_route)) {
				routeError = 'Endpoint already taken'
			} else {
				routeError = ''
			}
			validateTimeout = undefined
		}, 500)
	}
	async function routeExists(
		route_path: string,
		method: Exclude<typeof http_method, undefined>,
		workspaced_route: boolean
	) {
		return await HttpTriggerService.existsRoute({
			workspace: $workspaceStore!,
			requestBody: {
				route_path,
				http_method: method,
				trigger_path: initialTriggerPath,
				workspaced_route: workspaced_route
			}
		})
	}

	$: validateRoute(route_path, http_method, workspaced_route)

	$: isValid = routeError === ''

	$: fullRoute = getHttpRoute('r', route_path, workspaced_route, $workspaceStore ?? '')

	$: !http_method && (http_method = 'post')
	$: route_path === undefined && (route_path = '')

	$: userIsAdmin = $userStore?.is_admin || $userStore?.is_super_admin
	$: userCanEditConfig = userIsAdmin || isDraftOnly // User can edit config if they are admin or if the trigger is a draft which will not be saved
</script>

<div>
	<Section label="HTTP" {headless}>
		<svelte:fragment slot="header">
			{#if showTestingBadge}
				<TestingBadge />
			{/if}
		</svelte:fragment>
		{#if !userCanEditConfig && isDraftOnly}
			<Alert type="info" title="Admin only" collapsible size="xs">
				Route endpoints can only be edited by workspace admins
			</Alert>
			<div class="my-2"></div>
		{/if}
		<div class="flex flex-col w-full gap-4">
			<label class="block grow w-full">
				<div class="flex flex-col gap-1">
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
						disabled={!userCanEditConfig || !can_write}
						class={routeError === ''
							? ''
							: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
						on:input={() => {
							dirtyRoutePath = true
						}}
					/>
				</div>
			</label>

			<ToggleButtonGroup
				class="w-auto"
				bind:selected={http_method}
				disabled={!userCanEditConfig || !can_write || !!static_asset_config}
			>
				{#snippet children({ item, disabled })}
					<ToggleButton label="GET" value="get" {item} {disabled} />
					<ToggleButton label="POST" value="post" {item} {disabled} />
					<ToggleButton label="PUT" value="put" {item} {disabled} />
					<ToggleButton label="PATCH" value="patch" {item} {disabled} />
					<ToggleButton label="DELETE" value="delete" {item} {disabled} />
				{/snippet}
			</ToggleButtonGroup>
			<div class="flex flex-col w-full">
				<Url url={fullRoute} label="Production URL" />

				<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5"
					>{dirtyRoutePath ? routeError : ''}</div
				>
				{#if !isCloudHosted()}
					<div class="mt-1">
						<Toggle
							size="sm"
							checked={workspaced_route}
							disabled={!can_write}
							on:change={() => {
								workspaced_route = !workspaced_route
								dirtyRoutePath = true
							}}
							options={{
								right: 'Prefix with workspace',
								rightTooltip:
									'Prefixes the route with the workspace ID (e.g., {base_url}/api/r/{workspace_id}/{route}). Note: deploying the HTTP trigger to another workspace updates the route workspace prefix accordingly.',
								rightDocumentationLink:
									'https://www.windmill.dev/docs/core_concepts/http_routing#workspace-prefix'
							}}
						/>
					</div>
				{/if}
			</div>
		</div>
	</Section>
</div>
