<script lang="ts">
	import { onMount } from 'svelte'
	import { CheckCircle, XCircle } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'

	let isLoading = true
	let isSuccess = false
	let errorMessage = ''

	function closeWindow() {
		window.close()
	}

	onMount(async () => {
		const url = new URL(window.location.href)
		const workspace_id = url.searchParams.get('workspace_id') || ''
		const installation_id_str = url.searchParams.get('installation_id') || ''
		const account_id = url.searchParams.get('account_id') || ''
		const jwt_token = url.searchParams.get('jwt_token') || ''

		const installation_id = parseInt(installation_id_str, 10)

		if (!workspace_id || isNaN(installation_id) || !account_id || !jwt_token) {
			isLoading = false
			errorMessage = 'Missing or invalid required parameters'
			sendUserToast('Missing or invalid required parameters in the URL', true)
			return
		}

		try {
			const response = await fetch(`/api/w/${workspace_id}/github_app/installation_callback`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					installation_id,
					account_id,
					jwt_token
				})
			})

			if (!response.ok) {
				const errorData = await response.text()
				throw new Error(errorData || 'Failed to complete GitHub app installation')
			}

			isSuccess = true
			sendUserToast('GitHub app installed successfully', false)
		} catch (error) {
			console.error('Error during GitHub app installation:', error)
			errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
			sendUserToast(`Error installing GitHub app: ${errorMessage}`, true)
		} finally {
			isLoading = false
		}
	})
</script>

<div class="h-screen w-screen flex items-center justify-center bg-surface p-4">
	<div class="w-full max-w-3xl p-8 rounded-md border bg-surface shadow-sm">
		{#if isLoading}
			<div class="flex flex-col items-center justify-center py-8">
				<div class="animate-spin h-8 w-8 border-2 border-primary border-t-transparent rounded-full mb-4"></div>
				<p class="text-lg text-secondary">Processing GitHub app installation...</p>
			</div>
		{:else if isSuccess}
			<div class="flex flex-col">
				<h1 class="text-2xl font-bold pb-6 flex flex-row items-center gap-2">
					<CheckCircle class="w-8 h-8 text-green-500" />
					Windmill GitHub app installation completed successfully
				</h1>
				<p class="text-secondary mb-8">
					The GitHub app has been successfully installed. You can now close this window and return to Windmill to start using the GitHub integration.
				</p>
				<button
					on:click={closeWindow}
					class="w-full py-2 px-4 bg-green-600 hover:bg-green-700 text-white font-medium rounded transition-colors"
				>
					Close this window
				</button>
			</div>
		{:else}
			<div class="flex flex-col">
				<h1 class="text-2xl font-bold pb-6 flex flex-row items-center gap-2">
					<XCircle class="w-8 h-8 text-red-500" />
					Failed to install Windmill GitHub app
				</h1>
				<p class="text-secondary pb-4">There was an error during the installation process:</p>
				<div class="bg-surface-secondary p-4 rounded border border-red-300 text-red-600 mb-8">
					{errorMessage}
				</div>
				<p class="text-secondary mb-8">
					Please try again or contact your administrator for assistance.
				</p>
				<button
					on:click={closeWindow}
					class="w-full py-2 px-4 bg-red-600 hover:bg-red-700 text-white font-medium rounded transition-colors"
				>
					Close this window
				</button>
			</div>
		{/if}
	</div>
</div>
