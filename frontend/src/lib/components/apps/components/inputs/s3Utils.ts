import { Preview } from '$lib/gen'
import type { AppInput, RunnableByName } from '../../inputType'

export function createS3FileUpload(
	resource: string,
	filename: string,
	file: string
): AppInput | undefined {
	const code = `
  import { S3Client } from "https://deno.land/x/s3_lite_client@0.2.0/mod.ts";
import { ClientOptions } from "https://deno.land/x/s3_lite_client@0.2.0/client.ts";

type S3 = ClientOptions;
type Base64 = string;

export async function main(resource: S3, file: Base64, filename: string) {
const s3Client = new S3Client(resource);

const res = await s3Client.putObject(filename, file);
}
`

	const fileUploadRunnable: RunnableByName = {
		name: 'AppDbExplorer',
		type: 'runnableByName',
		inlineScript: {
			content: code,
			language: Preview.language.DENO,
			schema: {
				$schema: 'https://json-schema.org/draft/2020-12/schema',
				properties: {
					resource: {
						description: '',
						format: 'resource-s3',
						type: 'object'
					},
					file: {
						contentEncoding: 'base64',
						description: '',
						type: 'string',
						format: ''
					},
					filename: {
						description: '',
						type: 'string',
						format: ''
					}
				},
				required: ['resource', 'file', 'filename'],
				type: 'object'
			}
		}
	}

	const input: AppInput = {
		runnable: fileUploadRunnable,
		fields: {
			resource: {
				type: 'static',
				value: resource,
				fieldType: 'object',
				format: 'resource-s3'
			},

			filename: {
				type: 'static',
				value: filename,
				fieldType: 'text'
			},
			file: {
				type: 'static',
				value: file,
				fieldType: 'text'
			}
		},
		type: 'runnable',
		fieldType: 'object'
	}

	return input
}
