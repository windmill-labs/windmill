
// this file is generated — do not edit it


/// <reference types="@sveltejs/kit" />

/**
 * Environment variables [loaded by Vite](https://vitejs.dev/guide/env-and-mode.html#env-files) from `.env` files and `process.env`. Like [`$env/dynamic/private`](https://kit.svelte.dev/docs/modules#$env-dynamic-private), this module cannot be imported into client-side code. This module only includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://kit.svelte.dev/docs/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://kit.svelte.dev/docs/configuration#env) (if configured).
 * 
 * _Unlike_ [`$env/dynamic/private`](https://kit.svelte.dev/docs/modules#$env-dynamic-private), the values exported from this module are statically injected into your bundle at build time, enabling optimisations like dead code elimination.
 * 
 * ```ts
 * import { API_KEY } from '$env/static/private';
 * ```
 * 
 * Note that all environment variables referenced in your code should be declared (for example in an `.env` file), even if they don't have a value until the app is deployed:
 * 
 * ```
 * MY_FEATURE_FLAG=""
 * ```
 * 
 * You can override `.env` values from the command line like so:
 * 
 * ```bash
 * MY_FEATURE_FLAG="enabled" npm run dev
 * ```
 */
declare module '$env/static/private' {
	export const SHELL: string;
	export const npm_command: string;
	export const LSCOLORS: string;
	export const npm_config_userconfig: string;
	export const COLORTERM: string;
	export const GTK_THEME: string;
	export const GDK_DPI_SCALE: string;
	export const npm_config_cache: string;
	export const npm_package_dev_optional: string;
	export const LESS: string;
	export const npm_package_integrity: string;
	export const I3SOCK: string;
	export const NODE: string;
	export const CREDENTIALS_DIRECTORY: string;
	export const MEMORY_PRESSURE_WRITE: string;
	export const AWS_CLI_AUTO_PROMPT: string;
	export const COLOR: string;
	export const npm_config_local_prefix: string;
	export const GREP_COLORS: string;
	export const npm_config_globalconfig: string;
	export const XCURSOR_SIZE: string;
	export const EDITOR: string;
	export const XDG_SEAT: string;
	export const PWD: string;
	export const LOGNAME: string;
	export const XDG_SESSION_TYPE: string;
	export const GOOGLE_APPLICATION_CREDENTIALS: string;
	export const npm_package_dev: string;
	export const npm_config_init_module: string;
	export const SYSTEMD_EXEC_PID: string;
	export const _: string;
	export const MOTD_SHOWN: string;
	export const HOME: string;
	export const npm_package_peer: string;
	export const LANG: string;
	export const LS_COLORS: string;
	export const XDG_CURRENT_DESKTOP: string;
	export const npm_package_version: string;
	export const MEMORY_PRESSURE_WATCH: string;
	export const SWAYSOCK: string;
	export const WAYLAND_DISPLAY: string;
	export const npm_package_resolved: string;
	export const INVOCATION_ID: string;
	export const INIT_CWD: string;
	export const npm_lifecycle_script: string;
	export const npm_package_optional: string;
	export const npm_config_npm_version: string;
	export const XDG_SESSION_CLASS: string;
	export const TERM: string;
	export const npm_package_name: string;
	export const npm_config_prefix: string;
	export const USER: string;
	export const NOMAD_ADDR: string;
	export const DISPLAY: string;
	export const npm_lifecycle_event: string;
	export const SHLVL: string;
	export const MOZ_ENABLE_WAYLAND: string;
	export const PAGER: string;
	export const XDG_VTNR: string;
	export const XDG_SESSION_ID: string;
	export const npm_config_user_agent: string;
	export const npm_execpath: string;
	export const UPDATE_ZSH_DAYS: string;
	export const XDG_RUNTIME_DIR: string;
	export const DEBUGINFOD_URLS: string;
	export const npm_package_json: string;
	export const BUN_INSTALL: string;
	export const WLR_DRM_NO_MODIFIERS: string;
	export const npm_config_legacy_peer_deps: string;
	export const npm_config_noproxy: string;
	export const PATH: string;
	export const GDK_SCALE: string;
	export const npm_config_node_gyp: string;
	export const DBUS_SESSION_BUS_ADDRESS: string;
	export const CARGO_INCREMENTAL: string;
	export const npm_config_global_prefix: string;
	export const MAIL: string;
	export const npm_node_execpath: string;
	export const OLDPWD: string;
	export const GOPATH: string;
	export const npm_package_engines_node: string;
	export const CONSUL_HTTP_ADDR: string;
}

/**
 * Similar to [`$env/static/private`](https://kit.svelte.dev/docs/modules#$env-static-private), except that it only includes environment variables that begin with [`config.kit.env.publicPrefix`](https://kit.svelte.dev/docs/configuration#env) (which defaults to `PUBLIC_`), and can therefore safely be exposed to client-side code.
 * 
 * Values are replaced statically at build time.
 * 
 * ```ts
 * import { PUBLIC_BASE_URL } from '$env/static/public';
 * ```
 */
declare module '$env/static/public' {
	
}

/**
 * This module provides access to runtime environment variables, as defined by the platform you're running on. For example if you're using [`adapter-node`](https://github.com/sveltejs/kit/tree/main/packages/adapter-node) (or running [`vite preview`](https://kit.svelte.dev/docs/cli)), this is equivalent to `process.env`. This module only includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://kit.svelte.dev/docs/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://kit.svelte.dev/docs/configuration#env) (if configured).
 * 
 * This module cannot be imported into client-side code.
 * 
 * Dynamic environment variables cannot be used during prerendering.
 * 
 * ```ts
 * import { env } from '$env/dynamic/private';
 * console.log(env.DEPLOYMENT_SPECIFIC_VARIABLE);
 * ```
 * 
 * > In `dev`, `$env/dynamic` always includes environment variables from `.env`. In `prod`, this behavior will depend on your adapter.
 */
declare module '$env/dynamic/private' {
	export const env: {
		SHELL: string;
		npm_command: string;
		LSCOLORS: string;
		npm_config_userconfig: string;
		COLORTERM: string;
		GTK_THEME: string;
		GDK_DPI_SCALE: string;
		npm_config_cache: string;
		npm_package_dev_optional: string;
		LESS: string;
		npm_package_integrity: string;
		I3SOCK: string;
		NODE: string;
		CREDENTIALS_DIRECTORY: string;
		MEMORY_PRESSURE_WRITE: string;
		AWS_CLI_AUTO_PROMPT: string;
		COLOR: string;
		npm_config_local_prefix: string;
		GREP_COLORS: string;
		npm_config_globalconfig: string;
		XCURSOR_SIZE: string;
		EDITOR: string;
		XDG_SEAT: string;
		PWD: string;
		LOGNAME: string;
		XDG_SESSION_TYPE: string;
		GOOGLE_APPLICATION_CREDENTIALS: string;
		npm_package_dev: string;
		npm_config_init_module: string;
		SYSTEMD_EXEC_PID: string;
		_: string;
		MOTD_SHOWN: string;
		HOME: string;
		npm_package_peer: string;
		LANG: string;
		LS_COLORS: string;
		XDG_CURRENT_DESKTOP: string;
		npm_package_version: string;
		MEMORY_PRESSURE_WATCH: string;
		SWAYSOCK: string;
		WAYLAND_DISPLAY: string;
		npm_package_resolved: string;
		INVOCATION_ID: string;
		INIT_CWD: string;
		npm_lifecycle_script: string;
		npm_package_optional: string;
		npm_config_npm_version: string;
		XDG_SESSION_CLASS: string;
		TERM: string;
		npm_package_name: string;
		npm_config_prefix: string;
		USER: string;
		NOMAD_ADDR: string;
		DISPLAY: string;
		npm_lifecycle_event: string;
		SHLVL: string;
		MOZ_ENABLE_WAYLAND: string;
		PAGER: string;
		XDG_VTNR: string;
		XDG_SESSION_ID: string;
		npm_config_user_agent: string;
		npm_execpath: string;
		UPDATE_ZSH_DAYS: string;
		XDG_RUNTIME_DIR: string;
		DEBUGINFOD_URLS: string;
		npm_package_json: string;
		BUN_INSTALL: string;
		WLR_DRM_NO_MODIFIERS: string;
		npm_config_legacy_peer_deps: string;
		npm_config_noproxy: string;
		PATH: string;
		GDK_SCALE: string;
		npm_config_node_gyp: string;
		DBUS_SESSION_BUS_ADDRESS: string;
		CARGO_INCREMENTAL: string;
		npm_config_global_prefix: string;
		MAIL: string;
		npm_node_execpath: string;
		OLDPWD: string;
		GOPATH: string;
		npm_package_engines_node: string;
		CONSUL_HTTP_ADDR: string;
		[key: `PUBLIC_${string}`]: undefined;
		[key: `${string}`]: string | undefined;
	}
}

/**
 * Similar to [`$env/dynamic/private`](https://kit.svelte.dev/docs/modules#$env-dynamic-private), but only includes variables that begin with [`config.kit.env.publicPrefix`](https://kit.svelte.dev/docs/configuration#env) (which defaults to `PUBLIC_`), and can therefore safely be exposed to client-side code.
 * 
 * Note that public dynamic environment variables must all be sent from the server to the client, causing larger network requests — when possible, use `$env/static/public` instead.
 * 
 * Dynamic environment variables cannot be used during prerendering.
 * 
 * ```ts
 * import { env } from '$env/dynamic/public';
 * console.log(env.PUBLIC_DEPLOYMENT_SPECIFIC_VARIABLE);
 * ```
 */
declare module '$env/dynamic/public' {
	export const env: {
		[key: `PUBLIC_${string}`]: string | undefined;
	}
}
