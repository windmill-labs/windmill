import type { CancelablePromise } from './core/CancelablePromise';
import type { BackendVersionResponse, BackendUptodateResponse, GetLicenseIdResponse, GetOpenApiYamlResponse, GetAuditLogData, GetAuditLogResponse, ListAuditLogsData, ListAuditLogsResponse, LoginData, LoginResponse, LogoutResponse, GetUserData, GetUserResponse, UpdateUserData, UpdateUserResponse, IsOwnerOfPathData, IsOwnerOfPathResponse, SetPasswordData, SetPasswordResponse, CreateUserGloballyData, CreateUserGloballyResponse, GlobalUserUpdateData, GlobalUserUpdateResponse, GlobalUsernameInfoData, GlobalUsernameInfoResponse, GlobalUserRenameData, GlobalUserRenameResponse, GlobalUserDeleteData, GlobalUserDeleteResponse, DeleteUserData, DeleteUserResponse, GetCurrentEmailResponse, RefreshUserTokenResponse, GetTutorialProgressResponse, UpdateTutorialProgressData, UpdateTutorialProgressResponse, LeaveInstanceResponse, GetUsageResponse, GetRunnableResponse, GlobalWhoamiResponse, ListWorkspaceInvitesResponse, WhoamiData, WhoamiResponse, AcceptInviteData, AcceptInviteResponse, DeclineInviteData, DeclineInviteResponse, WhoisData, WhoisResponse, ExistsEmailData, ExistsEmailResponse, ListUsersAsSuperAdminData, ListUsersAsSuperAdminResponse, ListUsersData, ListUsersResponse, ListUsersUsageData, ListUsersUsageResponse, ListUsernamesData, ListUsernamesResponse, CreateTokenData, CreateTokenResponse, CreateTokenImpersonateData, CreateTokenImpersonateResponse, DeleteTokenData, DeleteTokenResponse, ListTokensData, ListTokensResponse, LoginWithOauthData, LoginWithOauthResponse, ListWorkspacesResponse, IsDomainAllowedResponse, ListUserWorkspacesResponse, ListWorkspacesAsSuperAdminData, ListWorkspacesAsSuperAdminResponse, CreateWorkspaceData, CreateWorkspaceResponse, ExistsWorkspaceData, ExistsWorkspaceResponse, ExistsUsernameData, ExistsUsernameResponse, InviteUserData, InviteUserResponse, AddUserData, AddUserResponse, DeleteInviteData, DeleteInviteResponse, ArchiveWorkspaceData, ArchiveWorkspaceResponse, UnarchiveWorkspaceData, UnarchiveWorkspaceResponse, DeleteWorkspaceData, DeleteWorkspaceResponse, LeaveWorkspaceData, LeaveWorkspaceResponse, GetWorkspaceNameData, GetWorkspaceNameResponse, ChangeWorkspaceNameData, ChangeWorkspaceNameResponse, ChangeWorkspaceIdData, ChangeWorkspaceIdResponse, ListPendingInvitesData, ListPendingInvitesResponse, GetSettingsData, GetSettingsResponse, GetDeployToData, GetDeployToResponse, GetIsPremiumData, GetIsPremiumResponse, GetPremiumInfoData, GetPremiumInfoResponse, SetAutomaticBillingData, SetAutomaticBillingResponse, EditSlackCommandData, EditSlackCommandResponse, RunSlackMessageTestJobData, RunSlackMessageTestJobResponse, EditDeployToData, EditDeployToResponse, EditAutoInviteData, EditAutoInviteResponse, EditWebhookData, EditWebhookResponse, EditCopilotConfigData, EditCopilotConfigResponse, GetCopilotInfoData, GetCopilotInfoResponse, EditErrorHandlerData, EditErrorHandlerResponse, EditLargeFileStorageConfigData, EditLargeFileStorageConfigResponse, EditWorkspaceGitSyncConfigData, EditWorkspaceGitSyncConfigResponse, EditWorkspaceDefaultAppData, EditWorkspaceDefaultAppResponse, EditDefaultScriptsData, EditDefaultScriptsResponse, GetDefaultScriptsData, GetDefaultScriptsResponse, SetEnvironmentVariableData, SetEnvironmentVariableResponse, GetWorkspaceEncryptionKeyData, GetWorkspaceEncryptionKeyResponse, SetWorkspaceEncryptionKeyData, SetWorkspaceEncryptionKeyResponse, GetWorkspaceDefaultAppData, GetWorkspaceDefaultAppResponse, GetLargeFileStorageConfigData, GetLargeFileStorageConfigResponse, GetWorkspaceUsageData, GetWorkspaceUsageResponse, GetGlobalData, GetGlobalResponse, SetGlobalData, SetGlobalResponse, GetLocalResponse, TestSmtpData, TestSmtpResponse, TestLicenseKeyData, TestLicenseKeyResponse, TestObjectStorageConfigData, TestObjectStorageConfigResponse, SendStatsResponse, TestMetadataData, TestMetadataResponse, GetOidcTokenData, GetOidcTokenResponse, CreateVariableData, CreateVariableResponse, EncryptValueData, EncryptValueResponse, DeleteVariableData, DeleteVariableResponse, UpdateVariableData, UpdateVariableResponse, GetVariableData, GetVariableResponse, GetVariableValueData, GetVariableValueResponse, ExistsVariableData, ExistsVariableResponse, ListVariableData, ListVariableResponse, ListContextualVariablesData, ListContextualVariablesResponse, ConnectSlackCallbackData, ConnectSlackCallbackResponse, ConnectCallbackData, ConnectCallbackResponse, CreateAccountData, CreateAccountResponse, RefreshTokenData, RefreshTokenResponse, DisconnectAccountData, DisconnectAccountResponse, DisconnectSlackData, DisconnectSlackResponse, ListOauthLoginsResponse, ListOauthConnectsResponse, CreateResourceData, CreateResourceResponse, DeleteResourceData, DeleteResourceResponse, UpdateResourceData, UpdateResourceResponse, UpdateResourceValueData, UpdateResourceValueResponse, GetResourceData, GetResourceResponse, GetResourceValueInterpolatedData, GetResourceValueInterpolatedResponse, GetResourceValueData, GetResourceValueResponse, ExistsResourceData, ExistsResourceResponse, ListResourceData, ListResourceResponse, ListSearchResourceData, ListSearchResourceResponse, ListResourceNamesData, ListResourceNamesResponse, CreateResourceTypeData, CreateResourceTypeResponse, DeleteResourceTypeData, DeleteResourceTypeResponse, UpdateResourceTypeData, UpdateResourceTypeResponse, GetResourceTypeData, GetResourceTypeResponse, ExistsResourceTypeData, ExistsResourceTypeResponse, ListResourceTypeData, ListResourceTypeResponse, ListResourceTypeNamesData, ListResourceTypeNamesResponse, QueryResourceTypesData, QueryResourceTypesResponse, ListHubIntegrationsData, ListHubIntegrationsResponse, ListHubFlowsResponse, GetHubFlowByIdData, GetHubFlowByIdResponse, ListFlowPathsData, ListFlowPathsResponse, ListSearchFlowData, ListSearchFlowResponse, ListFlowsData, ListFlowsResponse, GetFlowByPathData, GetFlowByPathResponse, ToggleWorkspaceErrorHandlerForFlowData, ToggleWorkspaceErrorHandlerForFlowResponse, GetFlowByPathWithDraftData, GetFlowByPathWithDraftResponse, ExistsFlowByPathData, ExistsFlowByPathResponse, CreateFlowData, CreateFlowResponse, UpdateFlowData, UpdateFlowResponse, ArchiveFlowByPathData, ArchiveFlowByPathResponse, DeleteFlowByPathData, DeleteFlowByPathResponse, GetFlowInputHistoryByPathData, GetFlowInputHistoryByPathResponse, ListHubAppsResponse, GetHubAppByIdData, GetHubAppByIdResponse, ListSearchAppData, ListSearchAppResponse, ListAppsData, ListAppsResponse, CreateAppData, CreateAppResponse, ExistsAppData, ExistsAppResponse, GetAppByPathData, GetAppByPathResponse, GetAppByPathWithDraftData, GetAppByPathWithDraftResponse, GetAppHistoryByPathData, GetAppHistoryByPathResponse, UpdateAppHistoryData, UpdateAppHistoryResponse, GetPublicAppBySecretData, GetPublicAppBySecretResponse, GetPublicResourceData, GetPublicResourceResponse, GetPublicSecretOfAppData, GetPublicSecretOfAppResponse, GetAppByVersionData, GetAppByVersionResponse, DeleteAppData, DeleteAppResponse, UpdateAppData, UpdateAppResponse, ExecuteComponentData, ExecuteComponentResponse, GetHubScriptContentByPathData, GetHubScriptContentByPathResponse, GetHubScriptByPathData, GetHubScriptByPathResponse, GetTopHubScriptsData, GetTopHubScriptsResponse, QueryHubScriptsData, QueryHubScriptsResponse, ListSearchScriptData, ListSearchScriptResponse, ListScriptsData, ListScriptsResponse, ListScriptPathsData, ListScriptPathsResponse, CreateScriptData, CreateScriptResponse, ToggleWorkspaceErrorHandlerForScriptData, ToggleWorkspaceErrorHandlerForScriptResponse, ArchiveScriptByPathData, ArchiveScriptByPathResponse, ArchiveScriptByHashData, ArchiveScriptByHashResponse, DeleteScriptByHashData, DeleteScriptByHashResponse, DeleteScriptByPathData, DeleteScriptByPathResponse, GetScriptByPathData, GetScriptByPathResponse, GetScriptByPathWithDraftData, GetScriptByPathWithDraftResponse, GetScriptHistoryByPathData, GetScriptHistoryByPathResponse, UpdateScriptHistoryData, UpdateScriptHistoryResponse, RawScriptByPathData, RawScriptByPathResponse, RawScriptByPathTokenedData, RawScriptByPathTokenedResponse, ExistsScriptByPathData, ExistsScriptByPathResponse, GetScriptByHashData, GetScriptByHashResponse, RawScriptByHashData, RawScriptByHashResponse, GetScriptDeploymentStatusData, GetScriptDeploymentStatusResponse, CreateDraftData, CreateDraftResponse, DeleteDraftData, DeleteDraftResponse, GetCustomTagsResponse, GeDefaultTagsResponse, IsDefaultTagsPerWorkspaceResponse, ListWorkersData, ListWorkersResponse, ExistsWorkerWithTagData, ExistsWorkerWithTagResponse, GetQueueMetricsResponse, RunScriptByPathData, RunScriptByPathResponse, OpenaiSyncScriptByPathData, OpenaiSyncScriptByPathResponse, RunWaitResultScriptByPathData, RunWaitResultScriptByPathResponse, RunWaitResultScriptByPathGetData, RunWaitResultScriptByPathGetResponse, OpenaiSyncFlowByPathData, OpenaiSyncFlowByPathResponse, RunWaitResultFlowByPathData, RunWaitResultFlowByPathResponse, ResultByIdData, ResultByIdResponse, RunFlowByPathData, RunFlowByPathResponse, RestartFlowAtStepData, RestartFlowAtStepResponse, RunScriptByHashData, RunScriptByHashResponse, RunScriptPreviewData, RunScriptPreviewResponse, RunCodeWorkflowTaskData, RunCodeWorkflowTaskResponse, RunRawScriptDependenciesData, RunRawScriptDependenciesResponse, RunFlowPreviewData, RunFlowPreviewResponse, ListQueueData, ListQueueResponse, GetQueueCountData, GetQueueCountResponse, GetCompletedCountData, GetCompletedCountResponse, CancelAllData, CancelAllResponse, ListCompletedJobsData, ListCompletedJobsResponse, ListJobsData, ListJobsResponse, GetDbClockResponse, GetJobData, GetJobResponse, GetRootJobIdData, GetRootJobIdResponse, GetJobLogsData, GetJobLogsResponse, GetJobUpdatesData, GetJobUpdatesResponse, GetLogFileFromStoreData, GetLogFileFromStoreResponse, GetFlowDebugInfoData, GetFlowDebugInfoResponse, GetCompletedJobData, GetCompletedJobResponse, GetCompletedJobResultData, GetCompletedJobResultResponse, GetCompletedJobResultMaybeData, GetCompletedJobResultMaybeResponse, DeleteCompletedJobData, DeleteCompletedJobResponse, CancelQueuedJobData, CancelQueuedJobResponse, CancelPersistentQueuedJobsData, CancelPersistentQueuedJobsResponse, ForceCancelQueuedJobData, ForceCancelQueuedJobResponse, CreateJobSignatureData, CreateJobSignatureResponse, GetResumeUrlsData, GetResumeUrlsResponse, ResumeSuspendedJobGetData, ResumeSuspendedJobGetResponse, ResumeSuspendedJobPostData, ResumeSuspendedJobPostResponse, SetFlowUserStateData, SetFlowUserStateResponse, GetFlowUserStateData, GetFlowUserStateResponse, ResumeSuspendedFlowAsOwnerData, ResumeSuspendedFlowAsOwnerResponse, CancelSuspendedJobGetData, CancelSuspendedJobGetResponse, CancelSuspendedJobPostData, CancelSuspendedJobPostResponse, GetSuspendedJobFlowData, GetSuspendedJobFlowResponse, ListRawAppsData, ListRawAppsResponse, ExistsRawAppData, ExistsRawAppResponse, GetRawAppDataData, GetRawAppDataResponse, CreateRawAppData, CreateRawAppResponse, UpdateRawAppData, UpdateRawAppResponse, DeleteRawAppData, DeleteRawAppResponse, PreviewScheduleData, PreviewScheduleResponse, CreateScheduleData, CreateScheduleResponse, UpdateScheduleData, UpdateScheduleResponse, SetScheduleEnabledData, SetScheduleEnabledResponse, DeleteScheduleData, DeleteScheduleResponse, GetScheduleData, GetScheduleResponse, ExistsScheduleData, ExistsScheduleResponse, ListSchedulesData, ListSchedulesResponse, ListSchedulesWithJobsData, ListSchedulesWithJobsResponse, SetDefaultErrorOrRecoveryHandlerData, SetDefaultErrorOrRecoveryHandlerResponse, ListInstanceGroupsResponse, GetInstanceGroupData, GetInstanceGroupResponse, CreateInstanceGroupData, CreateInstanceGroupResponse, UpdateInstanceGroupData, UpdateInstanceGroupResponse, DeleteInstanceGroupData, DeleteInstanceGroupResponse, AddUserToInstanceGroupData, AddUserToInstanceGroupResponse, RemoveUserFromInstanceGroupData, RemoveUserFromInstanceGroupResponse, ListGroupsData, ListGroupsResponse, ListGroupNamesData, ListGroupNamesResponse, CreateGroupData, CreateGroupResponse, UpdateGroupData, UpdateGroupResponse, DeleteGroupData, DeleteGroupResponse, GetGroupData, GetGroupResponse, AddUserToGroupData, AddUserToGroupResponse, RemoveUserToGroupData, RemoveUserToGroupResponse, ListFoldersData, ListFoldersResponse, ListFolderNamesData, ListFolderNamesResponse, CreateFolderData, CreateFolderResponse, UpdateFolderData, UpdateFolderResponse, DeleteFolderData, DeleteFolderResponse, GetFolderData, GetFolderResponse, GetFolderUsageData, GetFolderUsageResponse, AddOwnerToFolderData, AddOwnerToFolderResponse, RemoveOwnerToFolderData, RemoveOwnerToFolderResponse, ListWorkerGroupsResponse, GetConfigData, GetConfigResponse, UpdateConfigData, UpdateConfigResponse, DeleteConfigData, DeleteConfigResponse, GetGranularAclsData, GetGranularAclsResponse, AddGranularAclsData, AddGranularAclsResponse, RemoveGranularAclsData, RemoveGranularAclsResponse, UpdateCaptureData, UpdateCaptureResponse, CreateCaptureData, CreateCaptureResponse, GetCaptureData, GetCaptureResponse, StarData, StarResponse, UnstarData, UnstarResponse, GetInputHistoryData, GetInputHistoryResponse, GetArgsFromHistoryOrSavedInputData, GetArgsFromHistoryOrSavedInputResponse, ListInputsData, ListInputsResponse, CreateInputData, CreateInputResponse, UpdateInputData, UpdateInputResponse, DeleteInputData, DeleteInputResponse, DuckdbConnectionSettingsData, DuckdbConnectionSettingsResponse, DuckdbConnectionSettingsV2Data, DuckdbConnectionSettingsV2Response, PolarsConnectionSettingsData, PolarsConnectionSettingsResponse, PolarsConnectionSettingsV2Data, PolarsConnectionSettingsV2Response, S3ResourceInfoData, S3ResourceInfoResponse, DatasetStorageTestConnectionData, DatasetStorageTestConnectionResponse, ListStoredFilesData, ListStoredFilesResponse, LoadFileMetadataData, LoadFileMetadataResponse, LoadFilePreviewData, LoadFilePreviewResponse, LoadParquetPreviewData, LoadParquetPreviewResponse, DeleteS3FileData, DeleteS3FileResponse, MoveS3FileData, MoveS3FileResponse, FileUploadData, FileUploadResponse, FileDownloadData, FileDownloadResponse, GetJobMetricsData, GetJobMetricsResponse, ListConcurrencyGroupsResponse, DeleteConcurrencyGroupData, DeleteConcurrencyGroupResponse } from './types.gen';
export declare class SettingsService {
    /**
     * get backend version
     * @returns string git version of backend
     * @throws ApiError
     */
    static backendVersion(): CancelablePromise<BackendVersionResponse>;
    /**
     * is backend up to date
     * @returns string is backend up to date
     * @throws ApiError
     */
    static backendUptodate(): CancelablePromise<BackendUptodateResponse>;
    /**
     * get license id
     * @returns string get license id (empty if not ee)
     * @throws ApiError
     */
    static getLicenseId(): CancelablePromise<GetLicenseIdResponse>;
    /**
     * get openapi yaml spec
     * @returns string openapi yaml file content
     * @throws ApiError
     */
    static getOpenApiYaml(): CancelablePromise<GetOpenApiYamlResponse>;
}
export declare class AuditService {
    /**
     * get audit log (requires admin privilege)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @returns AuditLog an audit log
     * @throws ApiError
     */
    static getAuditLog(data: GetAuditLogData): CancelablePromise<GetAuditLogResponse>;
    /**
     * list audit logs (requires admin privilege)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @param data.before filter on created before (exclusive) timestamp
     * @param data.after filter on created after (exclusive) timestamp
     * @param data.username filter on exact username of user
     * @param data.operation filter on exact or prefix name of operation
     * @param data.resource filter on exact or prefix name of resource
     * @param data.actionKind filter on type of operation
     * @returns AuditLog a list of audit logs
     * @throws ApiError
     */
    static listAuditLogs(data: ListAuditLogsData): CancelablePromise<ListAuditLogsResponse>;
}
export declare class UserService {
    /**
     * login with password
     * @param data The data for the request.
     * @param data.requestBody credentials
     * @returns string Successfully authenticated. The session ID is returned in a cookie named `token` and as plaintext response. Preferred method of authorization is through the bearer token. The cookie is only for browser convenience.
     *
     * @throws ApiError
     */
    static login(data: LoginData): CancelablePromise<LoginResponse>;
    /**
     * logout
     * @returns string clear cookies and clear token (if applicable)
     * @throws ApiError
     */
    static logout(): CancelablePromise<LogoutResponse>;
    /**
     * get user (require admin privilege)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.username
     * @returns User user created
     * @throws ApiError
     */
    static getUser(data: GetUserData): CancelablePromise<GetUserResponse>;
    /**
     * update user (require admin privilege)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.username
     * @param data.requestBody new user
     * @returns string edited user
     * @throws ApiError
     */
    static updateUser(data: UpdateUserData): CancelablePromise<UpdateUserResponse>;
    /**
     * is owner of path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns boolean is owner
     * @throws ApiError
     */
    static isOwnerOfPath(data: IsOwnerOfPathData): CancelablePromise<IsOwnerOfPathResponse>;
    /**
     * set password
     * @param data The data for the request.
     * @param data.requestBody set password
     * @returns string password set
     * @throws ApiError
     */
    static setPassword(data: SetPasswordData): CancelablePromise<SetPasswordResponse>;
    /**
     * create user
     * @param data The data for the request.
     * @param data.requestBody user info
     * @returns string user created
     * @throws ApiError
     */
    static createUserGlobally(data: CreateUserGloballyData): CancelablePromise<CreateUserGloballyResponse>;
    /**
     * global update user (require super admin)
     * @param data The data for the request.
     * @param data.email
     * @param data.requestBody new user info
     * @returns string user updated
     * @throws ApiError
     */
    static globalUserUpdate(data: GlobalUserUpdateData): CancelablePromise<GlobalUserUpdateResponse>;
    /**
     * global username info (require super admin)
     * @param data The data for the request.
     * @param data.email
     * @returns unknown user renamed
     * @throws ApiError
     */
    static globalUsernameInfo(data: GlobalUsernameInfoData): CancelablePromise<GlobalUsernameInfoResponse>;
    /**
     * global rename user (require super admin)
     * @param data The data for the request.
     * @param data.email
     * @param data.requestBody new username
     * @returns string user renamed
     * @throws ApiError
     */
    static globalUserRename(data: GlobalUserRenameData): CancelablePromise<GlobalUserRenameResponse>;
    /**
     * global delete user (require super admin)
     * @param data The data for the request.
     * @param data.email
     * @returns string user deleted
     * @throws ApiError
     */
    static globalUserDelete(data: GlobalUserDeleteData): CancelablePromise<GlobalUserDeleteResponse>;
    /**
     * delete user (require admin privilege)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.username
     * @returns string delete user
     * @throws ApiError
     */
    static deleteUser(data: DeleteUserData): CancelablePromise<DeleteUserResponse>;
    /**
     * get current user email (if logged in)
     * @returns string user email
     * @throws ApiError
     */
    static getCurrentEmail(): CancelablePromise<GetCurrentEmailResponse>;
    /**
     * refresh the current token
     * @returns string free usage
     * @throws ApiError
     */
    static refreshUserToken(): CancelablePromise<RefreshUserTokenResponse>;
    /**
     * get tutorial progress
     * @returns unknown tutorial progress
     * @throws ApiError
     */
    static getTutorialProgress(): CancelablePromise<GetTutorialProgressResponse>;
    /**
     * update tutorial progress
     * @param data The data for the request.
     * @param data.requestBody progress update
     * @returns string tutorial progress
     * @throws ApiError
     */
    static updateTutorialProgress(data: UpdateTutorialProgressData): CancelablePromise<UpdateTutorialProgressResponse>;
    /**
     * leave instance
     * @returns string status
     * @throws ApiError
     */
    static leaveInstance(): CancelablePromise<LeaveInstanceResponse>;
    /**
     * get current usage outside of premium workspaces
     * @returns number free usage
     * @throws ApiError
     */
    static getUsage(): CancelablePromise<GetUsageResponse>;
    /**
     * get all runnables in every workspace
     * @returns unknown free all runnables
     * @throws ApiError
     */
    static getRunnable(): CancelablePromise<GetRunnableResponse>;
    /**
     * get current global whoami (if logged in)
     * @returns GlobalUserInfo user email
     * @throws ApiError
     */
    static globalWhoami(): CancelablePromise<GlobalWhoamiResponse>;
    /**
     * list all workspace invites
     * @returns WorkspaceInvite list all workspace invites
     * @throws ApiError
     */
    static listWorkspaceInvites(): CancelablePromise<ListWorkspaceInvitesResponse>;
    /**
     * whoami
     * @param data The data for the request.
     * @param data.workspace
     * @returns User user
     * @throws ApiError
     */
    static whoami(data: WhoamiData): CancelablePromise<WhoamiResponse>;
    /**
     * accept invite to workspace
     * @param data The data for the request.
     * @param data.requestBody accept invite
     * @returns string status
     * @throws ApiError
     */
    static acceptInvite(data: AcceptInviteData): CancelablePromise<AcceptInviteResponse>;
    /**
     * decline invite to workspace
     * @param data The data for the request.
     * @param data.requestBody decline invite
     * @returns string status
     * @throws ApiError
     */
    static declineInvite(data: DeclineInviteData): CancelablePromise<DeclineInviteResponse>;
    /**
     * whois
     * @param data The data for the request.
     * @param data.workspace
     * @param data.username
     * @returns User user
     * @throws ApiError
     */
    static whois(data: WhoisData): CancelablePromise<WhoisResponse>;
    /**
     * exists email
     * @param data The data for the request.
     * @param data.email
     * @returns boolean user
     * @throws ApiError
     */
    static existsEmail(data: ExistsEmailData): CancelablePromise<ExistsEmailResponse>;
    /**
     * list all users as super admin (require to be super amdin)
     * @param data The data for the request.
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @returns GlobalUserInfo user
     * @throws ApiError
     */
    static listUsersAsSuperAdmin(data?: ListUsersAsSuperAdminData): CancelablePromise<ListUsersAsSuperAdminResponse>;
    /**
     * list users
     * @param data The data for the request.
     * @param data.workspace
     * @returns User user
     * @throws ApiError
     */
    static listUsers(data: ListUsersData): CancelablePromise<ListUsersResponse>;
    /**
     * list users usage
     * @param data The data for the request.
     * @param data.workspace
     * @returns UserUsage user
     * @throws ApiError
     */
    static listUsersUsage(data: ListUsersUsageData): CancelablePromise<ListUsersUsageResponse>;
    /**
     * list usernames
     * @param data The data for the request.
     * @param data.workspace
     * @returns string user
     * @throws ApiError
     */
    static listUsernames(data: ListUsernamesData): CancelablePromise<ListUsernamesResponse>;
    /**
     * create token
     * @param data The data for the request.
     * @param data.requestBody new token
     * @returns string token created
     * @throws ApiError
     */
    static createToken(data: CreateTokenData): CancelablePromise<CreateTokenResponse>;
    /**
     * create token to impersonate a user (require superadmin)
     * @param data The data for the request.
     * @param data.requestBody new token
     * @returns string token created
     * @throws ApiError
     */
    static createTokenImpersonate(data: CreateTokenImpersonateData): CancelablePromise<CreateTokenImpersonateResponse>;
    /**
     * delete token
     * @param data The data for the request.
     * @param data.tokenPrefix
     * @returns string delete token
     * @throws ApiError
     */
    static deleteToken(data: DeleteTokenData): CancelablePromise<DeleteTokenResponse>;
    /**
     * list token
     * @param data The data for the request.
     * @param data.excludeEphemeral
     * @returns TruncatedToken truncated token
     * @throws ApiError
     */
    static listTokens(data?: ListTokensData): CancelablePromise<ListTokensResponse>;
    /**
     * login with oauth authorization flow
     * @param data The data for the request.
     * @param data.clientName
     * @param data.requestBody Partially filled script
     * @returns string Successfully authenticated. The session ID is returned in a cookie named `token` and as plaintext response. Preferred method of authorization is through the bearer token. The cookie is only for browser convenience.
     *
     * @throws ApiError
     */
    static loginWithOauth(data: LoginWithOauthData): CancelablePromise<LoginWithOauthResponse>;
}
export declare class AdminService {
    /**
     * get user (require admin privilege)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.username
     * @returns User user created
     * @throws ApiError
     */
    static getUser(data: GetUserData): CancelablePromise<GetUserResponse>;
    /**
     * update user (require admin privilege)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.username
     * @param data.requestBody new user
     * @returns string edited user
     * @throws ApiError
     */
    static updateUser(data: UpdateUserData): CancelablePromise<UpdateUserResponse>;
    /**
     * delete user (require admin privilege)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.username
     * @returns string delete user
     * @throws ApiError
     */
    static deleteUser(data: DeleteUserData): CancelablePromise<DeleteUserResponse>;
}
export declare class WorkspaceService {
    /**
     * list all workspaces visible to me
     * @returns Workspace all workspaces
     * @throws ApiError
     */
    static listWorkspaces(): CancelablePromise<ListWorkspacesResponse>;
    /**
     * is domain allowed for auto invi
     * @returns boolean domain allowed or not
     * @throws ApiError
     */
    static isDomainAllowed(): CancelablePromise<IsDomainAllowedResponse>;
    /**
     * list all workspaces visible to me with user info
     * @returns UserWorkspaceList workspace with associated username
     * @throws ApiError
     */
    static listUserWorkspaces(): CancelablePromise<ListUserWorkspacesResponse>;
    /**
     * list all workspaces as super admin (require to be super admin)
     * @param data The data for the request.
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @returns Workspace workspaces
     * @throws ApiError
     */
    static listWorkspacesAsSuperAdmin(data?: ListWorkspacesAsSuperAdminData): CancelablePromise<ListWorkspacesAsSuperAdminResponse>;
    /**
     * create workspace
     * @param data The data for the request.
     * @param data.requestBody new token
     * @returns string token created
     * @throws ApiError
     */
    static createWorkspace(data: CreateWorkspaceData): CancelablePromise<CreateWorkspaceResponse>;
    /**
     * exists workspace
     * @param data The data for the request.
     * @param data.requestBody id of workspace
     * @returns boolean status
     * @throws ApiError
     */
    static existsWorkspace(data: ExistsWorkspaceData): CancelablePromise<ExistsWorkspaceResponse>;
    /**
     * exists username
     * @param data The data for the request.
     * @param data.requestBody
     * @returns boolean status
     * @throws ApiError
     */
    static existsUsername(data: ExistsUsernameData): CancelablePromise<ExistsUsernameResponse>;
    /**
     * invite user to workspace
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody WorkspaceInvite
     * @returns string status
     * @throws ApiError
     */
    static inviteUser(data: InviteUserData): CancelablePromise<InviteUserResponse>;
    /**
     * add user to workspace
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody WorkspaceInvite
     * @returns string status
     * @throws ApiError
     */
    static addUser(data: AddUserData): CancelablePromise<AddUserResponse>;
    /**
     * delete user invite
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody WorkspaceInvite
     * @returns string status
     * @throws ApiError
     */
    static deleteInvite(data: DeleteInviteData): CancelablePromise<DeleteInviteResponse>;
    /**
     * archive workspace
     * @param data The data for the request.
     * @param data.workspace
     * @returns string status
     * @throws ApiError
     */
    static archiveWorkspace(data: ArchiveWorkspaceData): CancelablePromise<ArchiveWorkspaceResponse>;
    /**
     * unarchive workspace
     * @param data The data for the request.
     * @param data.workspace
     * @returns string status
     * @throws ApiError
     */
    static unarchiveWorkspace(data: UnarchiveWorkspaceData): CancelablePromise<UnarchiveWorkspaceResponse>;
    /**
     * delete workspace (require super admin)
     * @param data The data for the request.
     * @param data.workspace
     * @returns string status
     * @throws ApiError
     */
    static deleteWorkspace(data: DeleteWorkspaceData): CancelablePromise<DeleteWorkspaceResponse>;
    /**
     * leave workspace
     * @param data The data for the request.
     * @param data.workspace
     * @returns string status
     * @throws ApiError
     */
    static leaveWorkspace(data: LeaveWorkspaceData): CancelablePromise<LeaveWorkspaceResponse>;
    /**
     * get workspace name
     * @param data The data for the request.
     * @param data.workspace
     * @returns string status
     * @throws ApiError
     */
    static getWorkspaceName(data: GetWorkspaceNameData): CancelablePromise<GetWorkspaceNameResponse>;
    /**
     * change workspace name
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody
     * @returns string status
     * @throws ApiError
     */
    static changeWorkspaceName(data: ChangeWorkspaceNameData): CancelablePromise<ChangeWorkspaceNameResponse>;
    /**
     * change workspace id
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody
     * @returns string status
     * @throws ApiError
     */
    static changeWorkspaceId(data: ChangeWorkspaceIdData): CancelablePromise<ChangeWorkspaceIdResponse>;
    /**
     * list pending invites for a workspace
     * @param data The data for the request.
     * @param data.workspace
     * @returns WorkspaceInvite user
     * @throws ApiError
     */
    static listPendingInvites(data: ListPendingInvitesData): CancelablePromise<ListPendingInvitesResponse>;
    /**
     * get settings
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown status
     * @throws ApiError
     */
    static getSettings(data: GetSettingsData): CancelablePromise<GetSettingsResponse>;
    /**
     * get deploy to
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown status
     * @throws ApiError
     */
    static getDeployTo(data: GetDeployToData): CancelablePromise<GetDeployToResponse>;
    /**
     * get if workspace is premium
     * @param data The data for the request.
     * @param data.workspace
     * @returns boolean status
     * @throws ApiError
     */
    static getIsPremium(data: GetIsPremiumData): CancelablePromise<GetIsPremiumResponse>;
    /**
     * get premium info
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown status
     * @throws ApiError
     */
    static getPremiumInfo(data: GetPremiumInfoData): CancelablePromise<GetPremiumInfoResponse>;
    /**
     * set automatic billing
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody automatic billing
     * @returns string status
     * @throws ApiError
     */
    static setAutomaticBilling(data: SetAutomaticBillingData): CancelablePromise<SetAutomaticBillingResponse>;
    /**
     * edit slack command
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody WorkspaceInvite
     * @returns string status
     * @throws ApiError
     */
    static editSlackCommand(data: EditSlackCommandData): CancelablePromise<EditSlackCommandResponse>;
    /**
     * run a job that sends a message to Slack
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody path to hub script to run and its corresponding args
     * @returns unknown status
     * @throws ApiError
     */
    static runSlackMessageTestJob(data: RunSlackMessageTestJobData): CancelablePromise<RunSlackMessageTestJobResponse>;
    /**
     * edit deploy to
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody
     * @returns string status
     * @throws ApiError
     */
    static editDeployTo(data: EditDeployToData): CancelablePromise<EditDeployToResponse>;
    /**
     * edit auto invite
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody WorkspaceInvite
     * @returns string status
     * @throws ApiError
     */
    static editAutoInvite(data: EditAutoInviteData): CancelablePromise<EditAutoInviteResponse>;
    /**
     * edit webhook
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody WorkspaceWebhook
     * @returns string status
     * @throws ApiError
     */
    static editWebhook(data: EditWebhookData): CancelablePromise<EditWebhookResponse>;
    /**
     * edit copilot config
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody WorkspaceCopilotConfig
     * @returns string status
     * @throws ApiError
     */
    static editCopilotConfig(data: EditCopilotConfigData): CancelablePromise<EditCopilotConfigResponse>;
    /**
     * get copilot info
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown status
     * @throws ApiError
     */
    static getCopilotInfo(data: GetCopilotInfoData): CancelablePromise<GetCopilotInfoResponse>;
    /**
     * edit error handler
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody WorkspaceErrorHandler
     * @returns string status
     * @throws ApiError
     */
    static editErrorHandler(data: EditErrorHandlerData): CancelablePromise<EditErrorHandlerResponse>;
    /**
     * edit large file storage settings
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody LargeFileStorage info
     * @returns unknown status
     * @throws ApiError
     */
    static editLargeFileStorageConfig(data: EditLargeFileStorageConfigData): CancelablePromise<EditLargeFileStorageConfigResponse>;
    /**
     * edit workspace git sync settings
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody Workspace Git sync settings
     * @returns unknown status
     * @throws ApiError
     */
    static editWorkspaceGitSyncConfig(data: EditWorkspaceGitSyncConfigData): CancelablePromise<EditWorkspaceGitSyncConfigResponse>;
    /**
     * edit default app for workspace
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody Workspace default app
     * @returns string status
     * @throws ApiError
     */
    static editWorkspaceDefaultApp(data: EditWorkspaceDefaultAppData): CancelablePromise<EditWorkspaceDefaultAppResponse>;
    /**
     * edit default scripts for workspace
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody Workspace default app
     * @returns string status
     * @throws ApiError
     */
    static editDefaultScripts(data: EditDefaultScriptsData): CancelablePromise<EditDefaultScriptsResponse>;
    /**
     * get default scripts for workspace
     * @param data The data for the request.
     * @param data.workspace
     * @returns WorkspaceDefaultScripts status
     * @throws ApiError
     */
    static getDefaultScripts(data: GetDefaultScriptsData): CancelablePromise<GetDefaultScriptsResponse>;
    /**
     * set environment variable
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody Workspace default app
     * @returns string status
     * @throws ApiError
     */
    static setEnvironmentVariable(data: SetEnvironmentVariableData): CancelablePromise<SetEnvironmentVariableResponse>;
    /**
     * retrieves the encryption key for this workspace
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown status
     * @throws ApiError
     */
    static getWorkspaceEncryptionKey(data: GetWorkspaceEncryptionKeyData): CancelablePromise<GetWorkspaceEncryptionKeyResponse>;
    /**
     * update the encryption key for this workspace
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody New encryption key
     * @returns string status
     * @throws ApiError
     */
    static setWorkspaceEncryptionKey(data: SetWorkspaceEncryptionKeyData): CancelablePromise<SetWorkspaceEncryptionKeyResponse>;
    /**
     * get default app for workspace
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown status
     * @throws ApiError
     */
    static getWorkspaceDefaultApp(data: GetWorkspaceDefaultAppData): CancelablePromise<GetWorkspaceDefaultAppResponse>;
    /**
     * get large file storage config
     * @param data The data for the request.
     * @param data.workspace
     * @returns LargeFileStorage status
     * @throws ApiError
     */
    static getLargeFileStorageConfig(data: GetLargeFileStorageConfigData): CancelablePromise<GetLargeFileStorageConfigResponse>;
    /**
     * get usage
     * @param data The data for the request.
     * @param data.workspace
     * @returns number usage
     * @throws ApiError
     */
    static getWorkspaceUsage(data: GetWorkspaceUsageData): CancelablePromise<GetWorkspaceUsageResponse>;
}
export declare class SettingService {
    /**
     * get global settings
     * @param data The data for the request.
     * @param data.key
     * @returns unknown status
     * @throws ApiError
     */
    static getGlobal(data: GetGlobalData): CancelablePromise<GetGlobalResponse>;
    /**
     * post global settings
     * @param data The data for the request.
     * @param data.key
     * @param data.requestBody value set
     * @returns string status
     * @throws ApiError
     */
    static setGlobal(data: SetGlobalData): CancelablePromise<SetGlobalResponse>;
    /**
     * get local settings
     * @returns unknown status
     * @throws ApiError
     */
    static getLocal(): CancelablePromise<GetLocalResponse>;
    /**
     * test smtp
     * @param data The data for the request.
     * @param data.requestBody test smtp payload
     * @returns string status
     * @throws ApiError
     */
    static testSmtp(data: TestSmtpData): CancelablePromise<TestSmtpResponse>;
    /**
     * test license key
     * @param data The data for the request.
     * @param data.requestBody test license key
     * @returns string status
     * @throws ApiError
     */
    static testLicenseKey(data: TestLicenseKeyData): CancelablePromise<TestLicenseKeyResponse>;
    /**
     * test object storage config
     * @param data The data for the request.
     * @param data.requestBody test object storage config
     * @returns string status
     * @throws ApiError
     */
    static testObjectStorageConfig(data: TestObjectStorageConfigData): CancelablePromise<TestObjectStorageConfigResponse>;
    /**
     * send stats
     * @returns string status
     * @throws ApiError
     */
    static sendStats(): CancelablePromise<SendStatsResponse>;
    /**
     * test metadata
     * @param data The data for the request.
     * @param data.requestBody test metadata
     * @returns string status
     * @throws ApiError
     */
    static testMetadata(data: TestMetadataData): CancelablePromise<TestMetadataResponse>;
}
export declare class OidcService {
    /**
     * get OIDC token (ee only)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.audience
     * @returns string new oidc token
     * @throws ApiError
     */
    static getOidcToken(data: GetOidcTokenData): CancelablePromise<GetOidcTokenResponse>;
}
export declare class VariableService {
    /**
     * create variable
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody new variable
     * @param data.alreadyEncrypted
     * @returns string variable created
     * @throws ApiError
     */
    static createVariable(data: CreateVariableData): CancelablePromise<CreateVariableResponse>;
    /**
     * encrypt value
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody new variable
     * @returns string encrypted value
     * @throws ApiError
     */
    static encryptValue(data: EncryptValueData): CancelablePromise<EncryptValueResponse>;
    /**
     * delete variable
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string variable deleted
     * @throws ApiError
     */
    static deleteVariable(data: DeleteVariableData): CancelablePromise<DeleteVariableResponse>;
    /**
     * update variable
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody updated variable
     * @param data.alreadyEncrypted
     * @returns string variable updated
     * @throws ApiError
     */
    static updateVariable(data: UpdateVariableData): CancelablePromise<UpdateVariableResponse>;
    /**
     * get variable
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.decryptSecret ask to decrypt secret if this variable is secret
     * (if not secret no effect, default: true)
     *
     * @param data.includeEncrypted ask to include the encrypted value if secret and decrypt secret is not true (default: false)
     *
     * @returns ListableVariable variable
     * @throws ApiError
     */
    static getVariable(data: GetVariableData): CancelablePromise<GetVariableResponse>;
    /**
     * get variable value
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string variable
     * @throws ApiError
     */
    static getVariableValue(data: GetVariableValueData): CancelablePromise<GetVariableValueResponse>;
    /**
     * does variable exists at path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns boolean variable
     * @throws ApiError
     */
    static existsVariable(data: ExistsVariableData): CancelablePromise<ExistsVariableResponse>;
    /**
     * list variables
     * @param data The data for the request.
     * @param data.workspace
     * @returns ListableVariable variable list
     * @throws ApiError
     */
    static listVariable(data: ListVariableData): CancelablePromise<ListVariableResponse>;
    /**
     * list contextual variables
     * @param data The data for the request.
     * @param data.workspace
     * @returns ContextualVariable contextual variable list
     * @throws ApiError
     */
    static listContextualVariables(data: ListContextualVariablesData): CancelablePromise<ListContextualVariablesResponse>;
}
export declare class OauthService {
    /**
     * connect slack callback
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody code endpoint
     * @returns string slack token
     * @throws ApiError
     */
    static connectSlackCallback(data: ConnectSlackCallbackData): CancelablePromise<ConnectSlackCallbackResponse>;
    /**
     * connect callback
     * @param data The data for the request.
     * @param data.clientName
     * @param data.requestBody code endpoint
     * @returns TokenResponse oauth token
     * @throws ApiError
     */
    static connectCallback(data: ConnectCallbackData): CancelablePromise<ConnectCallbackResponse>;
    /**
     * create OAuth account
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody code endpoint
     * @returns string account set
     * @throws ApiError
     */
    static createAccount(data: CreateAccountData): CancelablePromise<CreateAccountResponse>;
    /**
     * refresh token
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.requestBody variable path
     * @returns string token refreshed
     * @throws ApiError
     */
    static refreshToken(data: RefreshTokenData): CancelablePromise<RefreshTokenResponse>;
    /**
     * disconnect account
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @returns string disconnected client
     * @throws ApiError
     */
    static disconnectAccount(data: DisconnectAccountData): CancelablePromise<DisconnectAccountResponse>;
    /**
     * disconnect slack
     * @param data The data for the request.
     * @param data.workspace
     * @returns string disconnected slack
     * @throws ApiError
     */
    static disconnectSlack(data: DisconnectSlackData): CancelablePromise<DisconnectSlackResponse>;
    /**
     * list oauth logins
     * @returns unknown list of oauth and saml login clients
     * @throws ApiError
     */
    static listOauthLogins(): CancelablePromise<ListOauthLoginsResponse>;
    /**
     * list oauth connects
     * @returns unknown list of oauth connects clients
     * @throws ApiError
     */
    static listOauthConnects(): CancelablePromise<ListOauthConnectsResponse>;
}
export declare class ResourceService {
    /**
     * create resource
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody new resource
     * @param data.updateIfExists
     * @returns string resource created
     * @throws ApiError
     */
    static createResource(data: CreateResourceData): CancelablePromise<CreateResourceResponse>;
    /**
     * delete resource
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string resource deleted
     * @throws ApiError
     */
    static deleteResource(data: DeleteResourceData): CancelablePromise<DeleteResourceResponse>;
    /**
     * update resource
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody updated resource
     * @returns string resource updated
     * @throws ApiError
     */
    static updateResource(data: UpdateResourceData): CancelablePromise<UpdateResourceResponse>;
    /**
     * update resource value
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody updated resource
     * @returns string resource value updated
     * @throws ApiError
     */
    static updateResourceValue(data: UpdateResourceValueData): CancelablePromise<UpdateResourceValueResponse>;
    /**
     * get resource
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns Resource resource
     * @throws ApiError
     */
    static getResource(data: GetResourceData): CancelablePromise<GetResourceResponse>;
    /**
     * get resource interpolated (variables and resources are fully unrolled)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.jobId job id
     * @returns unknown resource value
     * @throws ApiError
     */
    static getResourceValueInterpolated(data: GetResourceValueInterpolatedData): CancelablePromise<GetResourceValueInterpolatedResponse>;
    /**
     * get resource value
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns unknown resource value
     * @throws ApiError
     */
    static getResourceValue(data: GetResourceValueData): CancelablePromise<GetResourceValueResponse>;
    /**
     * does resource exists
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns boolean does resource exists
     * @throws ApiError
     */
    static existsResource(data: ExistsResourceData): CancelablePromise<ExistsResourceResponse>;
    /**
     * list resources
     * @param data The data for the request.
     * @param data.workspace
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @param data.resourceType resource_types to list from, separated by ',',
     * @param data.resourceTypeExclude resource_types to not list from, separated by ',',
     * @returns ListableResource resource list
     * @throws ApiError
     */
    static listResource(data: ListResourceData): CancelablePromise<ListResourceResponse>;
    /**
     * list resources for search
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown resource list
     * @throws ApiError
     */
    static listSearchResource(data: ListSearchResourceData): CancelablePromise<ListSearchResourceResponse>;
    /**
     * list resource names
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @returns unknown resource list names
     * @throws ApiError
     */
    static listResourceNames(data: ListResourceNamesData): CancelablePromise<ListResourceNamesResponse>;
    /**
     * create resource_type
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody new resource_type
     * @returns string resource_type created
     * @throws ApiError
     */
    static createResourceType(data: CreateResourceTypeData): CancelablePromise<CreateResourceTypeResponse>;
    /**
     * delete resource_type
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string resource_type deleted
     * @throws ApiError
     */
    static deleteResourceType(data: DeleteResourceTypeData): CancelablePromise<DeleteResourceTypeResponse>;
    /**
     * update resource_type
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody updated resource_type
     * @returns string resource_type updated
     * @throws ApiError
     */
    static updateResourceType(data: UpdateResourceTypeData): CancelablePromise<UpdateResourceTypeResponse>;
    /**
     * get resource_type
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns ResourceType resource_type deleted
     * @throws ApiError
     */
    static getResourceType(data: GetResourceTypeData): CancelablePromise<GetResourceTypeResponse>;
    /**
     * does resource_type exists
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns boolean does resource_type exist
     * @throws ApiError
     */
    static existsResourceType(data: ExistsResourceTypeData): CancelablePromise<ExistsResourceTypeResponse>;
    /**
     * list resource_types
     * @param data The data for the request.
     * @param data.workspace
     * @returns ResourceType resource_type list
     * @throws ApiError
     */
    static listResourceType(data: ListResourceTypeData): CancelablePromise<ListResourceTypeResponse>;
    /**
     * list resource_types names
     * @param data The data for the request.
     * @param data.workspace
     * @returns string resource_type list
     * @throws ApiError
     */
    static listResourceTypeNames(data: ListResourceTypeNamesData): CancelablePromise<ListResourceTypeNamesResponse>;
    /**
     * query resource types by similarity
     * @param data The data for the request.
     * @param data.workspace
     * @param data.text query text
     * @param data.limit query limit
     * @returns unknown resource type details
     * @throws ApiError
     */
    static queryResourceTypes(data: QueryResourceTypesData): CancelablePromise<QueryResourceTypesResponse>;
}
export declare class IntegrationService {
    /**
     * list hub integrations
     * @param data The data for the request.
     * @param data.kind query integrations kind
     * @returns unknown integrations details
     * @throws ApiError
     */
    static listHubIntegrations(data?: ListHubIntegrationsData): CancelablePromise<ListHubIntegrationsResponse>;
}
export declare class FlowService {
    /**
     * list all hub flows
     * @returns unknown hub flows list
     * @throws ApiError
     */
    static listHubFlows(): CancelablePromise<ListHubFlowsResponse>;
    /**
     * get hub flow by id
     * @param data The data for the request.
     * @param data.id
     * @returns unknown flow
     * @throws ApiError
     */
    static getHubFlowById(data: GetHubFlowByIdData): CancelablePromise<GetHubFlowByIdResponse>;
    /**
     * list all flow paths
     * @param data The data for the request.
     * @param data.workspace
     * @returns string list of flow paths
     * @throws ApiError
     */
    static listFlowPaths(data: ListFlowPathsData): CancelablePromise<ListFlowPathsResponse>;
    /**
     * list flows for search
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown flow list
     * @throws ApiError
     */
    static listSearchFlow(data: ListSearchFlowData): CancelablePromise<ListSearchFlowResponse>;
    /**
     * list all flows
     * @param data The data for the request.
     * @param data.workspace
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @param data.orderDesc order by desc order (default true)
     * @param data.createdBy mask to filter exact matching user creator
     * @param data.pathStart mask to filter matching starting path
     * @param data.pathExact mask to filter exact matching path
     * @param data.showArchived (default false)
     * show also the archived files.
     * when multiple archived hash share the same path, only the ones with the latest create_at
     * are displayed.
     *
     * @param data.starredOnly (default false)
     * show only the starred items
     *
     * @returns unknown All flow
     * @throws ApiError
     */
    static listFlows(data: ListFlowsData): CancelablePromise<ListFlowsResponse>;
    /**
     * get flow by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns Flow flow details
     * @throws ApiError
     */
    static getFlowByPath(data: GetFlowByPathData): CancelablePromise<GetFlowByPathResponse>;
    /**
     * Toggle ON and OFF the workspace error handler for a given flow
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody Workspace error handler enabled
     * @returns string error handler toggled
     * @throws ApiError
     */
    static toggleWorkspaceErrorHandlerForFlow(data: ToggleWorkspaceErrorHandlerForFlowData): CancelablePromise<ToggleWorkspaceErrorHandlerForFlowResponse>;
    /**
     * get flow by path with draft
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns unknown flow details with draft
     * @throws ApiError
     */
    static getFlowByPathWithDraft(data: GetFlowByPathWithDraftData): CancelablePromise<GetFlowByPathWithDraftResponse>;
    /**
     * exists flow by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns boolean flow details
     * @throws ApiError
     */
    static existsFlowByPath(data: ExistsFlowByPathData): CancelablePromise<ExistsFlowByPathResponse>;
    /**
     * create flow
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody Partially filled flow
     * @returns string flow created
     * @throws ApiError
     */
    static createFlow(data: CreateFlowData): CancelablePromise<CreateFlowResponse>;
    /**
     * update flow
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody Partially filled flow
     * @returns string flow updated
     * @throws ApiError
     */
    static updateFlow(data: UpdateFlowData): CancelablePromise<UpdateFlowResponse>;
    /**
     * archive flow by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody archiveFlow
     * @returns string flow archived
     * @throws ApiError
     */
    static archiveFlowByPath(data: ArchiveFlowByPathData): CancelablePromise<ArchiveFlowByPathResponse>;
    /**
     * delete flow by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string flow delete
     * @throws ApiError
     */
    static deleteFlowByPath(data: DeleteFlowByPathData): CancelablePromise<DeleteFlowByPathResponse>;
    /**
     * list inputs for previous completed flow jobs
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @returns Input input history for completed jobs with this flow path
     * @throws ApiError
     */
    static getFlowInputHistoryByPath(data: GetFlowInputHistoryByPathData): CancelablePromise<GetFlowInputHistoryByPathResponse>;
}
export declare class AppService {
    /**
     * list all hub apps
     * @returns unknown hub apps list
     * @throws ApiError
     */
    static listHubApps(): CancelablePromise<ListHubAppsResponse>;
    /**
     * get hub app by id
     * @param data The data for the request.
     * @param data.id
     * @returns unknown app
     * @throws ApiError
     */
    static getHubAppById(data: GetHubAppByIdData): CancelablePromise<GetHubAppByIdResponse>;
    /**
     * list apps for search
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown app list
     * @throws ApiError
     */
    static listSearchApp(data: ListSearchAppData): CancelablePromise<ListSearchAppResponse>;
    /**
     * list all apps
     * @param data The data for the request.
     * @param data.workspace
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @param data.orderDesc order by desc order (default true)
     * @param data.createdBy mask to filter exact matching user creator
     * @param data.pathStart mask to filter matching starting path
     * @param data.pathExact mask to filter exact matching path
     * @param data.starredOnly (default false)
     * show only the starred items
     *
     * @returns ListableApp All apps
     * @throws ApiError
     */
    static listApps(data: ListAppsData): CancelablePromise<ListAppsResponse>;
    /**
     * create app
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody new app
     * @returns string app created
     * @throws ApiError
     */
    static createApp(data: CreateAppData): CancelablePromise<CreateAppResponse>;
    /**
     * does an app exisst at path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns boolean app exists
     * @throws ApiError
     */
    static existsApp(data: ExistsAppData): CancelablePromise<ExistsAppResponse>;
    /**
     * get app by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns AppWithLastVersion app details
     * @throws ApiError
     */
    static getAppByPath(data: GetAppByPathData): CancelablePromise<GetAppByPathResponse>;
    /**
     * get app by path with draft
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns AppWithLastVersionWDraft app details with draft
     * @throws ApiError
     */
    static getAppByPathWithDraft(data: GetAppByPathWithDraftData): CancelablePromise<GetAppByPathWithDraftResponse>;
    /**
     * get app history by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns AppHistory app history
     * @throws ApiError
     */
    static getAppHistoryByPath(data: GetAppHistoryByPathData): CancelablePromise<GetAppHistoryByPathResponse>;
    /**
     * update app history
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.version
     * @param data.requestBody App deployment message
     * @returns string success
     * @throws ApiError
     */
    static updateAppHistory(data: UpdateAppHistoryData): CancelablePromise<UpdateAppHistoryResponse>;
    /**
     * get public app by secret
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns AppWithLastVersion app details
     * @throws ApiError
     */
    static getPublicAppBySecret(data: GetPublicAppBySecretData): CancelablePromise<GetPublicAppBySecretResponse>;
    /**
     * get public resource
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns unknown resource value
     * @throws ApiError
     */
    static getPublicResource(data: GetPublicResourceData): CancelablePromise<GetPublicResourceResponse>;
    /**
     * get public secret of app
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string app secret
     * @throws ApiError
     */
    static getPublicSecretOfApp(data: GetPublicSecretOfAppData): CancelablePromise<GetPublicSecretOfAppResponse>;
    /**
     * get app by version
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @returns AppWithLastVersion app details
     * @throws ApiError
     */
    static getAppByVersion(data: GetAppByVersionData): CancelablePromise<GetAppByVersionResponse>;
    /**
     * delete app
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string app deleted
     * @throws ApiError
     */
    static deleteApp(data: DeleteAppData): CancelablePromise<DeleteAppResponse>;
    /**
     * update app
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody update app
     * @returns string app updated
     * @throws ApiError
     */
    static updateApp(data: UpdateAppData): CancelablePromise<UpdateAppResponse>;
    /**
     * executeComponent
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody update app
     * @returns string job uuid
     * @throws ApiError
     */
    static executeComponent(data: ExecuteComponentData): CancelablePromise<ExecuteComponentResponse>;
}
export declare class ScriptService {
    /**
     * get hub script content by path
     * @param data The data for the request.
     * @param data.path
     * @returns string script details
     * @throws ApiError
     */
    static getHubScriptContentByPath(data: GetHubScriptContentByPathData): CancelablePromise<GetHubScriptContentByPathResponse>;
    /**
     * get full hub script by path
     * @param data The data for the request.
     * @param data.path
     * @returns unknown script details
     * @throws ApiError
     */
    static getHubScriptByPath(data: GetHubScriptByPathData): CancelablePromise<GetHubScriptByPathResponse>;
    /**
     * get top hub scripts
     * @param data The data for the request.
     * @param data.limit query limit
     * @param data.app query scripts app
     * @param data.kind query scripts kind
     * @returns unknown hub scripts list
     * @throws ApiError
     */
    static getTopHubScripts(data?: GetTopHubScriptsData): CancelablePromise<GetTopHubScriptsResponse>;
    /**
     * query hub scripts by similarity
     * @param data The data for the request.
     * @param data.text query text
     * @param data.kind query scripts kind
     * @param data.limit query limit
     * @param data.app query scripts app
     * @returns unknown script details
     * @throws ApiError
     */
    static queryHubScripts(data: QueryHubScriptsData): CancelablePromise<QueryHubScriptsResponse>;
    /**
     * list scripts for search
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown script list
     * @throws ApiError
     */
    static listSearchScript(data: ListSearchScriptData): CancelablePromise<ListSearchScriptResponse>;
    /**
     * list all scripts
     * @param data The data for the request.
     * @param data.workspace
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @param data.orderDesc order by desc order (default true)
     * @param data.createdBy mask to filter exact matching user creator
     * @param data.pathStart mask to filter matching starting path
     * @param data.pathExact mask to filter exact matching path
     * @param data.firstParentHash mask to filter scripts whom first direct parent has exact hash
     * @param data.lastParentHash mask to filter scripts whom last parent in the chain has exact hash.
     * Beware that each script stores only a limited number of parents. Hence
     * the last parent hash for a script is not necessarily its top-most parent.
     * To find the top-most parent you will have to jump from last to last hash
     * until finding the parent
     *
     * @param data.parentHash is the hash present in the array of stored parent hashes for this script.
     * The same warning applies than for last_parent_hash. A script only store a
     * limited number of direct parent
     *
     * @param data.showArchived (default false)
     * show also the archived files.
     * when multiple archived hash share the same path, only the ones with the latest create_at
     * are
     * ed.
     *
     * @param data.hideWithoutMain (default false)
     * hide the scripts without an exported main function
     *
     * @param data.isTemplate (default regardless)
     * if true show only the templates
     * if false show only the non templates
     * if not defined, show all regardless of if the script is a template
     *
     * @param data.kinds (default regardless)
     * script kinds to filter, split by comma
     *
     * @param data.starredOnly (default false)
     * show only the starred items
     *
     * @returns Script All scripts
     * @throws ApiError
     */
    static listScripts(data: ListScriptsData): CancelablePromise<ListScriptsResponse>;
    /**
     * list all scripts paths
     * @param data The data for the request.
     * @param data.workspace
     * @returns string list of script paths
     * @throws ApiError
     */
    static listScriptPaths(data: ListScriptPathsData): CancelablePromise<ListScriptPathsResponse>;
    /**
     * create script
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody Partially filled script
     * @returns string script created
     * @throws ApiError
     */
    static createScript(data: CreateScriptData): CancelablePromise<CreateScriptResponse>;
    /**
     * Toggle ON and OFF the workspace error handler for a given script
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody Workspace error handler enabled
     * @returns string error handler toggled
     * @throws ApiError
     */
    static toggleWorkspaceErrorHandlerForScript(data: ToggleWorkspaceErrorHandlerForScriptData): CancelablePromise<ToggleWorkspaceErrorHandlerForScriptResponse>;
    /**
     * archive script by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string script archived
     * @throws ApiError
     */
    static archiveScriptByPath(data: ArchiveScriptByPathData): CancelablePromise<ArchiveScriptByPathResponse>;
    /**
     * archive script by hash
     * @param data The data for the request.
     * @param data.workspace
     * @param data.hash
     * @returns Script script details
     * @throws ApiError
     */
    static archiveScriptByHash(data: ArchiveScriptByHashData): CancelablePromise<ArchiveScriptByHashResponse>;
    /**
     * delete script by hash (erase content but keep hash, require admin)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.hash
     * @returns Script script details
     * @throws ApiError
     */
    static deleteScriptByHash(data: DeleteScriptByHashData): CancelablePromise<DeleteScriptByHashResponse>;
    /**
     * delete all scripts at a given path (require admin)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string script path
     * @throws ApiError
     */
    static deleteScriptByPath(data: DeleteScriptByPathData): CancelablePromise<DeleteScriptByPathResponse>;
    /**
     * get script by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns Script script details
     * @throws ApiError
     */
    static getScriptByPath(data: GetScriptByPathData): CancelablePromise<GetScriptByPathResponse>;
    /**
     * get script by path with draft
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns NewScriptWithDraft script details
     * @throws ApiError
     */
    static getScriptByPathWithDraft(data: GetScriptByPathWithDraftData): CancelablePromise<GetScriptByPathWithDraftResponse>;
    /**
     * get history of a script by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns ScriptHistory script history
     * @throws ApiError
     */
    static getScriptHistoryByPath(data: GetScriptHistoryByPathData): CancelablePromise<GetScriptHistoryByPathResponse>;
    /**
     * update history of a script
     * @param data The data for the request.
     * @param data.workspace
     * @param data.hash
     * @param data.path
     * @param data.requestBody Script deployment message
     * @returns string success
     * @throws ApiError
     */
    static updateScriptHistory(data: UpdateScriptHistoryData): CancelablePromise<UpdateScriptHistoryResponse>;
    /**
     * raw script by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string script content
     * @throws ApiError
     */
    static rawScriptByPath(data: RawScriptByPathData): CancelablePromise<RawScriptByPathResponse>;
    /**
     * raw script by path with a token (mostly used by lsp to be used with import maps to resolve scripts)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.token
     * @param data.path
     * @returns string script content
     * @throws ApiError
     */
    static rawScriptByPathTokened(data: RawScriptByPathTokenedData): CancelablePromise<RawScriptByPathTokenedResponse>;
    /**
     * exists script by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns boolean does it exists
     * @throws ApiError
     */
    static existsScriptByPath(data: ExistsScriptByPathData): CancelablePromise<ExistsScriptByPathResponse>;
    /**
     * get script by hash
     * @param data The data for the request.
     * @param data.workspace
     * @param data.hash
     * @returns Script script details
     * @throws ApiError
     */
    static getScriptByHash(data: GetScriptByHashData): CancelablePromise<GetScriptByHashResponse>;
    /**
     * raw script by hash
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string script content
     * @throws ApiError
     */
    static rawScriptByHash(data: RawScriptByHashData): CancelablePromise<RawScriptByHashResponse>;
    /**
     * get script deployment status
     * @param data The data for the request.
     * @param data.workspace
     * @param data.hash
     * @returns unknown script details
     * @throws ApiError
     */
    static getScriptDeploymentStatus(data: GetScriptDeploymentStatusData): CancelablePromise<GetScriptDeploymentStatusResponse>;
}
export declare class DraftService {
    /**
     * create draft
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody
     * @returns string draft created
     * @throws ApiError
     */
    static createDraft(data: CreateDraftData): CancelablePromise<CreateDraftResponse>;
    /**
     * delete draft
     * @param data The data for the request.
     * @param data.workspace
     * @param data.kind
     * @param data.path
     * @returns string draft deleted
     * @throws ApiError
     */
    static deleteDraft(data: DeleteDraftData): CancelablePromise<DeleteDraftResponse>;
}
export declare class WorkerService {
    /**
     * get all instance custom tags (tags are used to dispatch jobs to different worker groups)
     * @returns string list of custom tags
     * @throws ApiError
     */
    static getCustomTags(): CancelablePromise<GetCustomTagsResponse>;
    /**
     * get all instance default tags
     * @returns string list of default tags
     * @throws ApiError
     */
    static geDefaultTags(): CancelablePromise<GeDefaultTagsResponse>;
    /**
     * is default tags per workspace
     * @returns boolean is the default tags per workspace
     * @throws ApiError
     */
    static isDefaultTagsPerWorkspace(): CancelablePromise<IsDefaultTagsPerWorkspaceResponse>;
    /**
     * list workers
     * @param data The data for the request.
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @param data.pingSince number of seconds the worker must have had a last ping more recent of (default to 300)
     * @returns WorkerPing a list of workers
     * @throws ApiError
     */
    static listWorkers(data?: ListWorkersData): CancelablePromise<ListWorkersResponse>;
    /**
     * exists worker with tag
     * @param data The data for the request.
     * @param data.tag
     * @returns boolean whether a worker with the tag exists
     * @throws ApiError
     */
    static existsWorkerWithTag(data: ExistsWorkerWithTagData): CancelablePromise<ExistsWorkerWithTagResponse>;
    /**
     * get queue metrics
     * @returns unknown metrics
     * @throws ApiError
     */
    static getQueueMetrics(): CancelablePromise<GetQueueMetricsResponse>;
}
export declare class JobService {
    /**
     * run script by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody script args
     * @param data.scheduledFor when to schedule this job (leave empty for immediate run)
     * @param data.scheduledInSecs schedule the script to execute in the number of seconds starting now
     * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param data.tag Override the tag to use
     * @param data.cacheTtl Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
     * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     * @param data.invisibleToOwner make the run invisible to the the script owner (default false)
     * @returns string job created
     * @throws ApiError
     */
    static runScriptByPath(data: RunScriptByPathData): CancelablePromise<RunScriptByPathResponse>;
    /**
     * run script by path in openai format
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody script args
     * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     * @param data.queueLimit The maximum size of the queue for which the request would get rejected if that job would push it above that limit
     *
     * @returns unknown job result
     * @throws ApiError
     */
    static openaiSyncScriptByPath(data: OpenaiSyncScriptByPathData): CancelablePromise<OpenaiSyncScriptByPathResponse>;
    /**
     * run script by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody script args
     * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param data.tag Override the tag to use
     * @param data.cacheTtl Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
     * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     * @param data.queueLimit The maximum size of the queue for which the request would get rejected if that job would push it above that limit
     *
     * @returns unknown job result
     * @throws ApiError
     */
    static runWaitResultScriptByPath(data: RunWaitResultScriptByPathData): CancelablePromise<RunWaitResultScriptByPathResponse>;
    /**
     * run script by path with get
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param data.tag Override the tag to use
     * @param data.cacheTtl Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
     * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     * @param data.queueLimit The maximum size of the queue for which the request would get rejected if that job would push it above that limit
     *
     * @param data.payload The base64 encoded payload that has been encoded as a JSON. e.g how to encode such payload encodeURIComponent
     * `encodeURIComponent(btoa(JSON.stringify({a: 2})))`
     *
     * @returns unknown job result
     * @throws ApiError
     */
    static runWaitResultScriptByPathGet(data: RunWaitResultScriptByPathGetData): CancelablePromise<RunWaitResultScriptByPathGetResponse>;
    /**
     * run flow by path and wait until completion in openai format
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody script args
     * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     * @param data.queueLimit The maximum size of the queue for which the request would get rejected if that job would push it above that limit
     *
     * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     * @returns unknown job result
     * @throws ApiError
     */
    static openaiSyncFlowByPath(data: OpenaiSyncFlowByPathData): CancelablePromise<OpenaiSyncFlowByPathResponse>;
    /**
     * run flow by path and wait until completion
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody script args
     * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     * @param data.queueLimit The maximum size of the queue for which the request would get rejected if that job would push it above that limit
     *
     * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     * @returns unknown job result
     * @throws ApiError
     */
    static runWaitResultFlowByPath(data: RunWaitResultFlowByPathData): CancelablePromise<RunWaitResultFlowByPathResponse>;
    /**
     * get job result by id
     * @param data The data for the request.
     * @param data.workspace
     * @param data.flowJobId
     * @param data.nodeId
     * @returns unknown job result
     * @throws ApiError
     */
    static resultById(data: ResultByIdData): CancelablePromise<ResultByIdResponse>;
    /**
     * run flow by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody flow args
     * @param data.scheduledFor when to schedule this job (leave empty for immediate run)
     * @param data.scheduledInSecs schedule the script to execute in the number of seconds starting now
     * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param data.tag Override the tag to use
     * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     * @param data.invisibleToOwner make the run invisible to the the flow owner (default false)
     * @returns string job created
     * @throws ApiError
     */
    static runFlowByPath(data: RunFlowByPathData): CancelablePromise<RunFlowByPathResponse>;
    /**
     * restart a completed flow at a given step
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.stepId step id to restart the flow from
     * @param data.branchOrIterationN for branchall or loop, the iteration at which the flow should restart
     * @param data.requestBody flow args
     * @param data.scheduledFor when to schedule this job (leave empty for immediate run)
     * @param data.scheduledInSecs schedule the script to execute in the number of seconds starting now
     * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param data.tag Override the tag to use
     * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     * @param data.invisibleToOwner make the run invisible to the the flow owner (default false)
     * @returns string job created
     * @throws ApiError
     */
    static restartFlowAtStep(data: RestartFlowAtStepData): CancelablePromise<RestartFlowAtStepResponse>;
    /**
     * run script by hash
     * @param data The data for the request.
     * @param data.workspace
     * @param data.hash
     * @param data.requestBody Partially filled args
     * @param data.scheduledFor when to schedule this job (leave empty for immediate run)
     * @param data.scheduledInSecs schedule the script to execute in the number of seconds starting now
     * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param data.tag Override the tag to use
     * @param data.cacheTtl Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
     * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     * @param data.invisibleToOwner make the run invisible to the the script owner (default false)
     * @returns string job created
     * @throws ApiError
     */
    static runScriptByHash(data: RunScriptByHashData): CancelablePromise<RunScriptByHashResponse>;
    /**
     * run script preview
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody preview
     * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     * @param data.invisibleToOwner make the run invisible to the the script owner (default false)
     * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     * @returns string job created
     * @throws ApiError
     */
    static runScriptPreview(data: RunScriptPreviewData): CancelablePromise<RunScriptPreviewResponse>;
    /**
     * run code-workflow task
     * @param data The data for the request.
     * @param data.workspace
     * @param data.jobId
     * @param data.entrypoint
     * @param data.requestBody preview
     * @returns string job created
     * @throws ApiError
     */
    static runCodeWorkflowTask(data: RunCodeWorkflowTaskData): CancelablePromise<RunCodeWorkflowTaskResponse>;
    /**
     * run a one-off dependencies job
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody raw script content
     * @returns unknown dependency job result
     * @throws ApiError
     */
    static runRawScriptDependencies(data: RunRawScriptDependenciesData): CancelablePromise<RunRawScriptDependenciesResponse>;
    /**
     * run flow preview
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody preview
     * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     * @param data.invisibleToOwner make the run invisible to the the script owner (default false)
     * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     * @returns string job created
     * @throws ApiError
     */
    static runFlowPreview(data: RunFlowPreviewData): CancelablePromise<RunFlowPreviewResponse>;
    /**
     * list all queued jobs
     * @param data The data for the request.
     * @param data.workspace
     * @param data.orderDesc order by desc order (default true)
     * @param data.createdBy mask to filter exact matching user creator
     * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param data.scriptPathExact mask to filter exact matching path
     * @param data.scriptPathStart mask to filter matching starting path
     * @param data.schedulePath mask to filter by schedule path
     * @param data.scriptHash mask to filter exact matching path
     * @param data.startedBefore filter on started before (inclusive) timestamp
     * @param data.startedAfter filter on started after (exclusive) timestamp
     * @param data.success filter on successful jobs
     * @param data.scheduledForBeforeNow filter on jobs scheduled_for before now (hence waitinf for a worker)
     * @param data.jobKinds filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
     * @param data.suspended filter on suspended jobs
     * @param data.running filter on running jobs
     * @param data.args filter on jobs containing those args as a json subset (@> in postgres)
     * @param data.result filter on jobs containing those result as a json subset (@> in postgres)
     * @param data.tag filter on jobs with a given tag/worker group
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @param data.allWorkspaces get jobs from all workspaces (only valid if request come from the `admins` workspace)
     * @param data.isNotSchedule is not a scheduled job
     * @returns QueuedJob All queued jobs
     * @throws ApiError
     */
    static listQueue(data: ListQueueData): CancelablePromise<ListQueueResponse>;
    /**
     * get queue count
     * @param data The data for the request.
     * @param data.workspace
     * @param data.allWorkspaces get jobs from all workspaces (only valid if request come from the `admins` workspace)
     * @returns unknown queue count
     * @throws ApiError
     */
    static getQueueCount(data: GetQueueCountData): CancelablePromise<GetQueueCountResponse>;
    /**
     * get completed count
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown completed count
     * @throws ApiError
     */
    static getCompletedCount(data: GetCompletedCountData): CancelablePromise<GetCompletedCountResponse>;
    /**
     * cancel all jobs
     * @param data The data for the request.
     * @param data.workspace
     * @returns string uuids of canceled jobs
     * @throws ApiError
     */
    static cancelAll(data: CancelAllData): CancelablePromise<CancelAllResponse>;
    /**
     * list all completed jobs
     * @param data The data for the request.
     * @param data.workspace
     * @param data.orderDesc order by desc order (default true)
     * @param data.createdBy mask to filter exact matching user creator
     * @param data.label mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
     * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param data.scriptPathExact mask to filter exact matching path
     * @param data.scriptPathStart mask to filter matching starting path
     * @param data.schedulePath mask to filter by schedule path
     * @param data.scriptHash mask to filter exact matching path
     * @param data.startedBefore filter on started before (inclusive) timestamp
     * @param data.startedAfter filter on started after (exclusive) timestamp
     * @param data.success filter on successful jobs
     * @param data.jobKinds filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
     * @param data.args filter on jobs containing those args as a json subset (@> in postgres)
     * @param data.result filter on jobs containing those result as a json subset (@> in postgres)
     * @param data.tag filter on jobs with a given tag/worker group
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @param data.isSkipped is the job skipped
     * @param data.isFlowStep is the job a flow step
     * @param data.hasNullParent has null parent
     * @param data.isNotSchedule is not a scheduled job
     * @returns CompletedJob All completed jobs
     * @throws ApiError
     */
    static listCompletedJobs(data: ListCompletedJobsData): CancelablePromise<ListCompletedJobsResponse>;
    /**
     * list all jobs
     * @param data The data for the request.
     * @param data.workspace
     * @param data.createdBy mask to filter exact matching user creator
     * @param data.label mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
     * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param data.scriptPathExact mask to filter exact matching path
     * @param data.scriptPathStart mask to filter matching starting path
     * @param data.schedulePath mask to filter by schedule path
     * @param data.scriptHash mask to filter exact matching path
     * @param data.startedBefore filter on started before (inclusive) timestamp
     * @param data.startedAfter filter on started after (exclusive) timestamp
     * @param data.createdOrStartedBefore filter on created_at for non non started job and started_at otherwise before (inclusive) timestamp
     * @param data.running filter on running jobs
     * @param data.scheduledForBeforeNow filter on jobs scheduled_for before now (hence waitinf for a worker)
     * @param data.createdOrStartedAfter filter on created_at for non non started job and started_at otherwise after (exclusive) timestamp
     * @param data.jobKinds filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
     * @param data.args filter on jobs containing those args as a json subset (@> in postgres)
     * @param data.tag filter on jobs with a given tag/worker group
     * @param data.result filter on jobs containing those result as a json subset (@> in postgres)
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @param data.isSkipped is the job skipped
     * @param data.isFlowStep is the job a flow step
     * @param data.hasNullParent has null parent
     * @param data.success filter on successful jobs
     * @param data.allWorkspaces get jobs from all workspaces (only valid if request come from the `admins` workspace)
     * @param data.isNotSchedule is not a scheduled job
     * @returns Job All jobs
     * @throws ApiError
     */
    static listJobs(data: ListJobsData): CancelablePromise<ListJobsResponse>;
    /**
     * get db clock
     * @returns number the timestamp of the db that can be used to compute the drift
     * @throws ApiError
     */
    static getDbClock(): CancelablePromise<GetDbClockResponse>;
    /**
     * get job
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.noLogs
     * @returns Job job details
     * @throws ApiError
     */
    static getJob(data: GetJobData): CancelablePromise<GetJobResponse>;
    /**
     * get root job id
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @returns string get root job id
     * @throws ApiError
     */
    static getRootJobId(data: GetRootJobIdData): CancelablePromise<GetRootJobIdResponse>;
    /**
     * get job logs
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @returns string job details
     * @throws ApiError
     */
    static getJobLogs(data: GetJobLogsData): CancelablePromise<GetJobLogsResponse>;
    /**
     * get job updates
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.running
     * @param data.logOffset
     * @returns unknown job details
     * @throws ApiError
     */
    static getJobUpdates(data: GetJobUpdatesData): CancelablePromise<GetJobUpdatesResponse>;
    /**
     * get log file from object store
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns unknown job log
     * @throws ApiError
     */
    static getLogFileFromStore(data: GetLogFileFromStoreData): CancelablePromise<GetLogFileFromStoreResponse>;
    /**
     * get flow debug info
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @returns unknown flow debug info details
     * @throws ApiError
     */
    static getFlowDebugInfo(data: GetFlowDebugInfoData): CancelablePromise<GetFlowDebugInfoResponse>;
    /**
     * get completed job
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @returns CompletedJob job details
     * @throws ApiError
     */
    static getCompletedJob(data: GetCompletedJobData): CancelablePromise<GetCompletedJobResponse>;
    /**
     * get completed job result
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @returns unknown result
     * @throws ApiError
     */
    static getCompletedJobResult(data: GetCompletedJobResultData): CancelablePromise<GetCompletedJobResultResponse>;
    /**
     * get completed job result if job is completed
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.getStarted
     * @returns unknown result
     * @throws ApiError
     */
    static getCompletedJobResultMaybe(data: GetCompletedJobResultMaybeData): CancelablePromise<GetCompletedJobResultMaybeResponse>;
    /**
     * delete completed job (erase content but keep run id)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @returns CompletedJob job details
     * @throws ApiError
     */
    static deleteCompletedJob(data: DeleteCompletedJobData): CancelablePromise<DeleteCompletedJobResponse>;
    /**
     * cancel queued job
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.requestBody reason
     * @returns string job canceled
     * @throws ApiError
     */
    static cancelQueuedJob(data: CancelQueuedJobData): CancelablePromise<CancelQueuedJobResponse>;
    /**
     * cancel all queued jobs for persistent script
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody reason
     * @returns string persistent job scaled down to zero
     * @throws ApiError
     */
    static cancelPersistentQueuedJobs(data: CancelPersistentQueuedJobsData): CancelablePromise<CancelPersistentQueuedJobsResponse>;
    /**
     * force cancel queued job
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.requestBody reason
     * @returns string job canceled
     * @throws ApiError
     */
    static forceCancelQueuedJob(data: ForceCancelQueuedJobData): CancelablePromise<ForceCancelQueuedJobResponse>;
    /**
     * create an HMac signature given a job id and a resume id
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.resumeId
     * @param data.approver
     * @returns string job signature
     * @throws ApiError
     */
    static createJobSignature(data: CreateJobSignatureData): CancelablePromise<CreateJobSignatureResponse>;
    /**
     * get resume urls given a job_id, resume_id and a nonce to resume a flow
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.resumeId
     * @param data.approver
     * @returns unknown url endpoints
     * @throws ApiError
     */
    static getResumeUrls(data: GetResumeUrlsData): CancelablePromise<GetResumeUrlsResponse>;
    /**
     * resume a job for a suspended flow
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.resumeId
     * @param data.signature
     * @param data.payload The base64 encoded payload that has been encoded as a JSON. e.g how to encode such payload encodeURIComponent
     * `encodeURIComponent(btoa(JSON.stringify({a: 2})))`
     *
     * @param data.approver
     * @returns string job resumed
     * @throws ApiError
     */
    static resumeSuspendedJobGet(data: ResumeSuspendedJobGetData): CancelablePromise<ResumeSuspendedJobGetResponse>;
    /**
     * resume a job for a suspended flow
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.resumeId
     * @param data.signature
     * @param data.requestBody
     * @param data.approver
     * @returns string job resumed
     * @throws ApiError
     */
    static resumeSuspendedJobPost(data: ResumeSuspendedJobPostData): CancelablePromise<ResumeSuspendedJobPostResponse>;
    /**
     * set flow user state at a given key
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.key
     * @param data.requestBody new value
     * @returns string flow user state updated
     * @throws ApiError
     */
    static setFlowUserState(data: SetFlowUserStateData): CancelablePromise<SetFlowUserStateResponse>;
    /**
     * get flow user state at a given key
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.key
     * @returns unknown flow user state updated
     * @throws ApiError
     */
    static getFlowUserState(data: GetFlowUserStateData): CancelablePromise<GetFlowUserStateResponse>;
    /**
     * resume a job for a suspended flow as an owner
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.requestBody
     * @returns string job resumed
     * @throws ApiError
     */
    static resumeSuspendedFlowAsOwner(data: ResumeSuspendedFlowAsOwnerData): CancelablePromise<ResumeSuspendedFlowAsOwnerResponse>;
    /**
     * cancel a job for a suspended flow
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.resumeId
     * @param data.signature
     * @param data.approver
     * @returns string job canceled
     * @throws ApiError
     */
    static cancelSuspendedJobGet(data: CancelSuspendedJobGetData): CancelablePromise<CancelSuspendedJobGetResponse>;
    /**
     * cancel a job for a suspended flow
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.resumeId
     * @param data.signature
     * @param data.requestBody
     * @param data.approver
     * @returns string job canceled
     * @throws ApiError
     */
    static cancelSuspendedJobPost(data: CancelSuspendedJobPostData): CancelablePromise<CancelSuspendedJobPostResponse>;
    /**
     * get parent flow job of suspended job
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.resumeId
     * @param data.signature
     * @param data.approver
     * @returns unknown parent flow details
     * @throws ApiError
     */
    static getSuspendedJobFlow(data: GetSuspendedJobFlowData): CancelablePromise<GetSuspendedJobFlowResponse>;
}
export declare class RawAppService {
    /**
     * list all raw apps
     * @param data The data for the request.
     * @param data.workspace
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @param data.orderDesc order by desc order (default true)
     * @param data.createdBy mask to filter exact matching user creator
     * @param data.pathStart mask to filter matching starting path
     * @param data.pathExact mask to filter exact matching path
     * @param data.starredOnly (default false)
     * show only the starred items
     *
     * @returns ListableRawApp All raw apps
     * @throws ApiError
     */
    static listRawApps(data: ListRawAppsData): CancelablePromise<ListRawAppsResponse>;
    /**
     * does an app exisst at path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns boolean app exists
     * @throws ApiError
     */
    static existsRawApp(data: ExistsRawAppData): CancelablePromise<ExistsRawAppResponse>;
    /**
     * get app by path
     * @param data The data for the request.
     * @param data.workspace
     * @param data.version
     * @param data.path
     * @returns string app details
     * @throws ApiError
     */
    static getRawAppData(data: GetRawAppDataData): CancelablePromise<GetRawAppDataResponse>;
    /**
     * create raw app
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody new raw app
     * @returns string raw app created
     * @throws ApiError
     */
    static createRawApp(data: CreateRawAppData): CancelablePromise<CreateRawAppResponse>;
    /**
     * update app
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody updateraw  app
     * @returns string app updated
     * @throws ApiError
     */
    static updateRawApp(data: UpdateRawAppData): CancelablePromise<UpdateRawAppResponse>;
    /**
     * delete raw app
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string app deleted
     * @throws ApiError
     */
    static deleteRawApp(data: DeleteRawAppData): CancelablePromise<DeleteRawAppResponse>;
}
export declare class ScheduleService {
    /**
     * preview schedule
     * @param data The data for the request.
     * @param data.requestBody schedule
     * @returns string List of 5 estimated upcoming execution events (in UTC)
     * @throws ApiError
     */
    static previewSchedule(data: PreviewScheduleData): CancelablePromise<PreviewScheduleResponse>;
    /**
     * create schedule
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody new schedule
     * @returns string schedule created
     * @throws ApiError
     */
    static createSchedule(data: CreateScheduleData): CancelablePromise<CreateScheduleResponse>;
    /**
     * update schedule
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody updated schedule
     * @returns string schedule updated
     * @throws ApiError
     */
    static updateSchedule(data: UpdateScheduleData): CancelablePromise<UpdateScheduleResponse>;
    /**
     * set enabled schedule
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.requestBody updated schedule enable
     * @returns string schedule enabled set
     * @throws ApiError
     */
    static setScheduleEnabled(data: SetScheduleEnabledData): CancelablePromise<SetScheduleEnabledResponse>;
    /**
     * delete schedule
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns string schedule deleted
     * @throws ApiError
     */
    static deleteSchedule(data: DeleteScheduleData): CancelablePromise<DeleteScheduleResponse>;
    /**
     * get schedule
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns Schedule schedule deleted
     * @throws ApiError
     */
    static getSchedule(data: GetScheduleData): CancelablePromise<GetScheduleResponse>;
    /**
     * does schedule exists
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns boolean schedule exists
     * @throws ApiError
     */
    static existsSchedule(data: ExistsScheduleData): CancelablePromise<ExistsScheduleResponse>;
    /**
     * list schedules
     * @param data The data for the request.
     * @param data.workspace
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @param data.args filter on jobs containing those args as a json subset (@> in postgres)
     * @param data.path filter by path
     * @param data.isFlow
     * @returns Schedule schedule list
     * @throws ApiError
     */
    static listSchedules(data: ListSchedulesData): CancelablePromise<ListSchedulesResponse>;
    /**
     * list schedules with last 20 jobs
     * @param data The data for the request.
     * @param data.workspace
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @returns ScheduleWJobs schedule list
     * @throws ApiError
     */
    static listSchedulesWithJobs(data: ListSchedulesWithJobsData): CancelablePromise<ListSchedulesWithJobsResponse>;
    /**
     * Set default error or recoevery handler
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody Handler description
     * @returns unknown default error handler set
     * @throws ApiError
     */
    static setDefaultErrorOrRecoveryHandler(data: SetDefaultErrorOrRecoveryHandlerData): CancelablePromise<SetDefaultErrorOrRecoveryHandlerResponse>;
}
export declare class GroupService {
    /**
     * list instance groups
     * @returns InstanceGroup instance group list
     * @throws ApiError
     */
    static listInstanceGroups(): CancelablePromise<ListInstanceGroupsResponse>;
    /**
     * get instance group
     * @param data The data for the request.
     * @param data.name
     * @returns InstanceGroup instance group
     * @throws ApiError
     */
    static getInstanceGroup(data: GetInstanceGroupData): CancelablePromise<GetInstanceGroupResponse>;
    /**
     * create instance group
     * @param data The data for the request.
     * @param data.requestBody create instance group
     * @returns string instance group created
     * @throws ApiError
     */
    static createInstanceGroup(data: CreateInstanceGroupData): CancelablePromise<CreateInstanceGroupResponse>;
    /**
     * update instance group
     * @param data The data for the request.
     * @param data.name
     * @param data.requestBody update instance group
     * @returns string instance group updated
     * @throws ApiError
     */
    static updateInstanceGroup(data: UpdateInstanceGroupData): CancelablePromise<UpdateInstanceGroupResponse>;
    /**
     * delete instance group
     * @param data The data for the request.
     * @param data.name
     * @returns string instance group deleted
     * @throws ApiError
     */
    static deleteInstanceGroup(data: DeleteInstanceGroupData): CancelablePromise<DeleteInstanceGroupResponse>;
    /**
     * add user to instance group
     * @param data The data for the request.
     * @param data.name
     * @param data.requestBody user to add to instance group
     * @returns string user added to instance group
     * @throws ApiError
     */
    static addUserToInstanceGroup(data: AddUserToInstanceGroupData): CancelablePromise<AddUserToInstanceGroupResponse>;
    /**
     * remove user from instance group
     * @param data The data for the request.
     * @param data.name
     * @param data.requestBody user to remove from instance group
     * @returns string user removed from instance group
     * @throws ApiError
     */
    static removeUserFromInstanceGroup(data: RemoveUserFromInstanceGroupData): CancelablePromise<RemoveUserFromInstanceGroupResponse>;
    /**
     * list groups
     * @param data The data for the request.
     * @param data.workspace
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @returns Group group list
     * @throws ApiError
     */
    static listGroups(data: ListGroupsData): CancelablePromise<ListGroupsResponse>;
    /**
     * list group names
     * @param data The data for the request.
     * @param data.workspace
     * @param data.onlyMemberOf only list the groups the user is member of (default false)
     * @returns string group list
     * @throws ApiError
     */
    static listGroupNames(data: ListGroupNamesData): CancelablePromise<ListGroupNamesResponse>;
    /**
     * create group
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody create group
     * @returns string group created
     * @throws ApiError
     */
    static createGroup(data: CreateGroupData): CancelablePromise<CreateGroupResponse>;
    /**
     * update group
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @param data.requestBody updated group
     * @returns string group updated
     * @throws ApiError
     */
    static updateGroup(data: UpdateGroupData): CancelablePromise<UpdateGroupResponse>;
    /**
     * delete group
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @returns string group deleted
     * @throws ApiError
     */
    static deleteGroup(data: DeleteGroupData): CancelablePromise<DeleteGroupResponse>;
    /**
     * get group
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @returns Group group
     * @throws ApiError
     */
    static getGroup(data: GetGroupData): CancelablePromise<GetGroupResponse>;
    /**
     * add user to group
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @param data.requestBody added user to group
     * @returns string user added to group
     * @throws ApiError
     */
    static addUserToGroup(data: AddUserToGroupData): CancelablePromise<AddUserToGroupResponse>;
    /**
     * remove user to group
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @param data.requestBody added user to group
     * @returns string user removed from group
     * @throws ApiError
     */
    static removeUserToGroup(data: RemoveUserToGroupData): CancelablePromise<RemoveUserToGroupResponse>;
}
export declare class FolderService {
    /**
     * list folders
     * @param data The data for the request.
     * @param data.workspace
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @returns Folder folder list
     * @throws ApiError
     */
    static listFolders(data: ListFoldersData): CancelablePromise<ListFoldersResponse>;
    /**
     * list folder names
     * @param data The data for the request.
     * @param data.workspace
     * @param data.onlyMemberOf only list the folders the user is member of (default false)
     * @returns string folder list
     * @throws ApiError
     */
    static listFolderNames(data: ListFolderNamesData): CancelablePromise<ListFolderNamesResponse>;
    /**
     * create folder
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody create folder
     * @returns string folder created
     * @throws ApiError
     */
    static createFolder(data: CreateFolderData): CancelablePromise<CreateFolderResponse>;
    /**
     * update folder
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @param data.requestBody update folder
     * @returns string folder updated
     * @throws ApiError
     */
    static updateFolder(data: UpdateFolderData): CancelablePromise<UpdateFolderResponse>;
    /**
     * delete folder
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @returns string folder deleted
     * @throws ApiError
     */
    static deleteFolder(data: DeleteFolderData): CancelablePromise<DeleteFolderResponse>;
    /**
     * get folder
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @returns Folder folder
     * @throws ApiError
     */
    static getFolder(data: GetFolderData): CancelablePromise<GetFolderResponse>;
    /**
     * get folder usage
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @returns unknown folder
     * @throws ApiError
     */
    static getFolderUsage(data: GetFolderUsageData): CancelablePromise<GetFolderUsageResponse>;
    /**
     * add owner to folder
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @param data.requestBody owner user to folder
     * @returns string owner added to folder
     * @throws ApiError
     */
    static addOwnerToFolder(data: AddOwnerToFolderData): CancelablePromise<AddOwnerToFolderResponse>;
    /**
     * remove owner to folder
     * @param data The data for the request.
     * @param data.workspace
     * @param data.name
     * @param data.requestBody added owner to folder
     * @returns string owner removed from folder
     * @throws ApiError
     */
    static removeOwnerToFolder(data: RemoveOwnerToFolderData): CancelablePromise<RemoveOwnerToFolderResponse>;
}
export declare class ConfigService {
    /**
     * list worker groups
     * @returns unknown a list of worker group configs
     * @throws ApiError
     */
    static listWorkerGroups(): CancelablePromise<ListWorkerGroupsResponse>;
    /**
     * get config
     * @param data The data for the request.
     * @param data.name
     * @returns unknown a config
     * @throws ApiError
     */
    static getConfig(data: GetConfigData): CancelablePromise<GetConfigResponse>;
    /**
     * Update config
     * @param data The data for the request.
     * @param data.name
     * @param data.requestBody worker group
     * @returns string Update a worker group
     * @throws ApiError
     */
    static updateConfig(data: UpdateConfigData): CancelablePromise<UpdateConfigResponse>;
    /**
     * Delete Config
     * @param data The data for the request.
     * @param data.name
     * @returns string Delete config
     * @throws ApiError
     */
    static deleteConfig(data: DeleteConfigData): CancelablePromise<DeleteConfigResponse>;
}
export declare class GranularAclService {
    /**
     * get granular acls
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.kind
     * @returns boolean acls
     * @throws ApiError
     */
    static getGranularAcls(data: GetGranularAclsData): CancelablePromise<GetGranularAclsResponse>;
    /**
     * add granular acls
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.kind
     * @param data.requestBody acl to add
     * @returns string granular acl added
     * @throws ApiError
     */
    static addGranularAcls(data: AddGranularAclsData): CancelablePromise<AddGranularAclsResponse>;
    /**
     * remove granular acls
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.kind
     * @param data.requestBody acl to add
     * @returns string granular acl removed
     * @throws ApiError
     */
    static removeGranularAcls(data: RemoveGranularAclsData): CancelablePromise<RemoveGranularAclsResponse>;
}
export declare class CaptureService {
    /**
     * update flow preview capture
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns void flow preview captured
     * @throws ApiError
     */
    static updateCapture(data: UpdateCaptureData): CancelablePromise<UpdateCaptureResponse>;
    /**
     * create flow preview capture
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns unknown flow preview capture created
     * @throws ApiError
     */
    static createCapture(data: CreateCaptureData): CancelablePromise<CreateCaptureResponse>;
    /**
     * get flow preview capture
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @returns unknown captured flow preview
     * @throws ApiError
     */
    static getCapture(data: GetCaptureData): CancelablePromise<GetCaptureResponse>;
}
export declare class FavoriteService {
    /**
     * star item
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody
     * @returns unknown star item
     * @throws ApiError
     */
    static star(data: StarData): CancelablePromise<StarResponse>;
    /**
     * unstar item
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody
     * @returns unknown unstar item
     * @throws ApiError
     */
    static unstar(data: UnstarData): CancelablePromise<UnstarResponse>;
}
export declare class InputService {
    /**
     * List Inputs used in previously completed jobs
     * @param data The data for the request.
     * @param data.workspace
     * @param data.runnableId
     * @param data.runnableType
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @returns Input Input history for completed jobs
     * @throws ApiError
     */
    static getInputHistory(data: GetInputHistoryData): CancelablePromise<GetInputHistoryResponse>;
    /**
     * Get args from history or saved input
     * @param data The data for the request.
     * @param data.workspace
     * @param data.jobOrInputId
     * @returns unknown args
     * @throws ApiError
     */
    static getArgsFromHistoryOrSavedInput(data: GetArgsFromHistoryOrSavedInputData): CancelablePromise<GetArgsFromHistoryOrSavedInputResponse>;
    /**
     * List saved Inputs for a Runnable
     * @param data The data for the request.
     * @param data.workspace
     * @param data.runnableId
     * @param data.runnableType
     * @param data.page which page to return (start at 1, default 1)
     * @param data.perPage number of items to return for a given page (default 30, max 100)
     * @returns Input Saved Inputs for a Runnable
     * @throws ApiError
     */
    static listInputs(data: ListInputsData): CancelablePromise<ListInputsResponse>;
    /**
     * Create an Input for future use in a script or flow
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody Input
     * @param data.runnableId
     * @param data.runnableType
     * @returns string Input created
     * @throws ApiError
     */
    static createInput(data: CreateInputData): CancelablePromise<CreateInputResponse>;
    /**
     * Update an Input
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody UpdateInput
     * @returns string Input updated
     * @throws ApiError
     */
    static updateInput(data: UpdateInputData): CancelablePromise<UpdateInputResponse>;
    /**
     * Delete a Saved Input
     * @param data The data for the request.
     * @param data.workspace
     * @param data.input
     * @returns string Input deleted
     * @throws ApiError
     */
    static deleteInput(data: DeleteInputData): CancelablePromise<DeleteInputResponse>;
}
export declare class HelpersService {
    /**
     * Converts an S3 resource to the set of instructions necessary to connect DuckDB to an S3 bucket
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody S3 resource to connect to
     * @returns unknown Connection settings
     * @throws ApiError
     */
    static duckdbConnectionSettings(data: DuckdbConnectionSettingsData): CancelablePromise<DuckdbConnectionSettingsResponse>;
    /**
     * Converts an S3 resource to the set of instructions necessary to connect DuckDB to an S3 bucket
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody S3 resource path to use to generate the connection settings. If empty, the S3 resource defined in the workspace settings will be used
     * @returns unknown Connection settings
     * @throws ApiError
     */
    static duckdbConnectionSettingsV2(data: DuckdbConnectionSettingsV2Data): CancelablePromise<DuckdbConnectionSettingsV2Response>;
    /**
     * Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody S3 resource to connect to
     * @returns unknown Connection settings
     * @throws ApiError
     */
    static polarsConnectionSettings(data: PolarsConnectionSettingsData): CancelablePromise<PolarsConnectionSettingsResponse>;
    /**
     * Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody S3 resource path to use to generate the connection settings. If empty, the S3 resource defined in the workspace settings will be used
     * @returns unknown Connection settings
     * @throws ApiError
     */
    static polarsConnectionSettingsV2(data: PolarsConnectionSettingsV2Data): CancelablePromise<PolarsConnectionSettingsV2Response>;
    /**
     * Returns the s3 resource associated to the provided path, or the workspace default S3 resource
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody S3 resource path to use. If empty, the S3 resource defined in the workspace settings will be used
     * @returns S3Resource Connection settings
     * @throws ApiError
     */
    static s3ResourceInfo(data: S3ResourceInfoData): CancelablePromise<S3ResourceInfoResponse>;
    /**
     * Test connection to the workspace datasets storage
     * @param data The data for the request.
     * @param data.workspace
     * @returns unknown Connection settings
     * @throws ApiError
     */
    static datasetStorageTestConnection(data: DatasetStorageTestConnectionData): CancelablePromise<DatasetStorageTestConnectionResponse>;
    /**
     * List the file keys available in the workspace files storage (S3)
     * @param data The data for the request.
     * @param data.workspace
     * @param data.maxKeys
     * @param data.marker
     * @param data.prefix
     * @returns unknown List of file keys
     * @throws ApiError
     */
    static listStoredFiles(data: ListStoredFilesData): CancelablePromise<ListStoredFilesResponse>;
    /**
     * Load metadata of the file
     * @param data The data for the request.
     * @param data.workspace
     * @param data.fileKey
     * @returns WindmillFileMetadata FileMetadata
     * @throws ApiError
     */
    static loadFileMetadata(data: LoadFileMetadataData): CancelablePromise<LoadFileMetadataResponse>;
    /**
     * Load a preview of the file
     * @param data The data for the request.
     * @param data.workspace
     * @param data.fileKey
     * @param data.fileSizeInBytes
     * @param data.fileMimeType
     * @param data.csvSeparator
     * @param data.csvHasHeader
     * @param data.readBytesFrom
     * @param data.readBytesLength
     * @returns WindmillFilePreview FilePreview
     * @throws ApiError
     */
    static loadFilePreview(data: LoadFilePreviewData): CancelablePromise<LoadFilePreviewResponse>;
    /**
     * Load a preview of a parquet file
     * @param data The data for the request.
     * @param data.workspace
     * @param data.path
     * @param data.offset
     * @param data.limit
     * @param data.sortCol
     * @param data.sortDesc
     * @param data.searchCol
     * @param data.searchTerm
     * @returns unknown Parquet Preview
     * @throws ApiError
     */
    static loadParquetPreview(data: LoadParquetPreviewData): CancelablePromise<LoadParquetPreviewResponse>;
    /**
     * Permanently delete file from S3
     * @param data The data for the request.
     * @param data.workspace
     * @param data.fileKey
     * @returns unknown Confirmation
     * @throws ApiError
     */
    static deleteS3File(data: DeleteS3FileData): CancelablePromise<DeleteS3FileResponse>;
    /**
     * Move a S3 file from one path to the other within the same bucket
     * @param data The data for the request.
     * @param data.workspace
     * @param data.srcFileKey
     * @param data.destFileKey
     * @returns unknown Confirmation
     * @throws ApiError
     */
    static moveS3File(data: MoveS3FileData): CancelablePromise<MoveS3FileResponse>;
    /**
     * Upload file to S3 bucket
     * @param data The data for the request.
     * @param data.workspace
     * @param data.requestBody File content
     * @param data.fileKey
     * @param data.fileExtension
     * @param data.s3ResourcePath
     * @param data.resourceType
     * @returns unknown File upload status
     * @throws ApiError
     */
    static fileUpload(data: FileUploadData): CancelablePromise<FileUploadResponse>;
    /**
     * Download file to S3 bucket
     * @param data The data for the request.
     * @param data.workspace
     * @param data.fileKey
     * @param data.s3ResourcePath
     * @param data.resourceType
     * @returns binary Chunk of the downloaded file
     * @throws ApiError
     */
    static fileDownload(data: FileDownloadData): CancelablePromise<FileDownloadResponse>;
}
export declare class MetricsService {
    /**
     * get job metrics
     * @param data The data for the request.
     * @param data.workspace
     * @param data.id
     * @param data.requestBody parameters for statistics retrieval
     * @returns unknown job details
     * @throws ApiError
     */
    static getJobMetrics(data: GetJobMetricsData): CancelablePromise<GetJobMetricsResponse>;
}
export declare class ConcurrencyGroupsService {
    /**
     * List all concurrency groups
     * @returns ConcurrencyGroup all concurrency groups
     * @throws ApiError
     */
    static listConcurrencyGroups(): CancelablePromise<ListConcurrencyGroupsResponse>;
    /**
     * Delete concurrency group
     * @param data The data for the request.
     * @param data.concurrencyId
     * @returns unknown concurrency group removed
     * @throws ApiError
     */
    static deleteConcurrencyGroup(data: DeleteConcurrencyGroupData): CancelablePromise<DeleteConcurrencyGroupResponse>;
}
