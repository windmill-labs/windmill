<script lang="ts">
	import { JobService } from '$lib/gen'
	import Button from '$lib/components/common/button/Button.svelte'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { sendUserToast } from '$lib/toast'
	import { onDestroy, onMount } from 'svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { LogIn, AlertTriangle } from 'lucide-svelte'
	import { emptyString } from '$lib/utils'
	import { Alert } from '$lib/components/common'
	import { getUserExt } from '$lib/user'
	import { setLicense } from '$lib/enterpriseUtils'
	import { page } from '$app/state'

	$workspaceStore = page.params.workspace
	let rd = page.url.href.replace(page.url.origin, '')
	let token = page.url.searchParams.get('token') ?? undefined

	let approvalInfo: any = $state(undefined)
	let completed = $state(false)
	let error: string | undefined = $state(undefined)
	let default_payload: any = $state({})
	let loading = $state(false)
	let valid = $state(true)

	let pollInterval: number | undefined = undefined

	setLicense()

	onMount(() => {
		window.onunhandledrejection = (event: PromiseRejectionEvent) => {
			event.preventDefault()
			pollInterval && clearInterval(pollInterval)
			if (event.reason?.message) {
				const { message, body } = event.reason
				if (body) {
					sendUserToast(`${body}`, true)
					error = body.toString()
				} else {
					sendUserToast(`${message}`, true)
					error = message.toString()
				}
			}
		}
		loadApprovalInfo()
		pollInterval = setInterval(loadApprovalInfo, 2000)
	})

	onDestroy(() => {
		pollInterval && clearInterval(pollInterval)
	})

	async function loadApprovalInfo() {
		try {
			approvalInfo = await JobService.getApprovalInfo({
				workspace: page.params.workspace ?? '',
				jobId: page.params.job ?? '',
				token
			})
		} catch (e: any) {
			error = e?.body ?? e?.message ?? 'Failed to load approval info'
			pollInterval && clearInterval(pollInterval)
		}
	}

	async function loadUser() {
		userStore.set(await getUserExt(page.params.workspace ?? ''))
	}

	async function resume() {
		loading = true
		try {
			await JobService.resumeSuspended({
				workspace: page.params.workspace ?? '',
				jobId: page.params.job ?? '',
				requestBody: {
					payload: default_payload,
					approval_token: token,
					approved: true
				}
			})
			sendUserToast('Flow approved')
			loadApprovalInfo()
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed to approve', true)
			error = e?.body ?? e?.message
		} finally {
			loading = false
		}
	}

	async function cancel() {
		loading = true
		try {
			await JobService.resumeSuspended({
				workspace: page.params.workspace ?? '',
				jobId: page.params.job ?? '',
				requestBody: {
					approval_token: token,
					approved: false
				}
			})
			sendUserToast('Flow denied!')
			loadApprovalInfo()
		} catch (e: any) {
			sendUserToast(e?.body ?? e?.message ?? 'Failed to cancel', true)
			error = e?.body ?? e?.message
		} finally {
			loading = false
		}
	}

	let schema = $derived(approvalInfo?.form_schema?.schema ?? {})
	let hasForm = $derived(schema && typeof schema === 'object' && Object.keys(schema).length > 0)

	$effect(() => {
		if (approvalInfo?.user_auth_required && !$userStore) {
			loadUser()
		}
	})
</script>

<CenteredModal title="Approval for resuming of flow" disableLogo centerVertically={false}>
	{#if error}
		<div class="space-y-6">
			{#if error.includes('logged in') || error.includes('sign in') || error.includes('Not authorized')}
				<div class="flex flex-row gap-4 justify-center">
					<AlertTriangle />
					<p class="text-lg">Authentication Required</p>
				</div>
				<p class="text-sm">{error}</p>
				<Button href={`/user/login?${rd ? 'rd=' + encodeURIComponent(rd) : ''}`}>
					Sign in
					<LogIn class="w-8" size={18} />
				</Button>
			{:else if error.includes('Permission denied') || error.includes('Self-approval')}
				<div class="flex flex-row gap-4 justify-center">
					<AlertTriangle />
					<p class="text-lg">Permission Denied</p>
				</div>
				<p class="text-sm">{error}</p>
			{:else}
				<div class="flex flex-row gap-4 justify-center">
					<AlertTriangle />
					<p class="text-lg">Error</p>
				</div>
				<p class="text-sm">{error}</p>
			{/if}
		</div>
	{:else if approvalInfo}
		<div class="space-y-4">
			{#if approvalInfo.approvers?.length > 0}
				<div>
					<h2 class="text-sm font-semibold text-emphasis">Approvers</h2>
					<ul class="mt-2 text-xs text-primary">
						{#each approvalInfo.approvers as a}
							<li>{a.approver}</li>
						{/each}
					</ul>
				</div>
			{/if}

			{#if approvalInfo.description}
				<div>
					<h2 class="text-sm font-semibold text-emphasis">Description</h2>
					<p class="mt-1 text-xs text-primary">{approvalInfo.description}</p>
				</div>
			{/if}

			{#if hasForm && !completed}
				{#if emptyString($enterpriseLicense)}
					<Alert type="warning" title="Adding a form to the approval page is an EE feature" />
				{:else}
					<SchemaForm
						onlyMaskPassword
						noVariablePicker
						bind:isValid={valid}
						bind:args={default_payload}
						{schema}
					/>
				{/if}
			{/if}

			{#if !completed && approvalInfo.can_approve}
				<div class="flex gap-2">
					{#if approvalInfo.hide_cancel !== true}
						<Button variant="default" destructive {loading} onclick={cancel}>Deny</Button>
					{/if}
					<Button variant="accent" {loading} disabled={!valid} onclick={resume}>Approve</Button>
				</div>
			{:else if !completed && !approvalInfo.can_approve}
				{#if approvalInfo.user_auth_required && !$userStore}
					<Button href={`/user/login?${rd ? 'rd=' + encodeURIComponent(rd) : ''}`}>
						Sign in to approve
						<LogIn class="w-8" size={18} />
					</Button>
				{:else}
					<p class="text-sm text-secondary">You are not authorized to approve this flow.</p>
				{/if}
			{:else}
				<p class="text-sm text-secondary">This approval has been completed.</p>
			{/if}
		</div>
	{:else}
		<p class="text-sm text-secondary">Loading...</p>
	{/if}
</CenteredModal>
