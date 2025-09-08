<script lang="ts">
	import EmailTriggerEditorInner from './EmailTriggerEditorInner.svelte'
	import Description from '$lib/components/Description.svelte'
	import { userStore } from '$lib/stores'
	import { Alert } from '$lib/components/common'
	import { onMount } from 'svelte'

	let emailTriggerEditor = $state<EmailTriggerEditorInner | null>(null)
	let {
		selectedTrigger,
		isFlow,
		path,
		defaultValues = undefined,
		isEditor = false,
		customLabel = undefined,
		...restProps
	} = $props()

	async function openEmailTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			emailTriggerEditor?.openNew(isFlow, path, defaultValues)
		} else {
			emailTriggerEditor?.openEdit(selectedTrigger.path, isFlow, defaultValues)
		}
	}

	onMount(() => {
		if (emailTriggerEditor) {
			openEmailTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
		}
	})
</script>

<EmailTriggerEditorInner
	useDrawer={false}
	bind:this={emailTriggerEditor}
	hideTarget
	{isEditor}
	{customLabel}
	trigger={selectedTrigger}
	allowDraft
	{...restProps}
>
	{#snippet description()}
		<div class="flex flex-col gap-2 pb-4">
			<Description link="https://www.windmill.dev/docs/advanced/email_triggers">
				Email triggers execute scripts and flows when emails are sent to specific addresses. Each
				trigger can be configured with a specific local part.
			</Description>

			{#if !$userStore?.is_admin && !$userStore?.is_super_admin && selectedTrigger.isDraft}
				<Alert title="Only workspace admins can create email triggers" type="info" size="xs" />
			{/if}
		</div>
	{/snippet}
</EmailTriggerEditorInner>
