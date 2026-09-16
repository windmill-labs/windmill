export { createChat } from './chat'
export { detectRawApp, type RawAppContext } from './config'
export {
  WindmillChatApi,
  WindmillApiError,
  readServerSentEvents,
  type WindmillChatApiOptions,
  type FlowConversation,
  type FlowConversationMessage,
  type JobUpdateEvent,
  type CompletedJobResult
} from './api'
export { parseStreamEvents, createStreamEventParser, type AgentStreamEvent } from './stream'
export { followJob, type FollowEvent } from './follow'
export { extractChatAnswer, conversationIdFor } from './utils'
export { storedAttachmentName, uploadAttachments, CHAT_UPLOADS_PREFIX, type UploadedAttachment } from './attachments'
export type {
  AttachmentsInput,
  Chat,
  ChatAttachment,
  ChatMessage,
  ChatOptions,
  ChatRole,
  ChatState,
  ChatStatus,
  Conversation,
  FetchLike,
  HistoryMode,
  SendMessageOptions,
  StorageLike,
  TokenSource,
  ToolInvocation
} from './types'
