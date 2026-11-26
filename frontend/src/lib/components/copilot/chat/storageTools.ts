import { z } from 'zod'
import { createToolDef } from './shared'
import { NONE, type SessionTool } from './sessionCapabilities'
import { runOnServer } from './global/backendSessionTools'

/**
 * The workspace's object storage (its large file storage and secondary storages), through the
 * endpoints of the S3 file browser, so the session sees and changes exactly what the user could
 * there. Run on the server, as every GLOBAL tool is, so MCP clients get the same tools.
 */

const storageArg = z
	.string()
	.optional()
	.describe(
		"Name of a secondary storage, as list_s3_files returns them in `storages`. Omit for the workspace's primary storage."
	)

const listS3FilesSchema = z.object({
	storage: storageArg,
	prefix: z
		.string()
		.optional()
		.describe('Folder to list, e.g. `charts/` or `charts/2026/`. Omit for the root.'),
	page_token: z
		.string()
		.optional()
		.describe('The `next_page_token` of a previous call, for the rest of the same folder.'),
	max_keys: z
		.number()
		.int()
		.min(1)
		.max(500)
		.optional()
		.describe('Max number of folders and files to return. Defaults to 100.')
})

const s3KeyArg = z
	.string()
	.describe('Key of the file, as list_s3_files returns it in `s3`, e.g. `charts/sales.csv`.')

const readS3FileSchema = z.object({ s3: s3KeyArg, storage: storageArg })
const deleteS3FileSchema = z.object({ s3: s3KeyArg, storage: storageArg })
const moveS3FileSchema = z.object({
	s3: s3KeyArg,
	dest_s3: z.string().describe('New key of the file, in the same storage.'),
	storage: storageArg
})

const listS3FilesToolDef = createToolDef(
	listS3FilesSchema,
	'list_s3_files',
	"List one folder of the workspace's object storage (S3, Azure Blob or GCS): its subfolders, and its files with size and last modified time. Call again with a subfolder as `prefix` to go deeper, and with `next_page_token` for the rest of a long folder. A call at the root also returns `storages`, the names of the workspace's secondary storages. Each file is an S3 object `{ s3, storage? }`, the shape a script or flow takes as an S3 file input. Only what the user may list is returned, as in the workspace's S3 file browser."
)
const readS3FileToolDef = createToolDef(
	readS3FileSchema,
	'read_s3_file',
	"Read a file in the workspace's object storage: its size and last modified time, and a preview of its content: the start of a text, JSON or CSV file, or the first rows of a Parquet file. Other files, such as images, return their metadata only."
)
const deleteS3FileToolDef = createToolDef(
	deleteS3FileSchema,
	'delete_s3_file',
	"Delete one file from the workspace's object storage. It cannot be undone: call it only when the user asked for that file to be deleted."
)
const moveS3FileToolDef = createToolDef(
	moveS3FileSchema,
	'move_s3_file',
	"Move or rename one file within one storage of the workspace's object storage: it is copied to `dest_s3`, then the original is deleted. A file already at `dest_s3` is replaced."
)

function storageLabel(storage: string | undefined) {
	return storage ? ` in ${storage}` : ''
}

/** The workspace object storage tools, for registration in global mode. */
export function getStorageTools(): SessionTool<{}>[] {
	return [
		{
			requires: NONE,
			def: listS3FilesToolDef,
			planModeSafe: true,
			lookup: true,
			showDetails: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const { prefix, storage } = listS3FilesSchema.parse(args)
				const where = (prefix || 'the root') + storageLabel(storage)
				toolCallbacks.setToolStatus(toolId, { content: `Listing ${where}...` })
				const { text } = await runOnServer('list_s3_files', workspace, args)
				const listed = JSON.parse(text)
				toolCallbacks.setToolStatus(toolId, {
					content: `Listed ${listed.folders.length} folder(s), ${listed.files.length} file(s) in ${where}`,
					result: listed
				})
				return text
			}
		},
		{
			requires: NONE,
			def: readS3FileToolDef,
			planModeSafe: true,
			lookup: true,
			showDetails: true,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const { s3, storage } = readS3FileSchema.parse(args)
				toolCallbacks.setToolStatus(toolId, { content: `Reading ${s3}${storageLabel(storage)}...` })
				const { text } = await runOnServer('read_s3_file', workspace, args)
				toolCallbacks.setToolStatus(toolId, {
					content: `Read ${s3}${storageLabel(storage)}`,
					result: JSON.parse(text)
				})
				return text
			}
		},
		{
			requires: NONE,
			def: deleteS3FileToolDef,
			requiresConfirmation: true,
			confirmAction: 'delete',
			confirmationMessage: (args: any) =>
				`Delete ${args?.s3 ?? 'file'}${storageLabel(args?.storage)} from object storage`,
			queuedLabel: (args) => `Delete ${args?.s3 ?? 'file'}`,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const { s3, storage } = deleteS3FileSchema.parse(args)
				toolCallbacks.setToolStatus(toolId, { content: `Deleting ${s3}...` })
				const { text } = await runOnServer('delete_s3_file', workspace, args)
				toolCallbacks.setToolStatus(toolId, { content: `Deleted ${s3}${storageLabel(storage)}` })
				return text
			}
		},
		{
			requires: NONE,
			def: moveS3FileToolDef,
			requiresConfirmation: true,
			confirmAction: 'apply',
			confirmationMessage: (args: any) =>
				`Move ${args?.s3 ?? 'file'} to ${args?.dest_s3 ?? '…'}${storageLabel(args?.storage)}`,
			queuedLabel: (args) => `Move ${args?.s3 ?? 'file'}`,
			fn: async ({ args, workspace, toolId, toolCallbacks }) => {
				const { s3, dest_s3 } = moveS3FileSchema.parse(args)
				toolCallbacks.setToolStatus(toolId, { content: `Moving ${s3}...` })
				const { text } = await runOnServer('move_s3_file', workspace, args)
				toolCallbacks.setToolStatus(toolId, { content: `Moved ${s3} to ${dest_s3}` })
				return text
			}
		}
	]
}
