<script context="module">
	export function load() {
		return {
			stuff: { title: 'Resources' }
		}
	}
</script>

<script lang="ts">
	import { canWrite, emptySchema, sendUserToast } from '$lib/utils'
	import { ResourceService, VariableService } from '$lib/gen'
	import type { Resource, ResourceType } from '$lib/gen'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ResourceEditor from '$lib/components/ResourceEditor.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import Modal from '$lib/components/Modal.svelte'
	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import { userStore, workspaceStore, oauthStore } from '$lib/stores'
	import SchemaEditor from '$lib/components/SchemaEditor.svelte'
	import type { Schema } from '$lib/common'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import {
		faEdit,
		faPlus,
		faShare,
		faTrash,
		faCircle,
		faChain
	} from '@fortawesome/free-solid-svg-icons'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Icon from 'svelte-awesome'
	import Required from '$lib/components/Required.svelte'
	import AppConnect from '$lib/components/AppConnect.svelte'
	import { page } from '$app/stores'
	import { onMount } from 'svelte'
	import { Button } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'

	type ResourceW = Resource & { canWrite: boolean }
	type ResourceTypeW = ResourceType & { canWrite: boolean }

	let resources: ResourceW[] | undefined
	let resourceTypes: ResourceTypeW[] | undefined
	let resourceViewer: Modal
	let resourceViewerTitle: string = ''
	let resourceViewerSchema: Schema = emptySchema()

	let typeModalMode: 'view' | 'view-type' | 'create' = 'view'

	let newResourceTypeName: string
	let newResourceTypeSchema: Schema
	let newResourceTypeDescription: string

	let resourceEditor: ResourceEditor | undefined

	let shareModal: ShareModal
	let appConnect: AppConnect
	let deleteConfirmedCallback: (() => void) | undefined = undefined
	$: open = Boolean(deleteConfirmedCallback)

	async function loadResources(): Promise<void> {
		resources = (await ResourceService.listResource({ workspace: $workspaceStore! })).map((x) => {
			return {
				canWrite:
					canWrite(x.path, x.extra_perms!, $userStore) && $workspaceStore! == x.workspace_id,
				...x
			}
		})
	}

	async function loadResourceTypes(): Promise<void> {
		resourceTypes = (await ResourceService.listResourceType({ workspace: $workspaceStore! })).map(
			(x) => {
				return {
					canWrite: $workspaceStore! == x.workspace_id,
					...x
				}
			}
		)
	}

	async function deleteResource(path: string, is_oauth: boolean): Promise<void> {
		if (is_oauth) {
			await VariableService.deleteVariable({ workspace: $workspaceStore!, path })
		}
		await ResourceService.deleteResource({ workspace: $workspaceStore!, path })
		loadResources()
	}

	async function addResourceType(): Promise<void> {
		await ResourceService.createResourceType({
			workspace: $workspaceStore!,
			requestBody: {
				name: 'c_' + newResourceTypeName,
				schema: newResourceTypeSchema,
				description: newResourceTypeDescription
			}
		})
		resourceViewer.closeModal()
		loadResourceTypes()
	}

	async function handleDeleteResourceType(name: string) {
		try {
			await ResourceService.deleteResourceType({ workspace: $workspaceStore!, path: name })
			loadResourceTypes()
		} catch (err) {
			if (err.status === 400 && err.body.includes('foreign key')) {
				sendUserToast(
					`Could not delete resource type because there are resources attached to it. Delete resources using this type first.`,
					true
				)
			} else {
				sendUserToast(`Could not delete resource type: ${err?.body || err}`, true)
			}
		}
	}

	const startNewType = () => {
		resourceViewerTitle = `Create resource type`
		newResourceTypeName = 'my_resource_type'
		newResourceTypeSchema = emptySchema()
		newResourceTypeDescription = 'my description'
		typeModalMode = 'create'
		resourceViewer.openModal()
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadResources()
			loadResourceTypes()
		}
	}

	onMount(() => {
		let resource_type = $page.url.searchParams.get('resource_type')
		if ($oauthStore && resource_type) {
			appConnect.openFromOauth(resource_type)
		}
		if ($page.url.searchParams.get('connect_app')) {
			appConnect.open($page.url.searchParams.get('connect_app') ?? undefined)
		}
	})
</script>

<CenteredPage>
	<PageHeader title="Resources">
		<div class="flex flex-row space-x-4">
			<Button size="sm" startIcon={{ icon: faChain }} on:click={() => appConnect.open()}>
				Connect an API
			</Button>
			<Button size="sm" startIcon={{ icon: faPlus }} on:click={() => resourceEditor?.initNew()}>
				Add a resource
			</Button>
		</div>
	</PageHeader>

	<div class="relative">
		<TableCustom>
			<tr slot="header-row">
				<th>path</th>
				<th>resource_type</th>
				<th>description</th>
				<th>OAuth</th>
				<th />
			</tr>
			<tbody slot="body">
				{#if resources}
					{#each resources as { path, description, resource_type, extra_perms, canWrite, is_oauth }}
						<tr>
							<td class="my-12"
								><a
									href="#{path}"
									on:click={async () => {
										resourceViewerTitle = `Resource ${path}`
										resourceViewerSchema = (
											await ResourceService.getResource({
												workspace: $workspaceStore ?? 'no_workspace',
												path
											})
										).value
										typeModalMode = 'view'
										resourceViewer.openModal()
									}}>{path}</a
								>
								<div class="mb-1 -mt-1"><SharedBadge {canWrite} extraPerms={extra_perms} /></div>
							</td>
							<td class="px-2"><IconedResourceType name={resource_type} after={false} /></td>
							<td><SvelteMarkdown source={description ?? ''} /></td>
							<td>
								{#if is_oauth}
									<Icon
										class="text-green-600"
										data={faCircle}
										scale={0.7}
										label="Resource is tied to an OAuth app"
									/>
								{/if}
							</td>
							<td>
								<Dropdown
									dropdownItems={[
										{
											displayName: 'Share',
											icon: faShare,
											disabled: !canWrite,
											action: () => {
												shareModal.openModal(path)
											}
										},
										{
											displayName: 'Edit',
											icon: faEdit,
											disabled: !canWrite,
											action: () => {
												resourceEditor?.initEdit(path)
											}
										},
										{
											displayName: 'Delete',
											disabled: !canWrite,
											icon: faTrash,
											type: 'delete',
											action: (event) => {
												if (event?.shiftKey) {
													deleteResource(path, is_oauth)
												} else {
													deleteConfirmedCallback = () => {
														deleteResource(path, is_oauth)
													}
												}
											}
										}
									]}
									relative={false}
								/>
							</td>
						</tr>
					{/each}
				{:else if resources}
					<tr> No resources to display</tr>
				{:else}
					<tr>Loading...</tr>
				{/if}
			</tbody>
		</TableCustom>
	</div>
	<div class="py-10" />
	<PageHeader title="Resources types" primary={false}>
		<Button size="sm" startIcon={{ icon: faPlus }} on:click={startNewType}>Add a type</Button>
	</PageHeader>

	<TableCustom>
		<tr slot="header-row">
			<th>name</th>
			<th>description</th>
			<th />
		</tr>
		<tbody slot="body">
			{#if resourceTypes}
				{#each resourceTypes as { name, description, schema, canWrite }}
					<tr>
						<td class="pr-4"
							><a
								href="#{name}"
								on:click={() => {
									resourceViewerTitle = `Resource type ${name}`
									resourceViewerSchema = schema
									typeModalMode = 'view-type'
									resourceViewer.openModal()
								}}><span class="text-gray-700"><IconedResourceType {name} /></span></a
							></td
						>
						<td><SvelteMarkdown source={description ?? ''} /></td>
						<td>
							{#if canWrite}
								<Button
									size="sm"
									color="red"
									startIcon={{ icon: faTrash }}
									on:click={() => handleDeleteResourceType(name)}
									disabled={!($userStore?.is_admin || false)}
								>
									Delete
								</Button>
							{/if}
						</td>
					</tr>
				{/each}
			{:else if resources}
				<tr> No resources types to display</tr>
			{:else}
				<tr>Loading...</tr>
			{/if}
		</tbody>
	</TableCustom>
</CenteredPage>

<AppConnect bind:this={appConnect} on:refresh={loadResources} />
<ResourceEditor bind:this={resourceEditor} on:refresh={loadResources} />

<ShareModal
	bind:this={shareModal}
	kind="resource"
	on:change={() => {
		loadResources()
	}}
/>

<Modal bind:this={resourceViewer}>
	<div slot="title">{resourceViewerTitle}</div>
	<div slot="content">
		{#if typeModalMode === 'create'}
			<label for="inp"
				><h3 class="font-semibold text-gray-700">Name<Required required={true} /></h3>
				<div>
					<span
						class="border border-gray-700 rounded p-1 -mr-4 text-sm bg-gray-200 inline-block w-8"
						>c_</span
					>
					<div class="inline-block">
						<input id="inp" type="text" bind:value={newResourceTypeName} />
					</div>
				</div>
			</label>
			<label
				><h3 class="mt-4 font-semibold text-gray-700">Description</h3>
				<input type="text" bind:value={newResourceTypeDescription} /></label
			>
			<h3 class="mt-4 mb-2 font-semibold text-gray-700">Schema</h3>
			<SchemaEditor bind:schema={newResourceTypeSchema} />
		{:else if typeModalMode === 'view'}
			<Highlight language={json} code={JSON.stringify(resourceViewerSchema, null, 4)} />
		{:else if typeModalMode === 'view-type'}
			<SchemaViewer schema={resourceViewerSchema} />
		{/if}
	</div>
	<div slot="submission">
		{#if typeModalMode === 'create'}
			<Button on:click={addResourceType}>Save</Button>
		{/if}
	</div>
</Modal>

<ConfirmationModal
	{open}
	title="Remove resource"
	confirmationText="Remove"
	on:canceled={() => {
		deleteConfirmedCallback = undefined
	}}
	on:confirmed={() => {
		if (deleteConfirmedCallback) {
			deleteConfirmedCallback()
		}
		deleteConfirmedCallback = undefined
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to remove this resource?</span>
		<Alert type="info" title="Bypass confirmation">
			<div>
				You can press
				<Badge color="dark-gray">SHIFT</Badge>
				while removing a resource to bypass confirmation.
			</div>
		</Alert>
	</div>
</ConfirmationModal>
