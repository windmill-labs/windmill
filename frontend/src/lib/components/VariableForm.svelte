<script lang="ts">
	import Path from './Path.svelte'
	import LabelsInput from './LabelsInput.svelte'
	import Toggle from './Toggle.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { Button } from './common'
	import Tooltip from './Tooltip.svelte'
	import Label from './Label.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { Loader2 } from 'lucide-svelte'
	import autosize from '$lib/autosize'
	import { userStore, workspaceStore } from '$lib/stores'
	import { isOwner } from '$lib/utils'

	interface Variable {
		value: string
		is_secret: boolean
		description: string
	}

	interface Props {
		path: string
		initialPath: string
		pathError: string
		variable: Variable
		labels: string[] | undefined
		wsSpecific: boolean
		deployTo: string | undefined
		can_write: boolean
		edit: boolean
		onLoadSecret?: () => void
	}

	let {
		path = $bindable(),
		initialPath,
		pathError = $bindable(),
		variable = $bindable(),
		labels = $bindable(),
		wsSpecific = $bindable(),
		deployTo,
		can_write,
		edit,
		onLoadSecret
	}: Props = $props()

	const MAX_VARIABLE_LENGTH = 10000

	let editorKind: 'plain' | 'json' | 'yaml' = $state('plain')
	let editor: any = $state(undefined)

	export function setCode(value: string) {
		editor?.setCode(value)
	}
</script>

<div class="flex flex-col gap-1">
	<label for="path" class="text-xs font-semibold text-emphasis">Path</label>
	<Path
		disabled={initialPath != '' && !isOwner(initialPath, $userStore, $workspaceStore)}
		bind:error={pathError}
		bind:path
		{initialPath}
		namePlaceholder="variable"
		kind="variable"
	/>
	<LabelsInput bind:labels />
</div>
<label class="flex flex-col gap-1">
	<span class="text-xs font-semibold text-emphasis">Secret</span>
	<Toggle
		on:change={() => edit && onLoadSecret?.()}
		bind:checked={variable.is_secret}
		disabled={edit && $userStore?.operator}
	/>
	{#if variable.is_secret}
		<Alert type="warning" title="Audit log for each access">
			Every secret is encrypted at rest and in transit with a key specific to this workspace. In
			addition, any read of a secret variable generates an audit log whose operation name is:
			variables.decrypt_secret
		</Alert>
	{/if}
</label>

{#if deployTo}
	<Label
		label="Workspace specific"
		tooltip="Prevents this variable from being deployed to prod/staging. May have been enabled automatically because a workspace-specific resource references this variable via $var:. Disabling this toggle does not retroactively un-mark the resource that referenced it."
	>
		<Toggle bind:checked={wsSpecific} />
	</Label>
{/if}

<div class="flex flex-col gap-1">
	<label for="variable-value" class="flex flex-row justify-left items-center">
		<span class="text-xs font-semibold text-emphasis">Variable value&nbsp;</span>
		<span class="text-xs text-secondary font-normal">
			({variable.value.length}/{MAX_VARIABLE_LENGTH} characters)
		</span>
		{#if edit && variable.is_secret}
			<div class="ml-3"></div>
			{#if $userStore?.operator}
				<div class="p-2 border">Operators cannot load secret value</div>
			{:else}
				<Button size="xs" variant="default" on:click={() => onLoadSecret?.()}>
					Load secret value<Tooltip>Will generate an audit log</Tooltip>
				</Button>
			{/if}
		{/if}
	</label>
	<div>
		<div class="flex flex-col gap-2">
			<ToggleButtonGroup bind:selected={editorKind}>
				{#snippet children({ item })}
					<ToggleButton value="plain" label="Plain" {item} />
					<ToggleButton value="json" label="Json" {item} />
					<ToggleButton value="yaml" label="YAML" {item} />
				{/snippet}
			</ToggleButtonGroup>
			{#if editorKind == 'plain'}
				<textarea
					disabled={!can_write}
					rows="4"
					use:autosize
					bind:value={variable.value}
					placeholder="Update variable value"
					id="variable-value"
				></textarea>
			{:else if editorKind == 'json'}
				<div class="border rounded mb-4 w-full">
					{#await import('$lib/components/SimpleEditor.svelte')}
						<Loader2 class="animate-spin" />
					{:then Module}
						<Module.default
							bind:this={editor}
							autoHeight
							lang="json"
							bind:code={variable.value}
							fixedOverflowWidgets={false}
							class="bg-surface-tertiary"
						/>
					{/await}
				</div>
			{:else if editorKind == 'yaml'}
				<div class="border rounded mb-4 w-full">
					{#await import('$lib/components/SimpleEditor.svelte')}
						<Loader2 class="animate-spin" />
					{:then Module}
						<Module.default
							bind:this={editor}
							autoHeight
							lang="yaml"
							bind:code={variable.value}
							fixedOverflowWidgets={false}
							class="bg-surface-tertiary"
						/>
					{/await}
				</div>
			{/if}
		</div>
	</div>
</div>
<label class="flex flex-col gap-1">
	<span class="text-xs font-semibold text-emphasis">Description</span>
	<textarea rows="4" use:autosize bind:value={variable.description} placeholder="Used for X"
	></textarea>
</label>
