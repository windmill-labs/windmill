<script lang="ts">
	import { Alert, Badge } from '$lib/components/common'
	import Required from '$lib/components/Required.svelte'
	import Section from '$lib/components/Section.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import Label from '$lib/components/Label.svelte'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import { bash } from 'svelte-highlight/languages'
	import { HttpTriggerService } from '$lib/gen'
	// import { page } from '$app/stores'
	import { base } from '$lib/base'
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import ClipboardPanel from '../../details/ClipboardPanel.svelte'
	import { isObject } from '$lib/utils'
	import { getHttpRoute } from './utils'
	import RouteBodyTransformerOption from './RouteBodyTransformerOption.svelte'
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
	export let showCapture = false
	export let headless: boolean = false
	export let captureInfo: CaptureInfo | undefined = undefined
	export let captureTable: CaptureTable | undefined = undefined
	export let workspaced_route: boolean = false
	export let isValid = false
	export let runnableArgs: any = {}
	export let raw_string: boolean = false
	export let wrap_body: boolean = false
	export let capture_mode: boolean
	export let isDraft: boolean = true

	let validateTimeout: NodeJS.Timeout | undefined = undefined
	let selectedRoute: 'test' | 'full' = 'full'

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

	$: fullRoute = getHttpRoute(route_path, workspaced_route, $workspaceStore ?? '')

	$: !http_method && (http_method = 'post')
	$: route_path === undefined && (route_path = '')

	$: captureURL = `${location.origin}${base}/api/w/${$workspaceStore}/capture_u/http/${
		captureInfo?.isFlow ? 'flow' : 'script'
	}/${captureInfo?.path.replaceAll('/', '.')}/${route_path}`

	$: userIsAdmin = $userStore?.is_admin || $userStore?.is_super_admin
	$: userCanEditConfig = userIsAdmin || isDraft // User can edit config if they are admin or if the trigger is a draft which will not be saved
</script>

<div>
	{#if showCapture && captureInfo}
		{@const captureURL = `${location.origin}${base}/api/w/${$workspaceStore}/capture_u/http/${
			captureInfo.isFlow ? 'flow' : 'script'
		}/${captureInfo.path.replaceAll('/', '.')}/${route_path}`}
		{@const cleanedRunnableArgs =
			isObject(runnableArgs) && 'wm_trigger' in runnableArgs
				? Object.fromEntries(Object.entries(runnableArgs).filter(([key]) => key !== 'wm_trigger'))
				: runnableArgs}
		<CaptureSection
			captureType="http"
			disabled={!isValid}
			{captureInfo}
			on:captureToggle
			on:applyArgs
			on:updateSchema
			on:addPreprocessor
			on:testWithArgs
			bind:captureTable
		>
			<Label label="URL">
				<ClipboardPanel content={captureURL} disabled={!captureInfo.active} />
			</Label>

			<Label label="Example cUrl">
				<CopyableCodeBlock
					disabled={!captureInfo.active}
					code={`curl \\
-X ${(http_method ?? 'post').toUpperCase()} ${captureURL} \\
-H 'Content-Type: application/json' \\
-d '${JSON.stringify(cleanedRunnableArgs ?? {}, null, 2)}'`}
					language={bash}
				/>
			</Label>
		</CaptureSection>
	{/if}
	<Section label="HTTP" {headless}>
		<svelte:fragment slot="header">
			{#if showCapture}
				<TestingBadge />
			{/if}
		</svelte:fragment>
		{#if !userCanEditConfig && !isDraft}
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
				let:item
				let:disabled
			>
				<ToggleButton label="GET" value="get" {item} {disabled} />
				<ToggleButton label="POST" value="post" {item} {disabled} />
				<ToggleButton label="PUT" value="put" {item} {disabled} />
				<ToggleButton label="PATCH" value="patch" {item} {disabled} />
				<ToggleButton label="DELETE" value="delete" {item} {disabled} />
			</ToggleButtonGroup>
			<div class="flex flex-col w-full">
				<div class="flex justify-start w-full">
					<Badge color="gray" class="rounded-r-none h-[27px]">Production URL</Badge>

					<ClipboardPanel
						content={selectedRoute === 'full' ? fullRoute : captureURL}
						class="rounded-l-none bg-surface border-none outline outline-2 outline-surface-secondary outline-offset-[-2px]"
					/>
				</div>
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
									'Prefixes the route with the workspace ID (e.g., {base_url}/api/r/{workspace_id}/{route}). Note: deploying the HTTP trigger to another workspace updates the route workspace prefix accordingly.'
							}}
						/>
					</div>
				{/if}
			</div>
			{#if capture_mode}
				<RouteBodyTransformerOption bind:raw_string bind:wrap_body />
			{/if}
		</div>
	</Section>
</div>
