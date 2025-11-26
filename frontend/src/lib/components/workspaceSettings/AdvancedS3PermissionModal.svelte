<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { defaultS3AdvancedPermissions, type S3ResourceSettings } from '$lib/workspace_settings'
	import { Plus } from 'lucide-svelte'
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import ClearableInput from '../common/clearableInput/ClearableInput.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import Modal2 from '../common/modal/Modal2.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'
	import Toggle from '../Toggle.svelte'
	import { emptyString } from '$lib/utils'

	type Props = {
		opened: NonNullable<S3ResourceSettings['secondaryStorage']>[number][1] | undefined
	}
	let { opened: storage = $bindable() }: Props = $props()
</script>

<Modal2
	title="Advanced S3 Permissions"
	bind:isOpen={() => !!storage, (v) => !v && (storage = undefined)}
	target="#content"
	contentClasses="flex flex-col gap-3"
	fixedWidth="md"
	fixedHeight="xl"
>
	{#if !!storage}
		{#if !$enterpriseLicense}
			<Alert
				type={storage.advancedPermissions ? 'error' : 'info'}
				title="Advanced permission rules are an Enterprise feature"
			>
				Consider upgrading to Windmill EE to use advanced permission rules to control access to your
				object storage at a more granular level.</Alert
			>
		{/if}
		<Toggle
			bind:checked={
				() => !!storage!.advancedPermissions,
				(v) => {
					storage!.advancedPermissions = v
						? defaultS3AdvancedPermissions(!!$enterpriseLicense)
						: undefined
					if (v) storage!.publicResource = false
				}
			}
			options={{
				right: 'Enable advanced permission rules',
				rightTooltip: 'Control precisely which paths are allowed to your users.'
			}}
			disabled={!storage.advancedPermissions && !$enterpriseLicense}
		/>
		{#if storage.advancedPermissions}
			<Alert title="Standard Unix-style glob syntax is supported">
				The following will be interpolated :
				<ul class="list-disc pl-6">
					<li><code>{'{username}'}</code> : Nickname of the user doing the request</li>
					<li><code>{'{group}'}</code> : Any group that the user belongs to</li>
					<li><code>{'{folder_read}'}</code> : Any folder that the user has read access to</li>
					<li><code>{'{folder_write}'}</code> : Any folder that the user has write access to</li>
				</ul>
				<br />
				Note that changes may take up to 1 minute to propagate due to cache invalidation
			</Alert>
			<div class="flex flex-col flex-1 gap-2 overflow-y-auto">
				{#each storage.advancedPermissions ?? [] as item, idx}
					<div class="flex gap-2">
						<ClearableInput bind:value={item.pattern} placeholder="Pattern" />
						<MultiSelect
							items={[
								{ value: 'read' },
								{ value: 'write' },
								{ value: 'delete' },
								{ value: 'list' }
							]}
							bind:value={item.allow}
							disablePortal
							class="w-[20rem]"
							placeholder="Deny all access"
							hideMainClearBtn
						/>
						<CloseButton onClick={() => storage!.advancedPermissions?.splice(idx, 1)} />
					</div>
				{/each}
			</div>
			<Button
				size="xs"
				variant="default"
				on:click={() => storage!.advancedPermissions?.push({ pattern: '', allow: [] })}
			>
				<Plus size={14} />
				Add permission rule
			</Button>
		{/if}
		{#if !storage.advancedPermissions}
			{#if storage.resourceType == 's3'}
				<div class="flex flex-col mt-2 mb-1 gap-1">
					<Toggle
						disabled={emptyString(storage.resourcePath)}
						bind:checked={storage.publicResource}
						options={{
							right:
								'S3 resource details and content can be accessed by all users of this workspace',
							rightTooltip:
								'If set, all users of this workspace will have access the to entire content of the S3 bucket, as well as the resource details and the "open preview" button. This effectively by-pass the permissions set on the resource and makes it public to everyone.'
						}}
					/>
					{#if storage.publicResource === true}
						<div class="pt-2"></div>

						<Alert
							type="warning"
							title="(Legacy) S3 bucket content and resource details are shared"
						>
							S3 resource public access is ON, which means that the entire content of the S3 bucket
							will be accessible to all the users of this workspace regardless of whether they have
							access the resource or not. Similarly, certain Windmill SDK endpoints can be used in
							scripts to access the resource details, including public and private keys.
						</Alert>
					{/if}
				</div>
			{:else}
				<div class="flex flex-col mt-5 mb-1 gap-1 max-w-[40rem]">
					<Toggle
						disabled={emptyString(storage.resourcePath)}
						bind:checked={storage.publicResource}
						options={{
							right: 'object storage content can be accessed by all users of this workspace',
							rightTooltip:
								'If set, all users of this workspace will have access the to entire content of the object storage.'
						}}
					/>
					{#if storage.publicResource === true}
						<div class="pt-2"></div>
						<Alert
							type="warning"
							title="(Legacy) Object storage content and resource details are shared"
						>
							object public access is ON, which means that the entire content of the object store
							will be accessible to all the users of this workspace regardless of whether they have
							access the resource or not.
						</Alert>
					{/if}
				</div>
			{/if}
		{/if}
	{/if}
</Modal2>
