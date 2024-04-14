import { useWorkerFactory } from 'monaco-editor-wrapper/workerFactory'

export const configureMonacoWorkers = () => {
	useWorkerFactory({
		basePath: '../../../node_modules'
	})
}
