import type { WindmillChatApi } from './api'
import type { ChatAttachment } from './types'
import { isAbortError } from './utils'

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
 * The extension each type a chat composer sends must be stored under.
 *
 * The worker reads an attachment's media type from the object key and nothing else —
 * `mime_guess::from_path` in `windmill-ai/src/image_handler.rs`, falling back to
 * `image/png` when it can read no extension — and never from the content type stored
 * beside it. So the key's extension is a claim about the bytes, and it has to be true.
 */
const EXTENSION_BY_MEDIA_TYPE: Record<string, string> = {
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'application/pdf': 'pdf'
}

/**
 * The name an attachment is stored under. A composer commonly re-encodes every image to PNG
 * or JPEG, so keeping the picked `photo.webp` would hand the provider PNG bytes labelled webp,
 * which Anthropic rejects outright; and a PDF picked without an extension would be read back
 * as the `image/png` fallback. A type not listed is left as picked — other files upload byte
 * for byte, so their name is already true.
 */
export function storedAttachmentName(filename: string, mediaType: string): string {
  const extension = EXTENSION_BY_MEDIA_TYPE[mediaType]
  if (!extension) return filename
  const stem = filename.replace(/\.[^./]+$/, '')
  return `${stem || filename}.${extension}`
}

/** The bytes of an attachment as a Blob carrying its media type. */
export function attachmentBlob(attachment: ChatAttachment): Blob {
  const data = attachment.data
  if (typeof data !== 'string') {
    return attachment.mediaType && attachment.mediaType !== data.type
      ? new Blob([data], { type: attachment.mediaType })
      : data
  }
  return dataUrlToBlob(data, attachment.mediaType ?? 'application/octet-stream')
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
 * The flow runs on a worker, so the bytes have to exist somewhere the worker can fetch.
 *
 * A prefix per turn, and a segment per attachment inside it. The turn's prefix keeps a
 * re-attached filename off the copy an earlier message still points at; the segment does the
 * same within one turn, where two files can arrive under one name and would otherwise race to
 * a single key and leave the agent reading one of them twice. The name itself stays the last
 * segment, so anything that reads a name off the key still sees what the user attached.
 */
export async function uploadAttachments(
  api: WindmillChatApi,
  attachments: ChatAttachment[],
  turnId: string,
  signal?: AbortSignal
): Promise<UploadedAttachment[]> {
  const prefix = `${CHAT_UPLOADS_PREFIX}/${turnId}`
  // One failed upload aborts the rest, and whatever already landed is deleted: no run will
  // read it, and a resend uploads under a fresh prefix. Best effort, so a delete that fails
  // leaves that object behind rather than masking the upload error.
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
    if (reasons.length === 0) return uploaded
    await Promise.all(uploaded.map((u) => api.deleteFile(u.s3).catch(() => {})))
    // The failure that started it, not the aborts it caused in the other uploads.
    throw reasons.find((reason) => !isAbortError(reason)) ?? reasons[0]
  } finally {
    signal?.removeEventListener('abort', abortBatch)
  }
}
