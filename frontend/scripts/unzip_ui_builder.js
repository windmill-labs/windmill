import fs from 'fs'
import https from 'https'
import unzipper from 'unzipper'
import path from 'path'

const zipUrl = 'https://pub-06154ed168a24e73a86ab84db6bf15d8.r2.dev/ui_builder-9db704a.zip'
const outputZipPath = path.join(process.cwd(), 'tmp.zip')
const extractTo = path.join(process.cwd(), 'static/ui_builder')

// Download zip
https
	.get(zipUrl, (res) => {
		if (res.statusCode !== 200) {
			console.error(`Failed to download zip. Status code: ${res.statusCode}`)
			return
		}

		const fileStream = fs.createWriteStream(outputZipPath)
		res.pipe(fileStream)

		fileStream.on('finish', () => {
			fileStream.close(() => {
				console.log('Download complete. Extracting...')

				fs.createReadStream(outputZipPath)
					.pipe(unzipper.Extract({ path: extractTo }))
					.on('close', () => {
						console.log(`Extraction complete to ${extractTo}`)
						fs.unlinkSync(outputZipPath) // optional cleanup
					})
					.on('error', (err) => {
						console.error('Error during extraction:', err)
					})
			})
		})
	})
	.on('error', (err) => {
		console.error('Download error:', err)
	})
