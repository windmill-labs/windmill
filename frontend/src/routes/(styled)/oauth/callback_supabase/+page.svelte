<script lang="ts">
	import { goto } from '$lib/navigation'
	import { page } from '$app/state'
	import { sendUserToast } from '$lib/toast'
	import { onMount } from 'svelte'
	import { OauthService } from '$lib/gen'
	import { oauthStore } from '$lib/stores'
	import { hasParkedWizard } from '$lib/components/workspaceSettings/wizardParking'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Loader2 } from 'lucide-svelte'

	const client_name = 'supabase_wizard'

	let error = page.url.searchParams.get('error')
	let code = page.url.searchParams.get('code') ?? undefined
	let state = page.url.searchParams.get('state') ?? undefined

	/**
	 * As the wizard's popup there is no page to land on: the tab behind us is still showing the
	 * flow and is watching for this window to go away. Leaving it open on a full Windmill page
	 * is what strands the caller's button spinning, and declining consent is a normal outcome,
	 * not an edge case.
	 */
	function closeIfPopup(): boolean {
		if (!window.opener) return false
		window.close()
		return true
	}

	/**
	 * Where a failed leg lands when this is not a popup. A parked run has to be handed back its
	 * own page: nothing else consumes the park, so sending it to `/resources` leaves the run in
	 * `sessionStorage` to spring the wizard open on some unrelated later visit.
	 */
	function failureDestination(): string {
		return hasParkedWizard() ? '/workspace_settings?tab=windmill_data_tables' : '/resources'
	}

	onMount(async () => {
		if (error) {
			if (closeIfPopup()) return
			sendUserToast(`Error trying to fetch projects from windmill: ${error}`, true)
			goto(failureDestination())
		} else if (code && state) {
			try {
				const res = await OauthService.connectCallback({
					clientName: client_name,
					requestBody: { code, state }
				})
				// Opened as the data table wizard's popup: hand the token to the tab that is still
				// sitting on the wizard and get out of the way, so nothing has to be resumed.
				if (window.opener) {
					window.opener.postMessage({ type: 'supabase_oauth', res }, window.location.origin)
					window.close()
					return
				}
				$oauthStore = res
				// The data table wizard parks its state before redirecting, so it can be resumed
				// where it left off. Everything else lands on the resources page, which opens the
				// Supabase drawer for this callback.
				if (hasParkedWizard()) {
					goto(`/workspace_settings?tab=windmill_data_tables&callback=${client_name}`)
				} else {
					goto(`/resources?callback=${client_name}`)
				}
			} catch (e) {
				if (closeIfPopup()) return
				sendUserToast(`Error parsing the response token, ${e.body}`, true)
				goto(failureDestination())
			}
		} else {
			if (closeIfPopup()) return
			sendUserToast('Missing code or state as query params', true)
			goto(failureDestination())
		}
	})
</script>

<CenteredPage>
	<PageHeader title="Connection to supabase in progress" />
	<div class="mx-auto w-0">
		<Loader2 class="animate-spin" />
	</div>
</CenteredPage>
