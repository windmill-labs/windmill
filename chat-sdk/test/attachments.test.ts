import { describe, expect, test } from 'bun:test'
import { WindmillChatApi } from '../src/api'
import { storedAttachmentName, uploadAttachments } from '../src/attachments'
import { createChat } from '../src/chat'
import type { ChatOptions } from '../src/types'
import { abortError } from '../src/utils'
import { fetchMock, json, memoryStorage, sse, text, type RecordedCall, type Route } from './support'

const BASE = 'http://wm.test'
const FLOW = 'f/chat/agent'
const UPLOAD_PATH = '/api/w/ws/job_helpers/upload_s3_file'

const run: Route = (c) =>
  c.method === 'POST' && c.url.pathname === `/api/w/ws/jobs/run/f/${FLOW}`
    ? text('job-1')
    : undefined

/** Stores under the key it was asked to, like the server with a `file_key`. */
const upload: Route = (c) =>
  c.url.pathname === UPLOAD_PATH
    ? json({ file_key: c.url.searchParams.get('file_key') })
    : undefined

const answer: Route = (c) =>
  c.url.pathname === '/api/w/ws/jobs_u/getupdate_sse/job-1'
    ? sse([
        {
          type: 'update',
          completed: true,
          only_result: { output: 'ok', messages: [] }
        }
      ])
    : undefined

function options(fetch: ChatOptions['fetch']): ChatOptions {
  return {
    flowPath: FLOW,
    baseUrl: BASE,
    workspace: 'ws',
    token: 'tok',
    fetch,
    storage: memoryStorage()
  }
}

const uploads = (calls: RecordedCall[]) => calls.filter((c) => c.url.pathname === UPLOAD_PATH)
const runs = (calls: RecordedCall[]) =>
  calls.filter((c) => c.url.pathname.startsWith('/api/w/ws/jobs/run/'))

const png = new Blob([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], {
  type: 'image/png'
})
const pdf = new Blob(['%PDF-1.7'], { type: 'application/pdf' })

describe('storedAttachmentName', () => {
  // The worker reads the media type from the key's extension, so it has to match the bytes.
  test('renames a re-encoded image and gives a bare name its extension', () => {
    expect(storedAttachmentName('photo.webp', 'image/png')).toBe('photo.png')
    expect(storedAttachmentName('holiday.png', 'image/jpeg')).toBe('holiday.jpg')
    expect(storedAttachmentName('contract', 'application/pdf')).toBe('contract.pdf')
    expect(storedAttachmentName('report.2026.final.webp', 'image/png')).toBe(
      'report.2026.final.png'
    )
  })

  test('leaves a type it does not know alone', () => {
    expect(storedAttachmentName('notes.csv', 'text/csv')).toBe('notes.csv')
  })
})

describe('sendMessage with attachments', () => {
  test('uploads each file under the turn prefix and hands the list to the input', async () => {
    const { fetch, calls } = fetchMock(upload, run, answer)
    const chat = createChat(options(fetch))

    await chat.sendMessage('read these', {
      inputs: { locale: 'fr' },
      attachments: [
        { name: 'photo.webp', data: png },
        { name: 'contract', data: pdf },
        // A data URL is decoded to its bytes; the mediaType names what they are.
        {
          name: 'photo.webp',
          data: `data:image/png;base64,${btoa('\x89PNG')}`
        }
      ],
      attachmentsInput: { name: 'files', multiple: true }
    })

    const keys = uploads(calls).map((c) => c.url.searchParams.get('file_key')!)
    expect(keys).toHaveLength(3)
    const prefix = keys[0].split('/').slice(0, 3).join('/')
    expect(prefix).toMatch(/^windmill_uploads\/chat\/[0-9a-f-]{36}$/)
    expect(keys).toEqual([
      `${prefix}/0/photo.png`,
      `${prefix}/1/contract.pdf`,
      `${prefix}/2/photo.png`
    ])
    expect(uploads(calls).map((c) => c.url.searchParams.get('content_type'))).toEqual([
      'image/png',
      'application/pdf',
      'image/png'
    ])
    expect(uploads(calls).map((c) => c.headers['content-type'])).toEqual([
      'image/png',
      'application/pdf',
      'image/png'
    ])
    expect(new Uint8Array(await uploads(calls)[2].raw!.arrayBuffer())).toEqual(
      new Uint8Array([0x89, 0x50, 0x4e, 0x47])
    )

    expect(runs(calls)[0].body).toEqual({
      locale: 'fr',
      user_message: 'read these',
      files: [
        { s3: `${prefix}/0/photo.png`, filename: 'photo.png' },
        { s3: `${prefix}/1/contract.pdf`, filename: 'contract.pdf' },
        { s3: `${prefix}/2/photo.png`, filename: 'photo.png' }
      ]
    })
    expect(chat.getState().status).toBe('idle')
  })

  test('hands a single object to an input that holds one file', async () => {
    const { fetch, calls } = fetchMock(upload, run, answer)
    const chat = createChat(options(fetch))
    await chat.sendMessage('read this', {
      attachments: [{ name: 'contract.pdf', data: pdf }],
      attachmentsInput: { name: 'file', multiple: false }
    })
    const body = runs(calls)[0].body as Record<string, unknown>
    expect(body.file).toEqual({
      s3: expect.stringMatching(/\/0\/contract\.pdf$/),
      filename: 'contract.pdf'
    })
  })

  test('refuses several files for an input that holds one, before uploading any', async () => {
    const { fetch, calls } = fetchMock(upload, run, answer)
    const chat = createChat(options(fetch))
    await expect(
      chat.sendMessage('read these', {
        attachments: [
          { name: 'a.pdf', data: pdf },
          { name: 'b.png', data: png }
        ],
        attachmentsInput: { name: 'file', multiple: false }
      })
    ).rejects.toThrow('holds one file')
    expect(calls).toHaveLength(0)
    expect(chat.getState().messages).toEqual([])
  })

  test('refuses attachments without message text, before uploading', async () => {
    const { fetch, calls } = fetchMock(upload, run, answer)
    const chat = createChat(options(fetch))
    await expect(
      chat.sendMessage('  ', {
        attachments: [{ name: 'a.pdf', data: pdf }],
        attachmentsInput: { name: 'files', multiple: true }
      })
    ).rejects.toThrow('need a message')
    expect(calls).toHaveLength(0)
  })

  test('refuses attachments without an input to put them in', async () => {
    const { fetch, calls } = fetchMock(upload, run, answer)
    const chat = createChat(options(fetch))
    await expect(
      chat.sendMessage('hi', { attachments: [{ name: 'a.pdf', data: pdf }] })
    ).rejects.toThrow('attachmentsInput')
    expect(calls).toHaveLength(0)
  })

  test('a failed upload rejects without a run, and withdraws the message', async () => {
    const { fetch, calls } = fetchMock(
      (c) => (c.url.pathname === UPLOAD_PATH ? text('no object storage', 500) : undefined),
      run,
      answer
    )
    const chat = createChat(options(fetch))
    const statuses: string[] = []
    chat.subscribe((s) => statuses.push(s.status))

    await expect(
      chat.sendMessage('read this', {
        attachments: [{ name: 'contract.pdf', data: pdf }],
        attachmentsInput: { name: 'files', multiple: true }
      })
    ).rejects.toThrow('no object storage')

    expect(runs(calls)).toHaveLength(0)
    // Shown as submitted while uploading, then withdrawn whole: no message, no conversation.
    expect(statuses).toContain('submitted')
    const state = chat.getState()
    expect(state.status).toBe('idle')
    expect(state.messages).toEqual([])
    expect(state.conversationId).toBeUndefined()
    expect(state.conversations).toEqual([])
    // The chat is free for the next message.
    await chat.sendMessage('plain')
    expect(runs(calls)).toHaveLength(1)
  })

  test('stop() during the upload aborts it and withdraws the message', async () => {
    const { fetch, calls } = fetchMock(
      (c) =>
        c.url.pathname === UPLOAD_PATH
          ? new Promise((_, reject) =>
              c.signal!.addEventListener('abort', () => reject(abortError()))
            )
          : undefined,
      run,
      answer
    )
    const chat = createChat(options(fetch))
    const sending = chat.sendMessage('read this', {
      attachments: [{ name: 'contract.pdf', data: pdf }],
      attachmentsInput: { name: 'files', multiple: true }
    })
    await new Promise((r) => setTimeout(r, 0))
    expect(chat.getState().status).toBe('submitted')
    await chat.stop()
    await expect(sending).rejects.toMatchObject({ name: 'AbortError' })
    expect(runs(calls)).toHaveLength(0)
    expect(chat.getState()).toMatchObject({
      status: 'idle',
      messages: [],
      conversationId: undefined
    })
  })

  test('a switch away mid-upload leaves no conversation behind', async () => {
    const storage = memoryStorage()
    const { fetch, calls } = fetchMock(
      (c) =>
        c.url.pathname === UPLOAD_PATH
          ? new Promise((_, reject) =>
              c.signal!.addEventListener('abort', () => reject(abortError()))
            )
          : undefined,
      run,
      answer
    )
    const chat = createChat({ ...options(fetch), storage })
    const sending = chat.sendMessage('never runs', {
      attachments: [{ name: 'contract.pdf', data: pdf }],
      attachmentsInput: { name: 'files', multiple: true }
    })
    await new Promise((r) => setTimeout(r, 0))
    const opened = chat.getState().conversationId!
    chat.newConversation()
    await expect(sending).rejects.toMatchObject({ name: 'AbortError' })

    expect(runs(calls)).toHaveLength(0)
    expect(chat.getState().conversations.map((c) => c.id)).not.toContain(opened)
    const reloaded = createChat({ ...options(fetch), storage })
    expect((await reloaded.loadConversations()).map((c) => c.id)).not.toContain(opened)
  })

  test('a failed upload aborts the rest of its batch and deletes nothing', async () => {
    let first: (r: Response) => void = () => {}
    const { fetch, calls } = fetchMock(
      (c) => {
        if (c.url.pathname !== UPLOAD_PATH) return undefined
        const key = c.url.searchParams.get('file_key')!
        // The first file lands after the second has already failed.
        if (key.includes('/0/')) return new Promise<Response>((resolve) => (first = resolve))
        setTimeout(() => first(json({ file_key: keys()[0] })), 5)
        return text('quota exceeded', 507)
      },
      run,
      answer
    )
    const keys = () => uploads(calls).map((c) => c.url.searchParams.get('file_key')!)
    const chat = createChat(options(fetch))
    await expect(
      chat.sendMessage('read these', {
        attachments: [
          { name: 'a.pdf', data: pdf },
          { name: 'b.png', data: png }
        ],
        attachmentsInput: { name: 'files', multiple: true }
      })
    ).rejects.toThrow('quota exceeded')
    // The upload still in flight when the other failed was told to stop.
    expect(uploads(calls)[0].signal?.aborted).toBe(true)
    expect(calls.filter((c) => c.method === 'DELETE')).toEqual([])
    expect(runs(calls)).toHaveLength(0)
  })

  test('a send made right after stop() is not reset by the stopped upload', async () => {
    let releaseRun: (r: Response) => void = () => {}
    const { fetch, calls } = fetchMock(
      (c) =>
        c.url.pathname === UPLOAD_PATH
          ? new Promise((_, reject) =>
              c.signal!.addEventListener('abort', () => reject(abortError()))
            )
          : undefined,
      (c) =>
        c.method === 'POST' && c.url.pathname === `/api/w/ws/jobs/run/f/${FLOW}`
          ? new Promise<Response>((resolve) => (releaseRun = resolve))
          : undefined,
      answer
    )
    const chat = createChat(options(fetch))
    // An existing conversation, so the stopped turn and the next one share it.
    const first = chat.sendMessage('first')
    await new Promise((r) => setTimeout(r, 0))
    releaseRun(text('job-1'))
    await first
    const stopped = chat.sendMessage('with a file', {
      attachments: [{ name: 'a.pdf', data: pdf }],
      attachmentsInput: { name: 'files', multiple: true }
    })
    await new Promise((r) => setTimeout(r, 0))
    void chat.stop()
    const next = chat.sendMessage('right after')
    await expect(stopped).rejects.toMatchObject({ name: 'AbortError' })
    await new Promise((r) => setTimeout(r, 0))
    expect(chat.getState().status).toBe('submitted')
    expect(chat.getState().messages.map((m) => m.content)).toContain('right after')
    expect(chat.getState().messages.map((m) => m.content)).not.toContain('with a file')
    releaseRun(text('job-1'))
    await next
    expect(runs(calls)).toHaveLength(2)
  })

  test('a conversation is listed only once its run starts', async () => {
    let failUpload: (r: Response) => void = () => {}
    const { fetch } = fetchMock(
      (c) =>
        c.url.pathname === UPLOAD_PATH
          ? new Promise<Response>((resolve) => (failUpload = resolve))
          : undefined,
      run,
      answer
    )
    const chat = createChat(options(fetch))
    const sending = chat.sendMessage('read this', {
      attachments: [{ name: 'a.pdf', data: pdf }],
      attachmentsInput: { name: 'files', multiple: true }
    })
    await new Promise((r) => setTimeout(r, 0))
    expect(chat.getState()).toMatchObject({ status: 'submitted', conversations: [] })
    failUpload(text('boom', 500))
    await expect(sending).rejects.toThrow('boom')
    expect(chat.getState().conversations).toEqual([])
  })

  test('an already aborted signal uploads nothing', async () => {
    const { fetch, calls } = fetchMock(upload)
    const api = new WindmillChatApi({ baseUrl: BASE, workspace: 'ws', token: 'tok', fetch })
    const controller = new AbortController()
    controller.abort()
    await expect(
      uploadAttachments(api, [{ name: 'a.pdf', data: pdf }], 'turn', controller.signal)
    ).rejects.toMatchObject({ name: 'AbortError' })
    expect(calls).toHaveLength(0)
  })

  test('the pending user message carries its uploaded files before the run returns', async () => {
    let releaseRun: (r: Response) => void = () => {}
    const { fetch, calls } = fetchMock(
      upload,
      (c) =>
        c.method === 'POST' && c.url.pathname === `/api/w/ws/jobs/run/f/${FLOW}`
          ? new Promise<Response>((resolve) => (releaseRun = resolve))
          : undefined,
      answer
    )
    const chat = createChat(options(fetch))
    const sending = chat.sendMessage('read these', {
      attachments: [
        { name: 'photo.webp', data: png },
        { name: 'contract', data: pdf }
      ],
      attachmentsInput: { name: 'files', multiple: true }
    })
    while (runs(calls).length === 0) await new Promise((r) => setTimeout(r, 1))
    const keys = uploads(calls).map((c) => c.url.searchParams.get('file_key')!)
    const pending = chat.getState().messages.find((m) => m.role === 'user')!
    expect(pending.pending).toBe(true)
    expect(pending.attachments).toEqual([
      { input: 'files', s3: keys[0], filename: 'photo.png' },
      { input: 'files', s3: keys[1], filename: 'contract.pdf' }
    ])
    releaseRun(text('job-1'))
    await sending
  })

  test('an explicit mediaType wins over the type a data URL declares', async () => {
    const { fetch, calls } = fetchMock(upload, run, answer)
    const chat = createChat(options(fetch))
    await chat.sendMessage('read this', {
      attachments: [
        {
          name: 'contract',
          data: `data:application/octet-stream;base64,${btoa('%PDF')}`,
          mediaType: 'application/pdf'
        }
      ],
      attachmentsInput: { name: 'files', multiple: true }
    })
    const call = uploads(calls)[0]
    expect(call.url.searchParams.get('file_key')).toMatch(/\/0\/contract\.pdf$/)
    expect(call.url.searchParams.get('content_type')).toBe('application/pdf')
  })

  test('stop() after the uploads land but before the run starts runs nothing', async () => {
    const { fetch, calls } = fetchMock(upload, run, answer)
    const chat = createChat(options(fetch))
    const sending = chat.sendMessage('read this', {
      attachments: [{ name: 'contract.pdf', data: pdf }],
      attachmentsInput: { name: 'files', multiple: true }
    })
    // The upload responds at once; Stop lands before the send resumes after it.
    while (uploads(calls).length === 0) await Promise.resolve()
    await chat.stop()
    await expect(sending).rejects.toMatchObject({ name: 'AbortError' })
    expect(runs(calls)).toHaveLength(0)
    expect(calls.filter((c) => c.method === 'DELETE')).toEqual([])
    expect(chat.getState()).toMatchObject({ status: 'idle', messages: [], conversations: [] })
  })

  test('a subscriber stopping when the attachments appear runs nothing', async () => {
    const { fetch, calls } = fetchMock(upload, run, answer)
    const chat = createChat(options(fetch))
    chat.subscribe((s) => {
      if (s.messages.some((m) => m.attachments)) void chat.stop()
    })
    await expect(
      chat.sendMessage('read this', {
        attachments: [{ name: 'contract.pdf', data: pdf }],
        attachmentsInput: { name: 'files', multiple: true }
      })
    ).rejects.toMatchObject({ name: 'AbortError' })
    expect(runs(calls)).toHaveLength(0)
    expect(calls.filter((c) => c.method === 'DELETE')).toEqual([])
    expect(chat.getState()).toMatchObject({ status: 'idle', messages: [], conversations: [] })
  })
})
