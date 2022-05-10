<script lang="ts">
	import { canWrite, emptySchema, getUser, sendUserToast } from '../utils'
	import { ResourceService } from '../gen'
	import type { Resource, ResourceType } from '../gen'
	import PageHeader from './components/PageHeader.svelte'
	import ResourceEditor from './components/ResourceEditor.svelte'
	import Button from './components/Button.svelte'
	import TableCustom from './components/TableCustom.svelte'
	import Modal from './components/Modal.svelte'
	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/src/languages/json'
	import github from 'svelte-highlight/src/styles/github'
	import IconedResourceType from './components/IconedResourceType.svelte'
	import ShareModal from './components/ShareModal.svelte'
	import SharedBadge from './components/SharedBadge.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import { userStore, workspaceStore, type UserExt } from '../stores'
	import SchemaEditor from './components/SchemaEditor.svelte'
	import type { Schema } from '../common'
	import SchemaViewer from './components/SchemaViewer.svelte'
	import Dropdown from './components/Dropdown.svelte'
	import { faEdit, faPlus, faShare, faTrash } from '@fortawesome/free-solid-svg-icons'
	import CenteredPage from './components/CenteredPage.svelte'
	import Icon from 'svelte-awesome'
	import Required from './components/Required.svelte'

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

	let user: UserExt | undefined
	$: user = $userStore

	let shareModal: ShareModal

	async function loadResources(): Promise<void> {
		const user = await getUser($workspaceStore!)
		resources = (await ResourceService.listResource({ workspace: $workspaceStore! })).map((x) => {
			return {
				canWrite: canWrite(x.path, x.extra_perms!, user) && $workspaceStore! == x.workspace_id,
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

	async function deleteResource(path: string): Promise<void> {
		await ResourceService.deleteResource({ workspace: $workspaceStore!, path })
		loadResources()
	}

	async function addResourceType(): Promise<void> {
		try {
			await ResourceService.createResourceType({
				workspace: $workspaceStore!,
				requestBody: {
					name: newResourceTypeName,
					schema: newResourceTypeSchema,
					description: newResourceTypeDescription
				}
			})
			resourceViewer.closeModal()
			loadResourceTypes()
		} catch (err) {
			sendUserToast(`Could not create resource type: ${err?.body || err}`, true)
		}
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

	$: {
		if ($workspaceStore) {
			loadResources()
			loadResourceTypes()
		}
	}
</script>

<svelte:head>
	{@html github}
</svelte:head>

<CenteredPage>
	<PageHeader title="Resources">
		<button
			class="default-button"
			on:click={() => {
				resourceEditor?.initNew()
			}}><Icon class="text-white mb-1" data={faPlus} scale={0.9} /> &nbsp; Add a resource</button
		>
	</PageHeader>

	<div class="relative">
		<TableCustom>
			<tr slot="header-row">
				<th>path</th>
				<th>resource_type</th>
				<th>description</th>
				<th />
			</tr>
			<tbody slot="body">
				{#if resources}
					{#each resources as { path, description, resource_type, extra_perms, canWrite }}
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
							<td><IconedResourceType name={resource_type} /></td>
							<td><SvelteMarkdown source={description ?? ''} /></td>
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
											action: () => {
												deleteResource(path)
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
		<button
			class="default-button"
			on:click={() => {
				resourceViewerTitle = `Create resource type`

				newResourceTypeName = 'my_resource_type'
				newResourceTypeSchema = emptySchema()
				newResourceTypeDescription = 'my description'
				typeModalMode = 'create'
				resourceViewer.openModal()
			}}><Icon class="text-white mb-1" data={faPlus} scale={0.9} /> &nbsp; Add a type</button
		>
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
						<td
							><a
								href="#{name}"
								on:click={() => {
									resourceViewerTitle = `Resource type ${name}`
									resourceViewerSchema = schema
									typeModalMode = 'view-type'
									resourceViewer.openModal()
								}}><IconedResourceType {name} /></a
							></td
						>
						<td><SvelteMarkdown source={description ?? ''} /></td>
						<td>
							{#if canWrite}
								<Button
									category="delete"
									class="mx-2"
									on:click={() => {
										handleDeleteResourceType(name)
									}}
									disabled={!(user?.is_admin ?? false)}
								/>
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
			<label
				><h3 class="font-semibold text-gray-700">Name<Required required={true} /></h3>
				<input bind:value={newResourceTypeName} /></label
			>
			<label
				><h3 class="mt-4 font-semibold text-gray-700">Description</h3>
				<input bind:value={newResourceTypeDescription} /></label
			>
			<!-- svelte-ignore a11y-label-has-associated-control -->
			<!-- <label>Schema <AutosizedTextarea minRows={10} bind:value={newResourceTypeSchema} /></label> -->
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
			<button class="default-button px-4 py-2 font-semibold" on:click={addResourceType}>
				Save
			</button>
		{/if}
	</div></Modal
>

<style>
</style>
