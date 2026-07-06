<script lang="ts">
	import { onMount } from 'svelte'
	import { ShieldOff, X } from 'lucide-svelte'
	import { SettingService } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { sendUserToast } from '$lib/toast'

	let show = $state(false)
	let confirmingDismiss = $state(false)
	let dismissing = $state(false)

	onMount(async () => {
		try {
			// Raw fetch (small, banner-only endpoint) rather than a generated
			// client method; mirrors the login page's is_first_time_setup check.
			const res = await fetch('/api/settings/no_auth_banner')
			if (res.ok) {
				show = (await res.json()) === true
			}
		} catch {
			// A banner failure should never break the app; just don't show it.
		}
	})

	async function dismissForever() {
		dismissing = true
		try {
			await SettingService.setGlobal({
				key: 'no_auth_banner_dismissed',
				requestBody: { value: true }
			})
			show = false
			confirmingDismiss = false
		} catch (e) {
			sendUserToast(`Could not dismiss the banner: ${e}`, true)
		} finally {
			dismissing = false
		}
	}
</script>

{#if show}
	<div
		class="w-full bg-red-100 dark:bg-red-900/40 border-b border-red-300 dark:border-red-800 text-red-900 dark:text-red-100"
		role="alert"
	>
		<div class="px-4 py-1.5 flex items-center justify-between gap-3">
			<div class="flex items-center gap-2 min-w-0">
				<ShieldOff size={16} class="shrink-0" />
				<span class="text-xs font-medium truncate">
					Authentication is disabled (NO_AUTH mode): every request is treated as the
					<span class="font-semibold">admin@windmill.dev</span> superadmin. Only run this behind a trusted
					gateway.
				</span>
			</div>
			<Button
				variant="subtle"
				unifiedSize="sm"
				iconOnly
				startIcon={{ icon: X }}
				title="Dismiss for all users permanently"
				onclick={() => (confirmingDismiss = true)}
			/>
		</div>
	</div>

	<ConfirmationModal
		open={confirmingDismiss}
		title="Hide the NO_AUTH warning?"
		confirmationText="Hide for everyone"
		loading={dismissing}
		onConfirmed={dismissForever}
		onCanceled={() => (confirmingDismiss = false)}
	>
		<span class="text-sm">
			This hides the authentication-disabled warning for <span class="font-semibold"
				>all users, permanently</span
			>. Authentication stays disabled — this only removes the banner. It can be shown again by
			unsetting the <code>no_auth_banner_dismissed</code> instance setting.
		</span>
	</ConfirmationModal>
{/if}
