<script lang="ts">
	/*
	 * WIN-2006: deploy-time migration prompt for grandfathered (legacy-unsandboxed)
	 * apps — the publisher makes an explicit sandbox choice on the first re-deploy
	 * instead of silently changing behavior. Shared by the low-code and raw app
	 * editor headers.
	 */
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	type Choice = 'sandbox' | 'unsandboxed' | 'cancel'

	let open = $state(false)
	let resolveChoice: ((choice: Choice) => void) | undefined = $state(undefined)

	/**
	 * Resolve the sandbox choice for a pending deploy. Non-legacy apps resolve
	 * `true` immediately; legacy apps prompt the publisher. The choice recorded
	 * here is what sticks: `disable_sandbox=true` to keep running same-origin
	 * (with viewer consent), or left unset to become sandboxed — and in both
	 * cases `legacy_unsandboxed=false` so the backend (which otherwise preserves
	 * the stored flag across updates) clears the grandfathering now that the
	 * publisher made an explicit choice. Resolves `false` when the publisher
	 * cancels.
	 */
	export function ensureLegacyResolved(policy: any): Promise<boolean> {
		if (!policy?.legacy_unsandboxed) return Promise.resolve(true)
		return new Promise<boolean>((resolve) => {
			resolveChoice = (choice) => {
				// Clear before closing so the close-without-choice fallback ($effect
				// below) doesn't resolve a second time.
				resolveChoice = undefined
				open = false
				if (choice === 'cancel') {
					resolve(false)
					return
				}
				policy.disable_sandbox = choice === 'unsandboxed' ? true : undefined
				policy.legacy_unsandboxed = false
				resolve(true)
			}
			open = true
		})
	}

	$effect(() => {
		// Modal closed without an explicit choice (backdrop click): treat as cancel
		// so the pending deploy promise always settles.
		if (!open && resolveChoice) {
			resolveChoice('cancel')
		}
	})
</script>

<Modal
	bind:open
	title="This app runs without sandbox isolation"
	on:confirmed={() => resolveChoice?.('sandbox')}
	on:canceled={() => resolveChoice?.('cancel')}
>
	<p>
		This app predates app sandbox isolation, so it currently runs with full access to each viewer's
		Windmill session. Choose how it should run from now on:
	</p>
	<ul class="text-xs text-secondary list-disc pl-5 mt-2 space-y-1">
		<li>
			<span class="font-semibold">Enable sandbox isolation</span> — isolates the app from the viewer's
			session (recommended). May break frontend scripts that call broad APIs, or features like IndexedDB
			/ third-party SDKs.
		</li>
		<li>
			<span class="font-semibold">Keep without isolation</span> — keeps full-session access; viewers
			are asked to consent once per app version.
		</li>
	</ul>
	{#snippet actions()}
		<Button variant="accent" size="sm" onClick={() => resolveChoice?.('sandbox')}>
			Enable sandbox isolation
		</Button>
		<Button variant="default" destructive size="sm" onClick={() => resolveChoice?.('unsandboxed')}>
			Keep without isolation
		</Button>
	{/snippet}
</Modal>
