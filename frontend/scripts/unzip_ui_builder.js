import unzipper from 'unzipper'
import path from 'path'

const outputZipPath = path.join(process.cwd(), 'ui_builder.zip')
const extractTo = path.join(process.cwd(), 'static/ui_builder')

import { fileURLToPath } from 'url'
import { dirname, join } from 'path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

// Check if this script is being run from the package root
const isRootInstall = process.cwd() + '/scripts' === __dirname

if (isRootInstall) {
	console.log('Running postinstall: direct install')
	// Your postinstall logic here
} else {
	console.log('Skipping postinstall: installed as dependency')
	process.exit(0)
}

const directory = await unzipper.Open.file(outputZipPath)
await directory.extract({ path: extractTo })

await fs.promises.unlink(outputZipPath)
