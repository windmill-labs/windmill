// patch-editor-worker.mjs
import { readFile, writeFile } from 'fs/promises'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const filePath = path.join(
	__dirname,
	'..',
	'node_modules',
	'@codingame',
	'monaco-vscode-api',
	'workers',
	'editor.worker.js'
)

const originalSnippet = `return requestHandler?.[propKey];`

const patchedCode = `
import '../vscode/src/vs/editor/common/services/editorWebWorkerMain.js';
import { start } from '../vscode/src/vs/editor/editor.worker.start.js';

function initialize(createFn) {
    let requestHandler;
    const foreignModule = new Proxy({}, {
        get(_target, propKey) {
            if (propKey === '$initialize') {
                return async (data) => {
                    if (!requestHandler) {
                        requestHandler = createFn(context, data);
                    }
                };
            }
            const value = requestHandler?.[propKey]
            if (typeof value === 'function') {
              return value.bind(requestHandler);
            }
            return value;
        }
    });
    const context = start(foreignModule);
}

export { initialize };
`

try {
	const current = await readFile(filePath, 'utf8')
	if (current.includes(originalSnippet)) {
		await writeFile(filePath, patchedCode, 'utf8')
		console.log('✅ editor.worker.js patched successfully.')
	} else {
		console.log('ℹ️ Patch already applied or file has changed.')
	}
} catch (err) {
	console.error('❌ Failed to patch editor.worker.js:', err)
}
