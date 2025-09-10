<script lang="ts">
	import EmailTriggerEditorInner from './EmailTriggerEditorInner.svelte'
	import Description from '$lib/components/Description.svelte'
	import { enterpriseLicense, userStore } from '$lib/stores'
	import { Alert } from '$lib/components/common'
	import { onMount, type Snippet } from 'svelte'
	import { getEmailDomain } from './utils'
	import type { Trigger } from '../utils'

	let emailTriggerEditor = $state<EmailTriggerEditorInner | null>(null)

	interface Props {
		selectedTrigger: Trigger
		isFlow: boolean
		path: string
		defaultValues?: Record<string, any>
		isEditor?: boolean
		customLabel: Snippet
		onEmailDomain: (domain: string) => void
	}
	let {
		selectedTrigger,
		isFlow,
		path,
		defaultValues = undefined,
		isEditor = false,
		customLabel,
		onEmailDomain,
		...restProps
	}: Props = $props()

	async function openEmailTriggerEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			emailTriggerEditor?.openNew(isFlow, path, defaultValues)
		} else {
			emailTriggerEditor?.openEdit(selectedTrigger.path ?? '', isFlow, defaultValues)
		}
	}

	onMount(() => {
		if (emailTriggerEditor) {
			openEmailTriggerEditor(isFlow, selectedTrigger.isDraft ?? false)
		}
	})

	let emailDomain: string | null = $state(null)

	getEmailDomain().then((domain) => {
		emailDomain = domain
	})

	$effect(() => {
		if (emailDomain) {
			onEmailDomain(emailDomain)
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

			{#if !$enterpriseLicense}
				<Alert title="Community Edition limitations" type="warning" size="xs">
					Email triggers on Windmill Community Edition are limited to 100 emails per day.
				</Alert>
			{/if}
		</div>
	{/snippet}
</EmailTriggerEditorInner>
