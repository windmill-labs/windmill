<!-- <script lang="ts">
	/* --------------------------------------------------------------------------------------------
	 * Copyright (c) 2018-2022 TypeFox GmbH (http://www.typefox.io). All rights reserved.
	 * Licensed under the MIT License. See License.txt in the project root for license information.
	 * ------------------------------------------------------------------------------------------ */

	import * as monaco from 'monaco-editor'
	import * as vscode from 'vscode'
	import '@codingame/monaco-vscode-python-default-extension'
	import { createConfiguredEditor, createModelReference } from 'vscode/monaco'
	import { registerExtension } from 'vscode/extensions'
	import { updateUserConfiguration } from '@codingame/monaco-vscode-configuration-service-override'
	import nord from '$lib/assets/nord.json'

	import { MonacoLanguageClient } from 'monaco-languageclient'
	import { CloseAction, ErrorAction, MessageTransports } from 'vscode-languageclient'
	import { WebSocketMessageReader, WebSocketMessageWriter, toSocket } from 'vscode-ws-jsonrpc'
	import {
		RegisteredFileSystemProvider,
		registerFileSystemOverlay,
		RegisteredMemoryFile
	} from '@codingame/monaco-vscode-files-service-override'

	import { buildWorkerDefinition } from 'monaco-editor-workers'
	import { onMount } from 'svelte'
	import { initializeVscode } from './vscode'
	buildWorkerDefinition(
		'../../../node_modules/monaco-editor-workers/dist/workers/',
		new URL('', window.location.href).href,
		false
	)

	const createUrl = (
		hostname: string,
		port: number,
		path: string,
		searchParams: Record<string, any> = {},
		secure: boolean = location.protocol === 'https:'
	): string => {
		const protocol = secure ? 'wss' : 'ws'
		const url = new URL(`${protocol}://${hostname}:${port}${path}`)

		for (let [key, value] of Object.entries(searchParams)) {
			if (value instanceof Array) {
				value = value.join(',')
			}
			if (value) {
				url.searchParams.set(key, value)
			}
		}

		return url.toString()
	}

	const languageId = 'python'
	let languageClient: MonacoLanguageClient

	const createWebSocket = (url: string): WebSocket => {
		const webSocket = new WebSocket(url)
		webSocket.onopen = async () => {
			const socket = toSocket(webSocket)
			const reader = new WebSocketMessageReader(socket)
			const writer = new WebSocketMessageWriter(socket)
			languageClient = createLanguageClient({
				reader,
				writer
			})
			await languageClient.start()
			reader.onClose(() => languageClient.stop())
		}
		return webSocket
	}

	const createLanguageClient = (transports: MessageTransports): MonacoLanguageClient => {
		return new MonacoLanguageClient({
			name: 'Pyright Language Client',
			clientOptions: {
				// use a language id as a document selector
				documentSelector: [languageId],
				// disable the default error handler
				errorHandler: {
					error: () => ({ action: ErrorAction.Continue }),
					closed: () => ({ action: CloseAction.DoNotRestart })
				},
				// pyright requires a workspace folder to be present, otherwise it will not work
				workspaceFolder: {
					index: 0,
					name: 'workspace',
					uri: monaco.Uri.parse('/workspace')
				},
				synchronize: {
					fileEvents: [vscode.workspace.createFileSystemWatcher('**')]
				}
			},
			// create a language client connection from the JSON RPC connection on demand
			connectionProvider: {
				get: () => {
					return Promise.resolve(transports)
				}
			}
		})
	}

	export const startPythonClient = async () => {
		await initializeVscode()
		// extension configuration derived from:
		// https://github.com/microsoft/pyright/blob/main/packages/vscode-pyright/package.json
		// only a minimum is required to get pyright working
		const extension = {
			name: 'python-client',
			publisher: 'monaco-languageclient-project',
			version: '1.0.0',
			engines: {
				vscode: '^1.78.0'
			},
			contributes: {
				languages: [
					{
						id: languageId,
						aliases: ['Python'],
						extensions: ['.py', '.pyi']
					}
				],
				commands: [
					{
						command: 'pyright.restartserver',
						title: 'Pyright: Restart Server',
						category: 'Pyright'
					},
					{
						command: 'pyright.organizeimports',
						title: 'Pyright: Organize Imports',
						category: 'Pyright'
					}
				],
				keybindings: [
					{
						key: 'ctrl+k',
						command: 'pyright.restartserver',
						when: 'editorTextFocus'
					}
				]
			}
		}
		registerExtension(extension, 1)

		updateUserConfiguration(`{
        "editor.fontSize": 14,
        "workbench.colorTheme": "Default Dark Modern"
    }`)

		const fileSystemProvider = new RegisteredFileSystemProvider(false)
		fileSystemProvider.registerFile(
			new RegisteredMemoryFile(vscode.Uri.file('/workspace/hello.py'), 'print("Hello, World!")')
		)
		registerFileSystemOverlay(1, fileSystemProvider)

		// create the web socket and configure to start the language client on open, can add extra parameters to the url if needed.
		createWebSocket(
			createUrl(
				'localhost',
				30001,
				'/pyright',
				{
					// Used to parse an auth token or additional parameters such as import IDs to the language server
					authorization: 'UserAuth'
					// By commenting above line out and commenting below line in, connection to language server will be denied.
					// authorization: 'FailedUserAuth'
				},
				false
			)
		)

		const registerCommand = async (cmdName: string, handler: (...args: unknown[]) => void) => {
			// commands sould not be there, but it demonstrates how to retrieve list of all external commands
			const commands = await vscode.commands.getCommands(true)
			if (!commands.includes(cmdName)) {
				vscode.commands.registerCommand(cmdName, handler)
			}
		}
		// always exectute the command with current language client
		await registerCommand('pyright.restartserver', (...args: unknown[]) => {
			languageClient.sendRequest('workspace/executeCommand', {
				command: 'pyright.restartserver',
				arguments: args
			})
		})
		await registerCommand('pyright.organizeimports', (...args: unknown[]) => {
			languageClient.sendRequest('workspace/executeCommand', {
				command: 'pyright.organizeimports',
				arguments: args
			})
		})

		// use the file create before
		const modelRef = await createModelReference(monaco.Uri.file('/workspace/hello.py'))
		modelRef.object.setLanguageId(languageId)

		// create monaco editor
		createConfiguredEditor(document.getElementById('container')!, {
			model: modelRef.object.textEditorModel,
			automaticLayout: true
		})
	}
	onMount(async () => {
		await startPythonClient()
	})
</script>

<div id="container" class="editor" style="width:800px;height:600px;border:1px solid grey" /> -->
