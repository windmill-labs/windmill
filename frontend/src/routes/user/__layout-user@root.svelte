<script lang="ts">
	import { SvelteToast } from '@zerodevx/svelte-toast';
	import { logout, sendUserToast } from '../../utils';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { superadmin, userStore, workspaceStore } from '../../stores';
	import { UserService } from '../../gen';

	// Default options
	const toastOptions = {
		duration: 4000, // duration of progress bar tween to the `next` value
		initial: 1, // initial progress bar value
		next: 0, // next progress value
		pausable: false, // pause progress bar tween on mouse hover
		dismissable: true, // allow dismiss with close button
		reversed: false, // insert new toast to bottom of stack
		intro: { x: 256 }, // toast intro fly animation settings
		theme: {} // css var overrides
	};

	$: {
		if ($workspaceStore) {
			localStorage.setItem('workspace', $workspaceStore);
		}
	}

	onMount(() => {
		window.onunhandledrejection = (e) => {
			e.preventDefault();
			if (e.reason && e.reason.message) {
				if (
					['Model not found', 'Connection is disposed.', 'Connection got disposed.'].includes(
						e.reason.message
					)
				) {
					// monaco editor promise cancelation
					console.log('caught expected error');
				}
				if (e.reason.status == '401') {
					sendUserToast('Logged out after a request was unauthorized', true);
					logout($page.url.pathname);
				} else {
					let message = `${e.reason?.message}: ${e.reason?.body ?? ''}`;
					sendUserToast(message, true);
				}
			} else {
				console.log('unexpected error ignored', e);
			}
			e.preventDefault();
			return false;
		};
		workspaceStore.set(undefined);
		userStore.set(undefined);
		if ($superadmin == undefined) {
			UserService.globalWhoami().then((x) => {
				if (x.super_admin) {
					superadmin.set(x.email);
				} else {
					superadmin.set(false);
				}
			});
		}
	});
</script>

<div class="min-h-screen antialiased text-gray-900">
	<slot />
	<SvelteToast {toastOptions} />
</div>

<style>
	:root {
		--toastBackground: #eff6ff;
		--toastBarBackground: #eff6ff;
		--toastColor: #123456;
	}
</style>
