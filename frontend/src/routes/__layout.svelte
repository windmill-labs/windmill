<script lang="ts">
	import '../app.css';

	import { OpenAPI, UserService, WorkspaceService } from '../gen';
	import { logout, clickOutside, sendUserToast, logoutWithRedirect, getUser } from '../utils';
	import { onDestroy, onMount } from 'svelte';
	import Icon from 'svelte-awesome';
	import {
		faScroll,
		faPlay,
		faWallet,
		faEye,
		faChevronDown,
		faChevronRight,
		faChevronLeft,
		faBookOpen,
		faCubes,
		faCalendar,
		faRobot,
		faCog,
		faUser,
		faCrown,
		faUsersCog,
		faWind
	} from '@fortawesome/free-solid-svg-icons';
	import { SvelteToast } from '@zerodevx/svelte-toast';
	import { faDiscord, faGithub, faPython } from '@fortawesome/free-brands-svg-icons';
	import { page } from '$app/stores';
	import {
		superadmin,
		usernameStore,
		userStore,
		usersWorkspaceStore,
		workspaceStore
	} from '../stores';
	import { goto } from '$app/navigation';

	OpenAPI.WITH_CREDENTIALS = true;

	// Default toast options
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

	let menuOpen = false;
	let workspacePickerOpen = false;
	let isMobile = false;
	let viewportWidth = 3000;
	let isCollapsed = false;

	function openMenu(): void {
		menuOpen = true;
	}

	function handleClickOutside(event: any): void {
		if (isMobile || viewportWidth < 640) {
			isCollapsed = true;
		}
	}

	function handleClickOutsideMenu(event: any): void {
		menuOpen = false;
	}
	function handleClickOutsideWorkspacePicker(event: any): void {
		workspacePickerOpen = false;
	}

	async function loadUserInfo() {
		if ($superadmin == undefined) {
			UserService.globalWhoami().then((x) => {
				if (x.super_admin) {
					superadmin.set(x.email);
				} else {
					superadmin.set(false);
				}
			});
		}

		if (!$usersWorkspaceStore) {
			try {
				usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces());
			} catch {}
		}
		if ($usersWorkspaceStore) {
			if (!$workspaceStore) {
				workspaceStore.set(localStorage.getItem('workspace')?.toString());
			}
			if ($workspaceStore && $usernameStore) {
				await getUser($workspaceStore);
			} else if ($superadmin) {
				console.log('You are a superadmin, you can go wherever you please');
			} else {
				goto('/user/workspaces');
			}
		} else {
			logoutWithRedirect($page.url.pathname);
		}
	}

	$: {
		if ($workspaceStore) {
			localStorage.setItem('workspace', $workspaceStore);
		}
	}

	onMount(() => {
		loadUserInfo();
		window.onunhandledrejection = (e) => {
			if (e.reason && e.reason.message) {
				if (
					['Model not found', 'Connection is disposed.', 'Connection got disposed.'].includes(
						e.reason.message
					)
				) {
					// monaco editor promise cancelation
					console.log('caught expected error');
				} else {
					if (e.reason.status == '401') {
						sendUserToast('Logged out after a request was unauthorized', true);
						logout($page.url.pathname);
					} else {
						let message = `${e.reason?.message}: ${e.reason?.body ?? ''}`;
						sendUserToast(message, true);
					}
				}
			} else {
				console.log('unexpected error ignored', e);
			}
			e.preventDefault();
			return false;
		};

		isMobile = /iPhone|iPad|iPod|Android/i.test(navigator.userAgent);
		//Mobile
		isCollapsed = isMobile;
	});
</script>

<div bind:clientWidth={viewportWidth} class="h-full max-w-screen">
	<nav
		use:clickOutside
		on:click_outside={handleClickOutside}
		class="flex flex-col fixed h-screen {isCollapsed
			? 'w-8'
			: 'w-36'} bg-blue-500 rounded-sm text-white z-10"
	>
		<div class="shrink">
			<button
				class="w-full flex flex-row-reverse transform hover:translate-x-1 transition-transform ease-in duration-200"
				on:click={() => {
					isCollapsed = !isCollapsed;
				}}
			>
				<div class="pt-1 pr-3">
					<Icon data={isCollapsed ? faChevronRight : faChevronLeft} scale={0.9} />
				</div>
			</button>
		</div>
		<ul class="flex flex-col {isCollapsed ? 'items-center' : 'px-6'}  bg-transparent pb-1">
			<li class="z-20 font-medium items-center bg-transparent">
				<div
					class="flex justify-center"
					use:clickOutside
					on:click_outside={handleClickOutsideWorkspacePicker}
				>
					<button
						type="button"
						class="flex text-sm focus:outline-none bg-transparent"
						id="user-menu-button"
						aria-expanded="false"
						aria-haspopup="true"
					>
						<span class="sr-only">Open user menu</span>
						<div
							class="flex flex-row items-center w-full justify-content"
							on:click={() => {
								workspacePickerOpen = true;
							}}
						>
							<span class:hidden={isCollapsed} class="pr-2 font-mono text-xs flex"
								>{$workspaceStore ?? '_______'}
								<Icon class="text-white float-right mt-1 pl-1" data={faChevronDown} scale={0.6} />
							</span>
							<span class:hidden={!isCollapsed}>W</span>
						</div>
					</button>
					<div
						class="absolute {isCollapsed
							? 'left-4'
							: 'left-20'} -top-2 mt-2 w-52 rounded-md shadow-lg py-1 bg-white ring-1 ring-black ring-opacity-5 focus:outline-none {workspacePickerOpen
							? 'visible'
							: 'invisible'} z-40"
						role="menu"
						tabindex="-1"
					>
						{#each $usersWorkspaceStore?.workspaces ?? [] as workspace}
							<button
								on:click={() => {
									workspaceStore.set(workspace.id);
									workspacePickerOpen = false;
								}}
								class="block px-4 py-2 text-xs text-gray-500 "
								role="menuitem"
								tabindex="-1"
								id="user-menu-item-2"
							>
								<span class="text-gray-300 font-mono pr-1 text-xs">{workspace.id}</span
								>{workspace.name}
							</button>
						{/each}
						<a
							href="/user/create_workspace"
							class="block px-4 py-2  text-blue-600 text-left text-xs "
							role="menuitem"
							tabindex="-1"
							id="user-menu-item-2"
						>
							<span class="text-gray-300 font-mono pr-1">+</span>Create new workspace</a
						>
						<a
							href="/user/workspaces"
							class="block px-4 py-2  text-blue-600 text-left text-xs "
							role="menuitem"
							tabindex="-1"
							id="user-menu-item-2"
							on:click={() => {
								localStorage.removeItem('workspace');
							}}
						>
							See all workspaces & invites</a
						>
					</div>
				</div>
			</li>
		</ul>
		<ul class="flex flex-col max-h-12 mt-2 {isCollapsed ? 'items-center' : 'px-6 '}   ">
			<li class="relative z-30 font-medium">
				<div
					class="flex justify-center"
					use:clickOutside
					on:click_outside={handleClickOutsideMenu}
					on:click={openMenu}
				>
					<button
						type="button"
						class="flex text-sm rounded-full focus:outline-none"
						id="user-menu-button"
						aria-expanded="false"
						aria-haspopup="true"
					>
						<span class="sr-only">Open user menu</span>
						<div class="mx-auto">
							<span class:hidden={isCollapsed} class="px-2 font-mono text-xs whitespace-nowrap">
								<Icon class="text-white" data={faUser} scale={0.6} />
								{$usernameStore ?? $superadmin ?? '___'}
								{#if $userStore?.is_admin}
									<Icon class="text-white" data={faCrown} scale={0.6} />
								{/if}
								<Icon class="inline text-white mt-1 ml-1" data={faChevronDown} scale={0.6} />
							</span>
						</div>
					</button>
					<div
						class="absolute {isCollapsed
							? 'left-4'
							: 'left-20'} -top-5 mt-2 w-48 rounded-md shadow-lg py-1 bg-white ring-1 ring-black ring-opacity-5 focus:outline-none {menuOpen
							? 'visible'
							: 'invisible'}"
						role="menu"
						tabindex="-1"
					>
						<span class="block px-4 py-2 text-sm text-gray-500">{$usersWorkspaceStore?.email}</span>
						<a
							href="/settings"
							class="block px-4 py-2 text-sm text-gray-700"
							role="menuitem"
							tabindex="-1"
							id="user-menu-item-1">User settings</a
						>
						<button
							on:click={() => logout()}
							class="block px-4 py-2 text-sm text-gray-700"
							role="menuitem"
							tabindex="-1"
							id="user-menu-item-2">Sign out</button
						>
					</div>
				</div>
			</li>
			<div class="border-t border-gray-300 border-opacity-30 my-2" />
		</ul>
		<div class="grow h-full" />
		<ul class="flex flex-col {isCollapsed ? 'items-center' : 'px-6'} ">
			<li>
				<a href="/scripts" class="menu-link text-sm font-medium items-center ">
					<Icon class="text-white" data={faScroll} scale={0.9} />
					<span class=" pl-2 {isCollapsed ? 'hidden' : ''}">Scripts</span>
				</a>
			</li>
			<li>
				<a href="/flows" class="menu-link text-sm font-medium items-center">
					<Icon class="text-white" data={faWind} scale={0.9} />
					<span class="pl-2 {isCollapsed ? 'hidden' : ''}">Flows</span>
				</a>
			</li>
			<li>
				<a href="/runs" class="menu-link text-sm font-medium items-center">
					<Icon class="text-white" data={faPlay} scale={0.8} />
					<span class=" pl-2 {isCollapsed ? 'hidden' : ''}">Runs</span>
				</a>
			</li>
			<li>
				<a href="/schedules" class="menu-link text-sm font-medium items-center">
					<Icon class="text-white" data={faCalendar} scale={0.9} />
					<span class=" pl-2 {isCollapsed ? 'hidden' : ''}">Schedules</span>
				</a>
			</li>
			<li>
				<a href="/variables" class="menu-link text-sm font-medium items-center">
					<Icon class="text-white" data={faWallet} scale={0.9} />
					<span class=" pl-2 {isCollapsed ? 'hidden' : ''}">Variables</span>
				</a>
			</li>
			<li>
				<a href="/resources" class="menu-link text-sm font-medium items-center">
					<Icon class="text-white" data={faCubes} scale={0.9} />
					<span class=" pl-2 {isCollapsed ? 'hidden' : ''}">Resources</span>
				</a>
			</li>
		</ul>

		<div class="grow h-full" />
		<ul class="flex flex-col bg-blue {isCollapsed ? 'items-center' : 'px-6'} ">
			<div class="border-t border-gray-300 border-opacity-30" />
			{#if $superadmin}
				<li>
					<a href="/workers" class="menu-link text-sm font-medium items-center">
						<Icon class="text-white" data={faRobot} scale={0.9} />
						<span class=" pl-2 {isCollapsed ? 'hidden' : ''}">Workers</span>
					</a>
				</li>
			{/if}
			<li>
				<a href="/groups" class="menu-link text-sm font-medium items-center">
					<Icon class="text-white" data={faUsersCog} scale={0.9} />
					<span class="pl-2 t {isCollapsed ? 'hidden' : ''}">Groups</span>
				</a>
			</li>
			<li>
				<a href="/audit_logs" class="menu-link text-sm font-medium items-center">
					<Icon class="text-white" data={faEye} scale={0.9} />
					<span class=" pl-2 {isCollapsed ? 'hidden' : ''}">Audit logs</span>
				</a>
			</li>
			{#if $userStore?.is_admin}
				<li>
					<a href="/workspace_settings" class="menu-link text-sm font-medium items-center pb-2">
						<Icon class="text-white" data={faCog} scale={0.9} />
						<span class=" pl-2 {isCollapsed ? 'hidden' : ''}">Workspace</span>
					</a>
				</li>
			{/if}
			<div class="border-t border-gray-300 border-opacity-30" />
		</ul>
		<div class="grow h-full" />
		<ul class="flex flex-col {isCollapsed ? 'items-center' : 'px-6'} block">
			<li class="">
				<a
					href="https://github.com/windmill-labs/windmill/issues/new"
					class="menu-link text-sm font-medium items-center"
					target="_blank"
				>
					<Icon class="text-white" data={faGithub} scale={0.9} />
					<span class="pl-2 {isCollapsed ? 'hidden' : ''}">Issue?</span>
				</a>
			</li>
			<li class="">
				<a
					href="https://discord.gg/V7PM2YHsPB"
					class="menu-link text-sm font-medium items-center"
					target="_blank"
				>
					<Icon class="text-white" data={faDiscord} scale={0.9} />
					<span class="pl-2 {isCollapsed ? 'hidden' : ''}">Feedback</span>
				</a>
			</li>
			<li class="">
				<a
					href="https://docs.windmill.dev/docs/intro"
					class="menu-link text-sm font-medium items-center"
					target="_blank"
				>
					<Icon class="text-white" data={faBookOpen} scale={0.9} />
					<span class="pl-2 {isCollapsed ? 'hidden' : ''}">Docs</span>
				</a>
			</li>
			<li class="">
				<button
					class="h-12 flex flex-row text-sm font-medium min-w-full px-5 items-center transform hover:translate-x-1 transition-transform ease-in duration-200"
					on:click={() => {
						isCollapsed = !isCollapsed;
					}}
				>
					<div class="w-full -ml-4">
						<div class="flex flex-row justify-between w-full">
							<div class="px-3 {isCollapsed ? 'hidden' : ''}">Windmill</div>
							<div class="ml-4">
								<Icon data={isCollapsed ? faChevronRight : faChevronLeft} scale={0.9} />
							</div>
						</div>
					</div>
				</button>
			</li>
		</ul>
	</nav>
	<div
		class="bg-white antialiased text-gray-900 pt-4 {isCollapsed
			? 'pl-12'
			: 'pl-44'} pr-8 flex h-full max-w-screen flex-col items-center"
	>
		<slot />
		<SvelteToast {toastOptions} />
	</div>
</div>

<style>
	:global[menu] {
		@apply text-white;
	}
	.menu-link {
		@apply flex flex-row h-10 transform hover:translate-x-1 transition-transform ease-in duration-200 text-gray-200 hover:text-white;
	}

	:root {
		--toastBackground: #eff6ff;
		--toastBarBackground: #eff6ff;
		--toastColor: #123456;
	}
</style>
