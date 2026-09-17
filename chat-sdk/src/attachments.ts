import type { WindmillChatApi } from './api'
import type { AttachmentUpload } from './types'
import { abortError, isAbortError } from './utils'

/**
 * Where a chat's uploads live in the workspace's object storage. Under `windmill_uploads/`
 * because the default Enterprise storage permissions grant every user write and read there
 * and deny any other top-level prefix: a key outside it is refused for non-admins, both on
 * upload and when the agent's job reads the file back.
 */
export const CHAT_UPLOADS_PREFIX = 'windmill_uploads/chat'

/** What an AI agent step reads out of `user_attachments`. */
export interface UploadedAttachment {
  s3: string
  filename: string
}

/**
 * The extension each type must be stored under. The worker reads an attachment's media type
 * from the key's extension only (`mime_guess` in `windmill-ai/src/image_handler.rs`, falling
 * back to `image/png`), never from the stored content type, so the extension must be true.
 */
const EXTENSION_BY_MEDIA_TYPE: Record<string, string> = {
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'application/pdf': 'pdf'
}

/**
 * The name an attachment is stored under: the picked name with the extension its media type
 * needs, e.g. a `photo.webp` re-encoded to PNG becomes `photo.png`. Other types keep their name.
 */
export function storedAttachmentName(filename: string, mediaType: string): string {
  const extension = EXTENSION_BY_MEDIA_TYPE[mediaType]
  if (!extension) return filename
  const stem = filename.replace(/\.[^./]+$/, '')
  return `${stem || filename}.${extension}`
}

/** The bytes of an attachment as a Blob carrying its media type. */
export function attachmentBlob(attachment: AttachmentUpload): Blob {
  const data =
    typeof attachment.data === 'string'
      ? dataUrlToBlob(attachment.data, 'application/octet-stream')
      : attachment.data
  return attachment.mediaType && attachment.mediaType !== data.type
    ? new Blob([data], { type: attachment.mediaType })
    : data
}

function dataUrlToBlob(dataUrl: string, fallbackType: string): Blob {
  const comma = dataUrl.indexOf(',')
  if (!dataUrl.startsWith('data:') || comma === -1) {
    throw new Error('windmill-chat: an attachment given as a string must be a data: URL')
  }
  const header = dataUrl.slice(5, comma)
  const isBase64 = header.endsWith(';base64')
  const mediaType = (isBase64 ? header.slice(0, -';base64'.length) : header) || fallbackType
  const payload = dataUrl.slice(comma + 1)
  if (!isBase64) return new Blob([decodeURIComponent(payload)], { type: mediaType })
  const binary = atob(payload)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)
  return new Blob([bytes], { type: mediaType })
}

/**
 * Put each attachment in the workspace's object storage and hand back what the agent reads.
 * The key's turn prefix and per-file index keep two files with the same name, in this turn or
 * an earlier one, from overwriting each other; the name stays the last segment.
 */
export async function uploadAttachments(
  api: WindmillChatApi,
  attachments: AttachmentUpload[],
  turnId: string,
  signal?: AbortSignal
): Promise<UploadedAttachment[]> {
  const prefix = `${CHAT_UPLOADS_PREFIX}/${turnId}`
  // One failed upload aborts the rest, and whatever already landed is deleted: no run will
  // read it, and a resend uploads under a fresh prefix. Best effort, so a delete that fails
  // leaves that object behind rather than masking the upload error.
  if (signal?.aborted) throw abortError()
  const batch = new AbortController()
  const abortBatch = () => batch.abort()
  signal?.addEventListener('abort', abortBatch, { once: true })
  try {
    const results = await Promise.allSettled(
      attachments.map(async (attachment, index) => {
        try {
          const blob = attachmentBlob(attachment)
          const filename = storedAttachmentName(
            attachment.name || `attachment-${index + 1}`,
            blob.type
          )
          const { file_key } = await api.uploadFile(`${prefix}/${index}/${filename}`, blob, {
            contentType: blob.type,
            signal: batch.signal
          })
          return { s3: file_key, filename }
        } catch (e) {
          batch.abort()
          throw e
        }
      })
    )
    const uploaded = results.flatMap((r) => (r.status === 'fulfilled' ? [r.value] : []))
    const reasons = results.flatMap((r) => (r.status === 'rejected' ? [r.reason] : []))
    // A stop that lands once every upload has answered still withdraws the batch.
    if (reasons.length === 0 && !signal?.aborted) return uploaded
    await Promise.all(uploaded.map((u) => api.deleteFile(u.s3).catch(() => {})))
    // The failure that started it, not the aborts it caused in the other uploads.
    throw reasons.find((reason) => !isAbortError(reason)) ?? reasons[0] ?? abortError()
  } finally {
    signal?.removeEventListener('abort', abortBatch)
  }
}
