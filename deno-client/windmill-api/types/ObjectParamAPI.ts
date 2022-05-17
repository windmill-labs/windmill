import { ResponseContext, RequestContext, HttpFile } from '../http/http.ts';
import * as models from '../models/all.ts';
import { Configuration} from '../configuration.ts'

import { AuditLog } from '../models/AuditLog.ts';
import { CompletedJob } from '../models/CompletedJob.ts';
import { ContextualVariable } from '../models/ContextualVariable.ts';
import { CreateResource } from '../models/CreateResource.ts';
import { CreateVariable } from '../models/CreateVariable.ts';
import { CreateWorkspace } from '../models/CreateWorkspace.ts';
import { EditResource } from '../models/EditResource.ts';
import { EditResourceType } from '../models/EditResourceType.ts';
import { EditSchedule } from '../models/EditSchedule.ts';
import { EditVariable } from '../models/EditVariable.ts';
import { EditWorkspaceUser } from '../models/EditWorkspaceUser.ts';
import { Flow } from '../models/Flow.ts';
import { FlowModule } from '../models/FlowModule.ts';
import { FlowModuleValue } from '../models/FlowModuleValue.ts';
import { FlowPreview } from '../models/FlowPreview.ts';
import { FlowStatus } from '../models/FlowStatus.ts';
import { FlowStatusModule } from '../models/FlowStatusModule.ts';
import { FlowValue } from '../models/FlowValue.ts';
import { GlobalUserInfo } from '../models/GlobalUserInfo.ts';
import { Group } from '../models/Group.ts';
import { InlineObject } from '../models/InlineObject.ts';
import { InlineObject1 } from '../models/InlineObject1.ts';
import { InlineObject10 } from '../models/InlineObject10.ts';
import { InlineObject11 } from '../models/InlineObject11.ts';
import { InlineObject12 } from '../models/InlineObject12.ts';
import { InlineObject13 } from '../models/InlineObject13.ts';
import { InlineObject14 } from '../models/InlineObject14.ts';
import { InlineObject15 } from '../models/InlineObject15.ts';
import { InlineObject16 } from '../models/InlineObject16.ts';
import { InlineObject17 } from '../models/InlineObject17.ts';
import { InlineObject18 } from '../models/InlineObject18.ts';
import { InlineObject19 } from '../models/InlineObject19.ts';
import { InlineObject2 } from '../models/InlineObject2.ts';
import { InlineObject20 } from '../models/InlineObject20.ts';
import { InlineObject21 } from '../models/InlineObject21.ts';
import { InlineObject3 } from '../models/InlineObject3.ts';
import { InlineObject4 } from '../models/InlineObject4.ts';
import { InlineObject5 } from '../models/InlineObject5.ts';
import { InlineObject6 } from '../models/InlineObject6.ts';
import { InlineObject7 } from '../models/InlineObject7.ts';
import { InlineObject8 } from '../models/InlineObject8.ts';
import { InlineObject9 } from '../models/InlineObject9.ts';
import { InlineResponse200 } from '../models/InlineResponse200.ts';
import { InlineResponse2001 } from '../models/InlineResponse2001.ts';
import { InlineResponse2002 } from '../models/InlineResponse2002.ts';
import { InputTransform } from '../models/InputTransform.ts';
import { Job } from '../models/Job.ts';
import { JobAllOf } from '../models/JobAllOf.ts';
import { ListableVariable } from '../models/ListableVariable.ts';
import { Login } from '../models/Login.ts';
import { MainArgSignature } from '../models/MainArgSignature.ts';
import { MainArgSignatureArgs } from '../models/MainArgSignatureArgs.ts';
import { NewSchedule } from '../models/NewSchedule.ts';
import { NewToken } from '../models/NewToken.ts';
import { NewUser } from '../models/NewUser.ts';
import { Preview } from '../models/Preview.ts';
import { QueuedJob } from '../models/QueuedJob.ts';
import { Resource } from '../models/Resource.ts';
import { ResourceType } from '../models/ResourceType.ts';
import { Schedule } from '../models/Schedule.ts';
import { Script } from '../models/Script.ts';
import { TruncatedToken } from '../models/TruncatedToken.ts';
import { User } from '../models/User.ts';
import { UserWorkspaceList } from '../models/UserWorkspaceList.ts';
import { UserWorkspaceListWorkspaces } from '../models/UserWorkspaceListWorkspaces.ts';
import { WorkerPing } from '../models/WorkerPing.ts';
import { Workspace } from '../models/Workspace.ts';
import { WorkspaceInvite } from '../models/WorkspaceInvite.ts';

import { ObservableAdminApi } from "./ObservableAPI.ts";
import { AdminApiRequestFactory, AdminApiResponseProcessor} from "../apis/AdminApi.ts";

export interface AdminApiCreateUserRequest {
    /**
     * 
     * @type string
     * @memberof AdminApicreateUser
     */
    workspace: string
    /**
     * new user
     * @type NewUser
     * @memberof AdminApicreateUser
     */
    newUser: NewUser
}

export interface AdminApiDeleteUserRequest {
    /**
     * 
     * @type string
     * @memberof AdminApideleteUser
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof AdminApideleteUser
     */
    username: string
}

export interface AdminApiUpdateUserRequest {
    /**
     * 
     * @type string
     * @memberof AdminApiupdateUser
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof AdminApiupdateUser
     */
    username: string
    /**
     * new user
     * @type EditWorkspaceUser
     * @memberof AdminApiupdateUser
     */
    editWorkspaceUser: EditWorkspaceUser
}

export class ObjectAdminApi {
    private api: ObservableAdminApi

    public constructor(configuration: Configuration, requestFactory?: AdminApiRequestFactory, responseProcessor?: AdminApiResponseProcessor) {
        this.api = new ObservableAdminApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * create user (require admin privilege)
     * @param param the request object
     */
    public createUser(param: AdminApiCreateUserRequest, options?: Configuration): Promise<string> {
        return this.api.createUser(param.workspace, param.newUser,  options).toPromise();
    }

    /**
     * delete user (require admin privilege)
     * @param param the request object
     */
    public deleteUser(param: AdminApiDeleteUserRequest, options?: Configuration): Promise<string> {
        return this.api.deleteUser(param.workspace, param.username,  options).toPromise();
    }

    /**
     * update user (require admin privilege)
     * @param param the request object
     */
    public updateUser(param: AdminApiUpdateUserRequest, options?: Configuration): Promise<string> {
        return this.api.updateUser(param.workspace, param.username, param.editWorkspaceUser,  options).toPromise();
    }

}

import { ObservableAuditApi } from "./ObservableAPI.ts";
import { AuditApiRequestFactory, AuditApiResponseProcessor} from "../apis/AuditApi.ts";

export interface AuditApiGetAuditLogRequest {
    /**
     * 
     * @type string
     * @memberof AuditApigetAuditLog
     */
    workspace: string
    /**
     * 
     * @type number
     * @memberof AuditApigetAuditLog
     */
    id: number
}

export interface AuditApiListAuditLogsRequest {
    /**
     * 
     * @type string
     * @memberof AuditApilistAuditLogs
     */
    workspace: string
    /**
     * which page to return (start at 1, default 1)
     * @type number
     * @memberof AuditApilistAuditLogs
     */
    page?: number
    /**
     * number of items to return for a given page (default 30, max 100)
     * @type number
     * @memberof AuditApilistAuditLogs
     */
    perPage?: number
    /**
     * filter on created before (exclusive) timestamp
     * @type Date
     * @memberof AuditApilistAuditLogs
     */
    before?: Date
    /**
     * filter on created after (exclusive) timestamp
     * @type Date
     * @memberof AuditApilistAuditLogs
     */
    after?: Date
    /**
     * filter on exact username of user
     * @type string
     * @memberof AuditApilistAuditLogs
     */
    username?: string
    /**
     * filter on exact or prefix name of operation
     * @type string
     * @memberof AuditApilistAuditLogs
     */
    operation?: string
    /**
     * filter on exact or prefix name of resource
     * @type string
     * @memberof AuditApilistAuditLogs
     */
    resource?: string
    /**
     * filter on type of operation
     * @type &#39;Create&#39; | &#39;Update&#39; | &#39;Delete&#39; | &#39;Execute&#39;
     * @memberof AuditApilistAuditLogs
     */
    actionKind?: 'Create' | 'Update' | 'Delete' | 'Execute'
}

export class ObjectAuditApi {
    private api: ObservableAuditApi

    public constructor(configuration: Configuration, requestFactory?: AuditApiRequestFactory, responseProcessor?: AuditApiResponseProcessor) {
        this.api = new ObservableAuditApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * get audit log (requires admin privilege)
     * @param param the request object
     */
    public getAuditLog(param: AuditApiGetAuditLogRequest, options?: Configuration): Promise<AuditLog> {
        return this.api.getAuditLog(param.workspace, param.id,  options).toPromise();
    }

    /**
     * list audit logs (requires admin privilege)
     * @param param the request object
     */
    public listAuditLogs(param: AuditApiListAuditLogsRequest, options?: Configuration): Promise<Array<AuditLog>> {
        return this.api.listAuditLogs(param.workspace, param.page, param.perPage, param.before, param.after, param.username, param.operation, param.resource, param.actionKind,  options).toPromise();
    }

}

import { ObservableFlowApi } from "./ObservableAPI.ts";
import { FlowApiRequestFactory, FlowApiResponseProcessor} from "../apis/FlowApi.ts";

export interface FlowApiArchiveFlowByPathRequest {
    /**
     * 
     * @type string
     * @memberof FlowApiarchiveFlowByPath
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof FlowApiarchiveFlowByPath
     */
    path: string
}

export interface FlowApiCreateFlowRequest {
    /**
     * 
     * @type string
     * @memberof FlowApicreateFlow
     */
    workspace: string
    /**
     * 
     * @type InlineObject11
     * @memberof FlowApicreateFlow
     */
    inlineObject11: InlineObject11
}

export interface FlowApiGetFlowByPathRequest {
    /**
     * 
     * @type string
     * @memberof FlowApigetFlowByPath
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof FlowApigetFlowByPath
     */
    path: string
}

export interface FlowApiListFlowsRequest {
    /**
     * 
     * @type string
     * @memberof FlowApilistFlows
     */
    workspace: string
    /**
     * which page to return (start at 1, default 1)
     * @type number
     * @memberof FlowApilistFlows
     */
    page?: number
    /**
     * number of items to return for a given page (default 30, max 100)
     * @type number
     * @memberof FlowApilistFlows
     */
    perPage?: number
    /**
     * order by desc order (default true)
     * @type boolean
     * @memberof FlowApilistFlows
     */
    orderDesc?: boolean
    /**
     * mask to filter exact matching user creator
     * @type string
     * @memberof FlowApilistFlows
     */
    createdBy?: string
    /**
     * mask to filter matching starting parh
     * @type string
     * @memberof FlowApilistFlows
     */
    pathStart?: string
    /**
     * mask to filter exact matching path
     * @type string
     * @memberof FlowApilistFlows
     */
    pathExact?: string
    /**
     * (default false) show also the archived files. when multiple archived hash share the same path, only the ones with the latest create_at are displayed. 
     * @type boolean
     * @memberof FlowApilistFlows
     */
    showArchived?: boolean
}

export interface FlowApiUpdateFlowRequest {
    /**
     * 
     * @type string
     * @memberof FlowApiupdateFlow
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof FlowApiupdateFlow
     */
    path: string
    /**
     * 
     * @type InlineObject12
     * @memberof FlowApiupdateFlow
     */
    inlineObject12: InlineObject12
}

export class ObjectFlowApi {
    private api: ObservableFlowApi

    public constructor(configuration: Configuration, requestFactory?: FlowApiRequestFactory, responseProcessor?: FlowApiResponseProcessor) {
        this.api = new ObservableFlowApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * archive flow by path
     * @param param the request object
     */
    public archiveFlowByPath(param: FlowApiArchiveFlowByPathRequest, options?: Configuration): Promise<string> {
        return this.api.archiveFlowByPath(param.workspace, param.path,  options).toPromise();
    }

    /**
     * create flow
     * @param param the request object
     */
    public createFlow(param: FlowApiCreateFlowRequest, options?: Configuration): Promise<string> {
        return this.api.createFlow(param.workspace, param.inlineObject11,  options).toPromise();
    }

    /**
     * get flow by path
     * @param param the request object
     */
    public getFlowByPath(param: FlowApiGetFlowByPathRequest, options?: Configuration): Promise<Flow> {
        return this.api.getFlowByPath(param.workspace, param.path,  options).toPromise();
    }

    /**
     * list all available flows
     * @param param the request object
     */
    public listFlows(param: FlowApiListFlowsRequest, options?: Configuration): Promise<Array<Flow>> {
        return this.api.listFlows(param.workspace, param.page, param.perPage, param.orderDesc, param.createdBy, param.pathStart, param.pathExact, param.showArchived,  options).toPromise();
    }

    /**
     * update flow
     * @param param the request object
     */
    public updateFlow(param: FlowApiUpdateFlowRequest, options?: Configuration): Promise<string> {
        return this.api.updateFlow(param.workspace, param.path, param.inlineObject12,  options).toPromise();
    }

}

import { ObservableGranularAclApi } from "./ObservableAPI.ts";
import { GranularAclApiRequestFactory, GranularAclApiResponseProcessor} from "../apis/GranularAclApi.ts";

export interface GranularAclApiAddGranularAclsRequest {
    /**
     * 
     * @type string
     * @memberof GranularAclApiaddGranularAcls
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof GranularAclApiaddGranularAcls
     */
    path: string
    /**
     * 
     * @type &#39;script&#39; | &#39;group_&#39; | &#39;resource&#39; | &#39;schedule&#39; | &#39;variable&#39; | &#39;flow&#39;
     * @memberof GranularAclApiaddGranularAcls
     */
    kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow'
    /**
     * 
     * @type InlineObject20
     * @memberof GranularAclApiaddGranularAcls
     */
    inlineObject20: InlineObject20
}

export interface GranularAclApiGetGranularAclsRequest {
    /**
     * 
     * @type string
     * @memberof GranularAclApigetGranularAcls
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof GranularAclApigetGranularAcls
     */
    path: string
    /**
     * 
     * @type &#39;script&#39; | &#39;group_&#39; | &#39;resource&#39; | &#39;schedule&#39; | &#39;variable&#39; | &#39;flow&#39;
     * @memberof GranularAclApigetGranularAcls
     */
    kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow'
}

export interface GranularAclApiRemoveGranularAclsRequest {
    /**
     * 
     * @type string
     * @memberof GranularAclApiremoveGranularAcls
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof GranularAclApiremoveGranularAcls
     */
    path: string
    /**
     * 
     * @type &#39;script&#39; | &#39;group_&#39; | &#39;resource&#39; | &#39;schedule&#39; | &#39;variable&#39; | &#39;flow&#39;
     * @memberof GranularAclApiremoveGranularAcls
     */
    kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow'
    /**
     * 
     * @type InlineObject21
     * @memberof GranularAclApiremoveGranularAcls
     */
    inlineObject21: InlineObject21
}

export class ObjectGranularAclApi {
    private api: ObservableGranularAclApi

    public constructor(configuration: Configuration, requestFactory?: GranularAclApiRequestFactory, responseProcessor?: GranularAclApiResponseProcessor) {
        this.api = new ObservableGranularAclApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * add granular acls
     * @param param the request object
     */
    public addGranularAcls(param: GranularAclApiAddGranularAclsRequest, options?: Configuration): Promise<string> {
        return this.api.addGranularAcls(param.workspace, param.path, param.kind, param.inlineObject20,  options).toPromise();
    }

    /**
     * get granular acls
     * @param param the request object
     */
    public getGranularAcls(param: GranularAclApiGetGranularAclsRequest, options?: Configuration): Promise<{ [key: string]: boolean; }> {
        return this.api.getGranularAcls(param.workspace, param.path, param.kind,  options).toPromise();
    }

    /**
     * remove granular acls
     * @param param the request object
     */
    public removeGranularAcls(param: GranularAclApiRemoveGranularAclsRequest, options?: Configuration): Promise<string> {
        return this.api.removeGranularAcls(param.workspace, param.path, param.kind, param.inlineObject21,  options).toPromise();
    }

}

import { ObservableGroupApi } from "./ObservableAPI.ts";
import { GroupApiRequestFactory, GroupApiResponseProcessor} from "../apis/GroupApi.ts";

export interface GroupApiAddUserToGroupRequest {
    /**
     * 
     * @type string
     * @memberof GroupApiaddUserToGroup
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof GroupApiaddUserToGroup
     */
    name: string
    /**
     * 
     * @type InlineObject18
     * @memberof GroupApiaddUserToGroup
     */
    inlineObject18: InlineObject18
}

export interface GroupApiCreateGroupRequest {
    /**
     * 
     * @type string
     * @memberof GroupApicreateGroup
     */
    workspace: string
    /**
     * 
     * @type InlineObject16
     * @memberof GroupApicreateGroup
     */
    inlineObject16: InlineObject16
}

export interface GroupApiDeleteGroupRequest {
    /**
     * 
     * @type string
     * @memberof GroupApideleteGroup
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof GroupApideleteGroup
     */
    name: string
}

export interface GroupApiGetGroupRequest {
    /**
     * 
     * @type string
     * @memberof GroupApigetGroup
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof GroupApigetGroup
     */
    name: string
}

export interface GroupApiListGroupNamesRequest {
    /**
     * 
     * @type string
     * @memberof GroupApilistGroupNames
     */
    workspace: string
}

export interface GroupApiListGroupsRequest {
    /**
     * 
     * @type string
     * @memberof GroupApilistGroups
     */
    workspace: string
    /**
     * which page to return (start at 1, default 1)
     * @type number
     * @memberof GroupApilistGroups
     */
    page?: number
    /**
     * number of items to return for a given page (default 30, max 100)
     * @type number
     * @memberof GroupApilistGroups
     */
    perPage?: number
}

export interface GroupApiRemoveUserToGroupRequest {
    /**
     * 
     * @type string
     * @memberof GroupApiremoveUserToGroup
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof GroupApiremoveUserToGroup
     */
    name: string
    /**
     * 
     * @type InlineObject19
     * @memberof GroupApiremoveUserToGroup
     */
    inlineObject19: InlineObject19
}

export interface GroupApiUpdateGroupRequest {
    /**
     * 
     * @type string
     * @memberof GroupApiupdateGroup
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof GroupApiupdateGroup
     */
    name: string
    /**
     * 
     * @type InlineObject17
     * @memberof GroupApiupdateGroup
     */
    inlineObject17: InlineObject17
}

export class ObjectGroupApi {
    private api: ObservableGroupApi

    public constructor(configuration: Configuration, requestFactory?: GroupApiRequestFactory, responseProcessor?: GroupApiResponseProcessor) {
        this.api = new ObservableGroupApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * add user to group
     * @param param the request object
     */
    public addUserToGroup(param: GroupApiAddUserToGroupRequest, options?: Configuration): Promise<string> {
        return this.api.addUserToGroup(param.workspace, param.name, param.inlineObject18,  options).toPromise();
    }

    /**
     * create group
     * @param param the request object
     */
    public createGroup(param: GroupApiCreateGroupRequest, options?: Configuration): Promise<string> {
        return this.api.createGroup(param.workspace, param.inlineObject16,  options).toPromise();
    }

    /**
     * delete group
     * @param param the request object
     */
    public deleteGroup(param: GroupApiDeleteGroupRequest, options?: Configuration): Promise<string> {
        return this.api.deleteGroup(param.workspace, param.name,  options).toPromise();
    }

    /**
     * get group
     * @param param the request object
     */
    public getGroup(param: GroupApiGetGroupRequest, options?: Configuration): Promise<Group> {
        return this.api.getGroup(param.workspace, param.name,  options).toPromise();
    }

    /**
     * list group names
     * @param param the request object
     */
    public listGroupNames(param: GroupApiListGroupNamesRequest, options?: Configuration): Promise<Array<string>> {
        return this.api.listGroupNames(param.workspace,  options).toPromise();
    }

    /**
     * list groups
     * @param param the request object
     */
    public listGroups(param: GroupApiListGroupsRequest, options?: Configuration): Promise<Array<Group>> {
        return this.api.listGroups(param.workspace, param.page, param.perPage,  options).toPromise();
    }

    /**
     * remove user to group
     * @param param the request object
     */
    public removeUserToGroup(param: GroupApiRemoveUserToGroupRequest, options?: Configuration): Promise<string> {
        return this.api.removeUserToGroup(param.workspace, param.name, param.inlineObject19,  options).toPromise();
    }

    /**
     * update group
     * @param param the request object
     */
    public updateGroup(param: GroupApiUpdateGroupRequest, options?: Configuration): Promise<string> {
        return this.api.updateGroup(param.workspace, param.name, param.inlineObject17,  options).toPromise();
    }

}

import { ObservableJobApi } from "./ObservableAPI.ts";
import { JobApiRequestFactory, JobApiResponseProcessor} from "../apis/JobApi.ts";

export interface JobApiCancelQueuedJobRequest {
    /**
     * 
     * @type string
     * @memberof JobApicancelQueuedJob
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof JobApicancelQueuedJob
     */
    id: string
    /**
     * 
     * @type InlineObject13
     * @memberof JobApicancelQueuedJob
     */
    inlineObject13: InlineObject13
}

export interface JobApiDeleteCompletedJobRequest {
    /**
     * 
     * @type string
     * @memberof JobApideleteCompletedJob
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof JobApideleteCompletedJob
     */
    id: string
}

export interface JobApiGetCompletedJobRequest {
    /**
     * 
     * @type string
     * @memberof JobApigetCompletedJob
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof JobApigetCompletedJob
     */
    id: string
}

export interface JobApiGetJobRequest {
    /**
     * 
     * @type string
     * @memberof JobApigetJob
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof JobApigetJob
     */
    id: string
}

export interface JobApiGetJobUpdatesRequest {
    /**
     * 
     * @type string
     * @memberof JobApigetJobUpdates
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof JobApigetJobUpdates
     */
    id: string
    /**
     * 
     * @type boolean
     * @memberof JobApigetJobUpdates
     */
    running?: boolean
    /**
     * 
     * @type number
     * @memberof JobApigetJobUpdates
     */
    logOffset?: number
}

export interface JobApiListCompletedJobsRequest {
    /**
     * 
     * @type string
     * @memberof JobApilistCompletedJobs
     */
    workspace: string
    /**
     * order by desc order (default true)
     * @type boolean
     * @memberof JobApilistCompletedJobs
     */
    orderDesc?: boolean
    /**
     * mask to filter exact matching user creator
     * @type string
     * @memberof JobApilistCompletedJobs
     */
    createdBy?: string
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     * @type string
     * @memberof JobApilistCompletedJobs
     */
    parentJob?: string
    /**
     * mask to filter exact matching path
     * @type string
     * @memberof JobApilistCompletedJobs
     */
    scriptPathExact?: string
    /**
     * mask to filter matching starting path
     * @type string
     * @memberof JobApilistCompletedJobs
     */
    scriptPathStart?: string
    /**
     * mask to filter exact matching path
     * @type string
     * @memberof JobApilistCompletedJobs
     */
    scriptHash?: string
    /**
     * filter on created before (inclusive) timestamp
     * @type Date
     * @memberof JobApilistCompletedJobs
     */
    createdBefore?: Date
    /**
     * filter on created after (exclusive) timestamp
     * @type Date
     * @memberof JobApilistCompletedJobs
     */
    createdAfter?: Date
    /**
     * filter on successful jobs
     * @type boolean
     * @memberof JobApilistCompletedJobs
     */
    success?: boolean
    /**
     * filter on job kind (values &#39;preview&#39;, &#39;script&#39;, &#39;dependencies&#39;, &#39;flow&#39;) separated by,
     * @type string
     * @memberof JobApilistCompletedJobs
     */
    jobKinds?: string
}

export interface JobApiListJobsRequest {
    /**
     * 
     * @type string
     * @memberof JobApilistJobs
     */
    workspace: string
    /**
     * mask to filter exact matching user creator
     * @type string
     * @memberof JobApilistJobs
     */
    createdBy?: string
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     * @type string
     * @memberof JobApilistJobs
     */
    parentJob?: string
    /**
     * mask to filter exact matching path
     * @type string
     * @memberof JobApilistJobs
     */
    scriptPathExact?: string
    /**
     * mask to filter matching starting path
     * @type string
     * @memberof JobApilistJobs
     */
    scriptPathStart?: string
    /**
     * mask to filter exact matching path
     * @type string
     * @memberof JobApilistJobs
     */
    scriptHash?: string
    /**
     * filter on created before (inclusive) timestamp
     * @type Date
     * @memberof JobApilistJobs
     */
    createdBefore?: Date
    /**
     * filter on created after (exclusive) timestamp
     * @type Date
     * @memberof JobApilistJobs
     */
    createdAfter?: Date
    /**
     * filter on job kind (values &#39;preview&#39;, &#39;script&#39;, &#39;dependencies&#39;, &#39;flow&#39;) separated by,
     * @type string
     * @memberof JobApilistJobs
     */
    jobKinds?: string
    /**
     * filter on successful jobs
     * @type boolean
     * @memberof JobApilistJobs
     */
    success?: boolean
}

export interface JobApiListQueueRequest {
    /**
     * 
     * @type string
     * @memberof JobApilistQueue
     */
    workspace: string
    /**
     * order by desc order (default true)
     * @type boolean
     * @memberof JobApilistQueue
     */
    orderDesc?: boolean
    /**
     * mask to filter exact matching user creator
     * @type string
     * @memberof JobApilistQueue
     */
    createdBy?: string
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     * @type string
     * @memberof JobApilistQueue
     */
    parentJob?: string
    /**
     * mask to filter exact matching path
     * @type string
     * @memberof JobApilistQueue
     */
    scriptPathExact?: string
    /**
     * mask to filter matching starting path
     * @type string
     * @memberof JobApilistQueue
     */
    scriptPathStart?: string
    /**
     * mask to filter exact matching path
     * @type string
     * @memberof JobApilistQueue
     */
    scriptHash?: string
    /**
     * filter on created before (inclusive) timestamp
     * @type Date
     * @memberof JobApilistQueue
     */
    createdBefore?: Date
    /**
     * filter on created after (exclusive) timestamp
     * @type Date
     * @memberof JobApilistQueue
     */
    createdAfter?: Date
    /**
     * filter on successful jobs
     * @type boolean
     * @memberof JobApilistQueue
     */
    success?: boolean
    /**
     * filter on job kind (values &#39;preview&#39;, &#39;script&#39;, &#39;dependencies&#39;, &#39;flow&#39;) separated by,
     * @type string
     * @memberof JobApilistQueue
     */
    jobKinds?: string
}

export interface JobApiRunFlowByPathRequest {
    /**
     * 
     * @type string
     * @memberof JobApirunFlowByPath
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof JobApirunFlowByPath
     */
    path: string
    /**
     * flow args
     * @type { [key: string]: any; }
     * @memberof JobApirunFlowByPath
     */
    requestBody: { [key: string]: any; }
    /**
     * when to schedule this job (leave empty for immediate run)
     * @type Date
     * @memberof JobApirunFlowByPath
     */
    scheduledFor?: Date
    /**
     * schedule the script to execute in the number of seconds starting now
     * @type number
     * @memberof JobApirunFlowByPath
     */
    scheduledInSecs?: number
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     * @type string
     * @memberof JobApirunFlowByPath
     */
    parentJob?: string
}

export interface JobApiRunFlowPreviewRequest {
    /**
     * 
     * @type string
     * @memberof JobApirunFlowPreview
     */
    workspace: string
    /**
     * preview
     * @type FlowPreview
     * @memberof JobApirunFlowPreview
     */
    flowPreview: FlowPreview
}

export interface JobApiRunScriptByHashRequest {
    /**
     * 
     * @type string
     * @memberof JobApirunScriptByHash
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof JobApirunScriptByHash
     */
    hash: string
    /**
     * Partially filled args
     * @type any
     * @memberof JobApirunScriptByHash
     */
    body: any
    /**
     * when to schedule this job (leave empty for immediate run)
     * @type Date
     * @memberof JobApirunScriptByHash
     */
    scheduledFor?: Date
    /**
     * schedule the script to execute in the number of seconds starting now
     * @type number
     * @memberof JobApirunScriptByHash
     */
    scheduledInSecs?: number
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     * @type string
     * @memberof JobApirunScriptByHash
     */
    parentJob?: string
}

export interface JobApiRunScriptByPathRequest {
    /**
     * 
     * @type string
     * @memberof JobApirunScriptByPath
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof JobApirunScriptByPath
     */
    path: string
    /**
     * script args
     * @type { [key: string]: any; }
     * @memberof JobApirunScriptByPath
     */
    requestBody: { [key: string]: any; }
    /**
     * when to schedule this job (leave empty for immediate run)
     * @type Date
     * @memberof JobApirunScriptByPath
     */
    scheduledFor?: Date
    /**
     * schedule the script to execute in the number of seconds starting now
     * @type number
     * @memberof JobApirunScriptByPath
     */
    scheduledInSecs?: number
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     * @type string
     * @memberof JobApirunScriptByPath
     */
    parentJob?: string
}

export interface JobApiRunScriptPreviewRequest {
    /**
     * 
     * @type string
     * @memberof JobApirunScriptPreview
     */
    workspace: string
    /**
     * previw
     * @type Preview
     * @memberof JobApirunScriptPreview
     */
    preview: Preview
}

export class ObjectJobApi {
    private api: ObservableJobApi

    public constructor(configuration: Configuration, requestFactory?: JobApiRequestFactory, responseProcessor?: JobApiResponseProcessor) {
        this.api = new ObservableJobApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * cancel queued job
     * @param param the request object
     */
    public cancelQueuedJob(param: JobApiCancelQueuedJobRequest, options?: Configuration): Promise<string> {
        return this.api.cancelQueuedJob(param.workspace, param.id, param.inlineObject13,  options).toPromise();
    }

    /**
     * delete completed job (erase content but keep run id)
     * @param param the request object
     */
    public deleteCompletedJob(param: JobApiDeleteCompletedJobRequest, options?: Configuration): Promise<CompletedJob> {
        return this.api.deleteCompletedJob(param.workspace, param.id,  options).toPromise();
    }

    /**
     * get completed job
     * @param param the request object
     */
    public getCompletedJob(param: JobApiGetCompletedJobRequest, options?: Configuration): Promise<CompletedJob> {
        return this.api.getCompletedJob(param.workspace, param.id,  options).toPromise();
    }

    /**
     * get job
     * @param param the request object
     */
    public getJob(param: JobApiGetJobRequest, options?: Configuration): Promise<Job> {
        return this.api.getJob(param.workspace, param.id,  options).toPromise();
    }

    /**
     * get job updates
     * @param param the request object
     */
    public getJobUpdates(param: JobApiGetJobUpdatesRequest, options?: Configuration): Promise<InlineResponse2002> {
        return this.api.getJobUpdates(param.workspace, param.id, param.running, param.logOffset,  options).toPromise();
    }

    /**
     * list all available completed jobs
     * @param param the request object
     */
    public listCompletedJobs(param: JobApiListCompletedJobsRequest, options?: Configuration): Promise<Array<CompletedJob>> {
        return this.api.listCompletedJobs(param.workspace, param.orderDesc, param.createdBy, param.parentJob, param.scriptPathExact, param.scriptPathStart, param.scriptHash, param.createdBefore, param.createdAfter, param.success, param.jobKinds,  options).toPromise();
    }

    /**
     * list all available jobs
     * @param param the request object
     */
    public listJobs(param: JobApiListJobsRequest, options?: Configuration): Promise<Array<Job>> {
        return this.api.listJobs(param.workspace, param.createdBy, param.parentJob, param.scriptPathExact, param.scriptPathStart, param.scriptHash, param.createdBefore, param.createdAfter, param.jobKinds, param.success,  options).toPromise();
    }

    /**
     * list all available queued jobs
     * @param param the request object
     */
    public listQueue(param: JobApiListQueueRequest, options?: Configuration): Promise<Array<QueuedJob>> {
        return this.api.listQueue(param.workspace, param.orderDesc, param.createdBy, param.parentJob, param.scriptPathExact, param.scriptPathStart, param.scriptHash, param.createdBefore, param.createdAfter, param.success, param.jobKinds,  options).toPromise();
    }

    /**
     * run flow by path
     * @param param the request object
     */
    public runFlowByPath(param: JobApiRunFlowByPathRequest, options?: Configuration): Promise<string> {
        return this.api.runFlowByPath(param.workspace, param.path, param.requestBody, param.scheduledFor, param.scheduledInSecs, param.parentJob,  options).toPromise();
    }

    /**
     * run flow preview
     * @param param the request object
     */
    public runFlowPreview(param: JobApiRunFlowPreviewRequest, options?: Configuration): Promise<string> {
        return this.api.runFlowPreview(param.workspace, param.flowPreview,  options).toPromise();
    }

    /**
     * run script by hash
     * @param param the request object
     */
    public runScriptByHash(param: JobApiRunScriptByHashRequest, options?: Configuration): Promise<string> {
        return this.api.runScriptByHash(param.workspace, param.hash, param.body, param.scheduledFor, param.scheduledInSecs, param.parentJob,  options).toPromise();
    }

    /**
     * run script by path
     * @param param the request object
     */
    public runScriptByPath(param: JobApiRunScriptByPathRequest, options?: Configuration): Promise<string> {
        return this.api.runScriptByPath(param.workspace, param.path, param.requestBody, param.scheduledFor, param.scheduledInSecs, param.parentJob,  options).toPromise();
    }

    /**
     * run script preview
     * @param param the request object
     */
    public runScriptPreview(param: JobApiRunScriptPreviewRequest, options?: Configuration): Promise<string> {
        return this.api.runScriptPreview(param.workspace, param.preview,  options).toPromise();
    }

}

import { ObservableResourceApi } from "./ObservableAPI.ts";
import { ResourceApiRequestFactory, ResourceApiResponseProcessor} from "../apis/ResourceApi.ts";

export interface ResourceApiCreateResourceRequest {
    /**
     * 
     * @type string
     * @memberof ResourceApicreateResource
     */
    workspace: string
    /**
     * new resource
     * @type CreateResource
     * @memberof ResourceApicreateResource
     */
    createResource: CreateResource
}

export interface ResourceApiCreateResourceTypeRequest {
    /**
     * 
     * @type string
     * @memberof ResourceApicreateResourceType
     */
    workspace: string
    /**
     * new resource_type
     * @type ResourceType
     * @memberof ResourceApicreateResourceType
     */
    resourceType: ResourceType
}

export interface ResourceApiDeleteResourceRequest {
    /**
     * 
     * @type string
     * @memberof ResourceApideleteResource
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ResourceApideleteResource
     */
    path: string
}

export interface ResourceApiDeleteResourceTypeRequest {
    /**
     * 
     * @type string
     * @memberof ResourceApideleteResourceType
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ResourceApideleteResourceType
     */
    path: string
}

export interface ResourceApiGetResourceRequest {
    /**
     * 
     * @type string
     * @memberof ResourceApigetResource
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ResourceApigetResource
     */
    path: string
}

export interface ResourceApiGetResourceTypeRequest {
    /**
     * 
     * @type string
     * @memberof ResourceApigetResourceType
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ResourceApigetResourceType
     */
    path: string
}

export interface ResourceApiListResourceRequest {
    /**
     * 
     * @type string
     * @memberof ResourceApilistResource
     */
    workspace: string
    /**
     * which page to return (start at 1, default 1)
     * @type number
     * @memberof ResourceApilistResource
     */
    page?: number
    /**
     * number of items to return for a given page (default 30, max 100)
     * @type number
     * @memberof ResourceApilistResource
     */
    perPage?: number
    /**
     * resource_type to list from
     * @type string
     * @memberof ResourceApilistResource
     */
    resourceType?: string
}

export interface ResourceApiListResourceTypeRequest {
    /**
     * 
     * @type string
     * @memberof ResourceApilistResourceType
     */
    workspace: string
}

export interface ResourceApiListResourceTypeNamesRequest {
    /**
     * 
     * @type string
     * @memberof ResourceApilistResourceTypeNames
     */
    workspace: string
}

export interface ResourceApiUpdateResourceRequest {
    /**
     * 
     * @type string
     * @memberof ResourceApiupdateResource
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ResourceApiupdateResource
     */
    path: string
    /**
     * updated resource
     * @type EditResource
     * @memberof ResourceApiupdateResource
     */
    editResource: EditResource
}

export interface ResourceApiUpdateResourceTypeRequest {
    /**
     * 
     * @type string
     * @memberof ResourceApiupdateResourceType
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ResourceApiupdateResourceType
     */
    path: string
    /**
     * updated resource_type
     * @type EditResourceType
     * @memberof ResourceApiupdateResourceType
     */
    editResourceType: EditResourceType
}

export class ObjectResourceApi {
    private api: ObservableResourceApi

    public constructor(configuration: Configuration, requestFactory?: ResourceApiRequestFactory, responseProcessor?: ResourceApiResponseProcessor) {
        this.api = new ObservableResourceApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * create resource
     * @param param the request object
     */
    public createResource(param: ResourceApiCreateResourceRequest, options?: Configuration): Promise<string> {
        return this.api.createResource(param.workspace, param.createResource,  options).toPromise();
    }

    /**
     * create resource_type
     * @param param the request object
     */
    public createResourceType(param: ResourceApiCreateResourceTypeRequest, options?: Configuration): Promise<string> {
        return this.api.createResourceType(param.workspace, param.resourceType,  options).toPromise();
    }

    /**
     * delete resource
     * @param param the request object
     */
    public deleteResource(param: ResourceApiDeleteResourceRequest, options?: Configuration): Promise<string> {
        return this.api.deleteResource(param.workspace, param.path,  options).toPromise();
    }

    /**
     * delete resource_type
     * @param param the request object
     */
    public deleteResourceType(param: ResourceApiDeleteResourceTypeRequest, options?: Configuration): Promise<string> {
        return this.api.deleteResourceType(param.workspace, param.path,  options).toPromise();
    }

    /**
     * get resource
     * @param param the request object
     */
    public getResource(param: ResourceApiGetResourceRequest, options?: Configuration): Promise<Resource> {
        return this.api.getResource(param.workspace, param.path,  options).toPromise();
    }

    /**
     * get resource_type
     * @param param the request object
     */
    public getResourceType(param: ResourceApiGetResourceTypeRequest, options?: Configuration): Promise<ResourceType> {
        return this.api.getResourceType(param.workspace, param.path,  options).toPromise();
    }

    /**
     * list resources
     * @param param the request object
     */
    public listResource(param: ResourceApiListResourceRequest, options?: Configuration): Promise<Array<Resource>> {
        return this.api.listResource(param.workspace, param.page, param.perPage, param.resourceType,  options).toPromise();
    }

    /**
     * list resource_types
     * @param param the request object
     */
    public listResourceType(param: ResourceApiListResourceTypeRequest, options?: Configuration): Promise<Array<ResourceType>> {
        return this.api.listResourceType(param.workspace,  options).toPromise();
    }

    /**
     * list resource_types names
     * @param param the request object
     */
    public listResourceTypeNames(param: ResourceApiListResourceTypeNamesRequest, options?: Configuration): Promise<Array<string>> {
        return this.api.listResourceTypeNames(param.workspace,  options).toPromise();
    }

    /**
     * update resource
     * @param param the request object
     */
    public updateResource(param: ResourceApiUpdateResourceRequest, options?: Configuration): Promise<string> {
        return this.api.updateResource(param.workspace, param.path, param.editResource,  options).toPromise();
    }

    /**
     * update resource_type
     * @param param the request object
     */
    public updateResourceType(param: ResourceApiUpdateResourceTypeRequest, options?: Configuration): Promise<string> {
        return this.api.updateResourceType(param.workspace, param.path, param.editResourceType,  options).toPromise();
    }

}

import { ObservableScheduleApi } from "./ObservableAPI.ts";
import { ScheduleApiRequestFactory, ScheduleApiResponseProcessor} from "../apis/ScheduleApi.ts";

export interface ScheduleApiCreateScheduleRequest {
    /**
     * 
     * @type string
     * @memberof ScheduleApicreateSchedule
     */
    workspace: string
    /**
     * new schedule
     * @type NewSchedule
     * @memberof ScheduleApicreateSchedule
     */
    newSchedule: NewSchedule
}

export interface ScheduleApiGetScheduleRequest {
    /**
     * 
     * @type string
     * @memberof ScheduleApigetSchedule
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ScheduleApigetSchedule
     */
    path: string
}

export interface ScheduleApiListSchedulesRequest {
    /**
     * 
     * @type string
     * @memberof ScheduleApilistSchedules
     */
    workspace: string
    /**
     * which page to return (start at 1, default 1)
     * @type number
     * @memberof ScheduleApilistSchedules
     */
    page?: number
    /**
     * number of items to return for a given page (default 30, max 100)
     * @type number
     * @memberof ScheduleApilistSchedules
     */
    perPage?: number
}

export interface ScheduleApiPreviewScheduleRequest {
    /**
     * 
     * @type InlineObject14
     * @memberof ScheduleApipreviewSchedule
     */
    inlineObject14: InlineObject14
}

export interface ScheduleApiSetScheduleEnabledRequest {
    /**
     * 
     * @type string
     * @memberof ScheduleApisetScheduleEnabled
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ScheduleApisetScheduleEnabled
     */
    path: string
    /**
     * 
     * @type InlineObject15
     * @memberof ScheduleApisetScheduleEnabled
     */
    inlineObject15: InlineObject15
}

export interface ScheduleApiUpdateScheduleRequest {
    /**
     * 
     * @type string
     * @memberof ScheduleApiupdateSchedule
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ScheduleApiupdateSchedule
     */
    path: string
    /**
     * updated schedule
     * @type EditSchedule
     * @memberof ScheduleApiupdateSchedule
     */
    editSchedule: EditSchedule
}

export class ObjectScheduleApi {
    private api: ObservableScheduleApi

    public constructor(configuration: Configuration, requestFactory?: ScheduleApiRequestFactory, responseProcessor?: ScheduleApiResponseProcessor) {
        this.api = new ObservableScheduleApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * create schedule
     * @param param the request object
     */
    public createSchedule(param: ScheduleApiCreateScheduleRequest, options?: Configuration): Promise<string> {
        return this.api.createSchedule(param.workspace, param.newSchedule,  options).toPromise();
    }

    /**
     * get schedule
     * @param param the request object
     */
    public getSchedule(param: ScheduleApiGetScheduleRequest, options?: Configuration): Promise<Schedule> {
        return this.api.getSchedule(param.workspace, param.path,  options).toPromise();
    }

    /**
     * list schedules
     * @param param the request object
     */
    public listSchedules(param: ScheduleApiListSchedulesRequest, options?: Configuration): Promise<Array<Schedule>> {
        return this.api.listSchedules(param.workspace, param.page, param.perPage,  options).toPromise();
    }

    /**
     * preview schedule
     * @param param the request object
     */
    public previewSchedule(param: ScheduleApiPreviewScheduleRequest, options?: Configuration): Promise<Array<Date>> {
        return this.api.previewSchedule(param.inlineObject14,  options).toPromise();
    }

    /**
     * set enabled schedule
     * @param param the request object
     */
    public setScheduleEnabled(param: ScheduleApiSetScheduleEnabledRequest, options?: Configuration): Promise<string> {
        return this.api.setScheduleEnabled(param.workspace, param.path, param.inlineObject15,  options).toPromise();
    }

    /**
     * update schedule
     * @param param the request object
     */
    public updateSchedule(param: ScheduleApiUpdateScheduleRequest, options?: Configuration): Promise<string> {
        return this.api.updateSchedule(param.workspace, param.path, param.editSchedule,  options).toPromise();
    }

}

import { ObservableScriptApi } from "./ObservableAPI.ts";
import { ScriptApiRequestFactory, ScriptApiResponseProcessor} from "../apis/ScriptApi.ts";

export interface ScriptApiArchiveScriptByHashRequest {
    /**
     * 
     * @type string
     * @memberof ScriptApiarchiveScriptByHash
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ScriptApiarchiveScriptByHash
     */
    hash: string
}

export interface ScriptApiArchiveScriptByPathRequest {
    /**
     * 
     * @type string
     * @memberof ScriptApiarchiveScriptByPath
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ScriptApiarchiveScriptByPath
     */
    path: string
}

export interface ScriptApiCreateScriptRequest {
    /**
     * 
     * @type string
     * @memberof ScriptApicreateScript
     */
    workspace: string
    /**
     * 
     * @type InlineObject10
     * @memberof ScriptApicreateScript
     */
    inlineObject10: InlineObject10
}

export interface ScriptApiDeleteScriptByHashRequest {
    /**
     * 
     * @type string
     * @memberof ScriptApideleteScriptByHash
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ScriptApideleteScriptByHash
     */
    hash: string
}

export interface ScriptApiDenoToJsonschemaRequest {
    /**
     * deno code with the main function
     * @type string
     * @memberof ScriptApidenoToJsonschema
     */
    body: string
}

export interface ScriptApiGetScriptByHashRequest {
    /**
     * 
     * @type string
     * @memberof ScriptApigetScriptByHash
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ScriptApigetScriptByHash
     */
    hash: string
}

export interface ScriptApiGetScriptByPathRequest {
    /**
     * 
     * @type string
     * @memberof ScriptApigetScriptByPath
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ScriptApigetScriptByPath
     */
    path: string
}

export interface ScriptApiGetScriptDeploymentStatusRequest {
    /**
     * 
     * @type string
     * @memberof ScriptApigetScriptDeploymentStatus
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof ScriptApigetScriptDeploymentStatus
     */
    hash: string
}

export interface ScriptApiListScriptsRequest {
    /**
     * 
     * @type string
     * @memberof ScriptApilistScripts
     */
    workspace: string
    /**
     * which page to return (start at 1, default 1)
     * @type number
     * @memberof ScriptApilistScripts
     */
    page?: number
    /**
     * number of items to return for a given page (default 30, max 100)
     * @type number
     * @memberof ScriptApilistScripts
     */
    perPage?: number
    /**
     * order by desc order (default true)
     * @type boolean
     * @memberof ScriptApilistScripts
     */
    orderDesc?: boolean
    /**
     * mask to filter exact matching user creator
     * @type string
     * @memberof ScriptApilistScripts
     */
    createdBy?: string
    /**
     * mask to filter matching starting parh
     * @type string
     * @memberof ScriptApilistScripts
     */
    pathStart?: string
    /**
     * mask to filter exact matching path
     * @type string
     * @memberof ScriptApilistScripts
     */
    pathExact?: string
    /**
     * mask to filter scripts whom first direct parent has exact hash
     * @type string
     * @memberof ScriptApilistScripts
     */
    firstParentHash?: string
    /**
     * mask to filter scripts whom last parent in the chain has exact hash.  Beware that each script stores only a limited number of parents. Hence the last parent hash for a script is not necessarily its top-most parent. To find the top-most parent you will have to jump from last to last hash  until finding the parent 
     * @type string
     * @memberof ScriptApilistScripts
     */
    lastParentHash?: string
    /**
     * is the hash present in the array of stored parent hashes for this script. The same warning applies than for last_parent_hash. A script only store a limited number of direct parent 
     * @type string
     * @memberof ScriptApilistScripts
     */
    parentHash?: string
    /**
     * (default false) show also the archived files. when multiple archived hash share the same path, only the ones with the latest create_at are displayed. 
     * @type boolean
     * @memberof ScriptApilistScripts
     */
    showArchived?: boolean
    /**
     * (default regardless) if true show only the templates if false show only the non templates if not defined, show all regardless of if the script is a template 
     * @type boolean
     * @memberof ScriptApilistScripts
     */
    isTemplate?: boolean
}

export interface ScriptApiPythonToJsonschemaRequest {
    /**
     * python code with the main function
     * @type string
     * @memberof ScriptApipythonToJsonschema
     */
    body: string
}

export class ObjectScriptApi {
    private api: ObservableScriptApi

    public constructor(configuration: Configuration, requestFactory?: ScriptApiRequestFactory, responseProcessor?: ScriptApiResponseProcessor) {
        this.api = new ObservableScriptApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * archive script by hash
     * @param param the request object
     */
    public archiveScriptByHash(param: ScriptApiArchiveScriptByHashRequest, options?: Configuration): Promise<Script> {
        return this.api.archiveScriptByHash(param.workspace, param.hash,  options).toPromise();
    }

    /**
     * archive script by path
     * @param param the request object
     */
    public archiveScriptByPath(param: ScriptApiArchiveScriptByPathRequest, options?: Configuration): Promise<string> {
        return this.api.archiveScriptByPath(param.workspace, param.path,  options).toPromise();
    }

    /**
     * create script
     * @param param the request object
     */
    public createScript(param: ScriptApiCreateScriptRequest, options?: Configuration): Promise<string> {
        return this.api.createScript(param.workspace, param.inlineObject10,  options).toPromise();
    }

    /**
     * delete script by hash (erase content but keep hash)
     * @param param the request object
     */
    public deleteScriptByHash(param: ScriptApiDeleteScriptByHashRequest, options?: Configuration): Promise<Script> {
        return this.api.deleteScriptByHash(param.workspace, param.hash,  options).toPromise();
    }

    /**
     * inspect deno code to infer jsonschema of arguments
     * @param param the request object
     */
    public denoToJsonschema(param: ScriptApiDenoToJsonschemaRequest, options?: Configuration): Promise<MainArgSignature> {
        return this.api.denoToJsonschema(param.body,  options).toPromise();
    }

    /**
     * get script by hash
     * @param param the request object
     */
    public getScriptByHash(param: ScriptApiGetScriptByHashRequest, options?: Configuration): Promise<Script> {
        return this.api.getScriptByHash(param.workspace, param.hash,  options).toPromise();
    }

    /**
     * get script by path
     * @param param the request object
     */
    public getScriptByPath(param: ScriptApiGetScriptByPathRequest, options?: Configuration): Promise<Script> {
        return this.api.getScriptByPath(param.workspace, param.path,  options).toPromise();
    }

    /**
     * get script deployment status
     * @param param the request object
     */
    public getScriptDeploymentStatus(param: ScriptApiGetScriptDeploymentStatusRequest, options?: Configuration): Promise<InlineResponse2001> {
        return this.api.getScriptDeploymentStatus(param.workspace, param.hash,  options).toPromise();
    }

    /**
     * list all available scripts
     * @param param the request object
     */
    public listScripts(param: ScriptApiListScriptsRequest, options?: Configuration): Promise<Array<Script>> {
        return this.api.listScripts(param.workspace, param.page, param.perPage, param.orderDesc, param.createdBy, param.pathStart, param.pathExact, param.firstParentHash, param.lastParentHash, param.parentHash, param.showArchived, param.isTemplate,  options).toPromise();
    }

    /**
     * inspect python code to infer jsonschema of arguments
     * @param param the request object
     */
    public pythonToJsonschema(param: ScriptApiPythonToJsonschemaRequest, options?: Configuration): Promise<MainArgSignature> {
        return this.api.pythonToJsonschema(param.body,  options).toPromise();
    }

}

import { ObservableSettingsApi } from "./ObservableAPI.ts";
import { SettingsApiRequestFactory, SettingsApiResponseProcessor} from "../apis/SettingsApi.ts";

export interface SettingsApiBackendVersionRequest {
}

export interface SettingsApiGetOpenApiYamlRequest {
}

export class ObjectSettingsApi {
    private api: ObservableSettingsApi

    public constructor(configuration: Configuration, requestFactory?: SettingsApiRequestFactory, responseProcessor?: SettingsApiResponseProcessor) {
        this.api = new ObservableSettingsApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * get backend version
     * @param param the request object
     */
    public backendVersion(param: SettingsApiBackendVersionRequest = {}, options?: Configuration): Promise<string> {
        return this.api.backendVersion( options).toPromise();
    }

    /**
     * get openapi yaml spec
     * @param param the request object
     */
    public getOpenApiYaml(param: SettingsApiGetOpenApiYamlRequest = {}, options?: Configuration): Promise<string> {
        return this.api.getOpenApiYaml( options).toPromise();
    }

}

import { ObservableUserApi } from "./ObservableAPI.ts";
import { UserApiRequestFactory, UserApiResponseProcessor} from "../apis/UserApi.ts";

export interface UserApiAcceptInviteRequest {
    /**
     * 
     * @type InlineObject5
     * @memberof UserApiacceptInvite
     */
    inlineObject5: InlineObject5
}

export interface UserApiCreateTokenRequest {
    /**
     * new token
     * @type NewToken
     * @memberof UserApicreateToken
     */
    newToken: NewToken
}

export interface UserApiCreateUserRequest {
    /**
     * 
     * @type string
     * @memberof UserApicreateUser
     */
    workspace: string
    /**
     * new user
     * @type NewUser
     * @memberof UserApicreateUser
     */
    newUser: NewUser
}

export interface UserApiCreateUserGloballyRequest {
    /**
     * 
     * @type InlineObject1
     * @memberof UserApicreateUserGlobally
     */
    inlineObject1: InlineObject1
}

export interface UserApiDeclineInviteRequest {
    /**
     * 
     * @type InlineObject6
     * @memberof UserApideclineInvite
     */
    inlineObject6: InlineObject6
}

export interface UserApiDeleteTokenRequest {
    /**
     * 
     * @type string
     * @memberof UserApideleteToken
     */
    tokenPrefix: string
}

export interface UserApiDeleteUserRequest {
    /**
     * 
     * @type string
     * @memberof UserApideleteUser
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof UserApideleteUser
     */
    username: string
}

export interface UserApiGetCurrentEmailRequest {
}

export interface UserApiGlobalUserUpdateRequest {
    /**
     * 
     * @type string
     * @memberof UserApiglobalUserUpdate
     */
    email: string
    /**
     * 
     * @type InlineObject2
     * @memberof UserApiglobalUserUpdate
     */
    inlineObject2: InlineObject2
}

export interface UserApiGlobalWhoamiRequest {
}

export interface UserApiLeaveWorkspaceRequest {
    /**
     * 
     * @type string
     * @memberof UserApileaveWorkspace
     */
    workspace: string
}

export interface UserApiListTokensRequest {
}

export interface UserApiListUsernamesRequest {
    /**
     * 
     * @type string
     * @memberof UserApilistUsernames
     */
    workspace: string
}

export interface UserApiListUsersRequest {
    /**
     * 
     * @type string
     * @memberof UserApilistUsers
     */
    workspace: string
}

export interface UserApiListUsersAsSuperAdminRequest {
    /**
     * which page to return (start at 1, default 1)
     * @type number
     * @memberof UserApilistUsersAsSuperAdmin
     */
    page?: number
    /**
     * number of items to return for a given page (default 30, max 100)
     * @type number
     * @memberof UserApilistUsersAsSuperAdmin
     */
    perPage?: number
}

export interface UserApiListWorkspaceInvitesRequest {
}

export interface UserApiLoginRequest {
    /**
     * Partially filled script
     * @type Login
     * @memberof UserApilogin
     */
    login: Login
}

export interface UserApiLogoutRequest {
}

export interface UserApiSetPasswordRequest {
    /**
     * 
     * @type InlineObject
     * @memberof UserApisetPassword
     */
    inlineObject: InlineObject
}

export interface UserApiUpdateUserRequest {
    /**
     * 
     * @type string
     * @memberof UserApiupdateUser
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof UserApiupdateUser
     */
    username: string
    /**
     * new user
     * @type EditWorkspaceUser
     * @memberof UserApiupdateUser
     */
    editWorkspaceUser: EditWorkspaceUser
}

export interface UserApiWhoamiRequest {
    /**
     * 
     * @type string
     * @memberof UserApiwhoami
     */
    workspace: string
}

export interface UserApiWhoisRequest {
    /**
     * 
     * @type string
     * @memberof UserApiwhois
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof UserApiwhois
     */
    username: string
}

export class ObjectUserApi {
    private api: ObservableUserApi

    public constructor(configuration: Configuration, requestFactory?: UserApiRequestFactory, responseProcessor?: UserApiResponseProcessor) {
        this.api = new ObservableUserApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * accept invite to workspace
     * @param param the request object
     */
    public acceptInvite(param: UserApiAcceptInviteRequest, options?: Configuration): Promise<string> {
        return this.api.acceptInvite(param.inlineObject5,  options).toPromise();
    }

    /**
     * create token
     * @param param the request object
     */
    public createToken(param: UserApiCreateTokenRequest, options?: Configuration): Promise<string> {
        return this.api.createToken(param.newToken,  options).toPromise();
    }

    /**
     * create user (require admin privilege)
     * @param param the request object
     */
    public createUser(param: UserApiCreateUserRequest, options?: Configuration): Promise<string> {
        return this.api.createUser(param.workspace, param.newUser,  options).toPromise();
    }

    /**
     * create user
     * @param param the request object
     */
    public createUserGlobally(param: UserApiCreateUserGloballyRequest, options?: Configuration): Promise<string> {
        return this.api.createUserGlobally(param.inlineObject1,  options).toPromise();
    }

    /**
     * decline invite to workspace
     * @param param the request object
     */
    public declineInvite(param: UserApiDeclineInviteRequest, options?: Configuration): Promise<string> {
        return this.api.declineInvite(param.inlineObject6,  options).toPromise();
    }

    /**
     * delete token
     * @param param the request object
     */
    public deleteToken(param: UserApiDeleteTokenRequest, options?: Configuration): Promise<string> {
        return this.api.deleteToken(param.tokenPrefix,  options).toPromise();
    }

    /**
     * delete user (require admin privilege)
     * @param param the request object
     */
    public deleteUser(param: UserApiDeleteUserRequest, options?: Configuration): Promise<string> {
        return this.api.deleteUser(param.workspace, param.username,  options).toPromise();
    }

    /**
     * get current user email (if logged in)
     * @param param the request object
     */
    public getCurrentEmail(param: UserApiGetCurrentEmailRequest = {}, options?: Configuration): Promise<string> {
        return this.api.getCurrentEmail( options).toPromise();
    }

    /**
     * global update user (require super admin)
     * @param param the request object
     */
    public globalUserUpdate(param: UserApiGlobalUserUpdateRequest, options?: Configuration): Promise<string> {
        return this.api.globalUserUpdate(param.email, param.inlineObject2,  options).toPromise();
    }

    /**
     * get current global whoami (if logged in)
     * @param param the request object
     */
    public globalWhoami(param: UserApiGlobalWhoamiRequest = {}, options?: Configuration): Promise<GlobalUserInfo> {
        return this.api.globalWhoami( options).toPromise();
    }

    /**
     * leave workspace
     * @param param the request object
     */
    public leaveWorkspace(param: UserApiLeaveWorkspaceRequest, options?: Configuration): Promise<string> {
        return this.api.leaveWorkspace(param.workspace,  options).toPromise();
    }

    /**
     * list token
     * @param param the request object
     */
    public listTokens(param: UserApiListTokensRequest = {}, options?: Configuration): Promise<Array<TruncatedToken>> {
        return this.api.listTokens( options).toPromise();
    }

    /**
     * list usernames
     * @param param the request object
     */
    public listUsernames(param: UserApiListUsernamesRequest, options?: Configuration): Promise<Array<string>> {
        return this.api.listUsernames(param.workspace,  options).toPromise();
    }

    /**
     * list users
     * @param param the request object
     */
    public listUsers(param: UserApiListUsersRequest, options?: Configuration): Promise<Array<User>> {
        return this.api.listUsers(param.workspace,  options).toPromise();
    }

    /**
     * list all users as super admin (require to be super amdin)
     * @param param the request object
     */
    public listUsersAsSuperAdmin(param: UserApiListUsersAsSuperAdminRequest = {}, options?: Configuration): Promise<Array<GlobalUserInfo>> {
        return this.api.listUsersAsSuperAdmin(param.page, param.perPage,  options).toPromise();
    }

    /**
     * list all workspace invites
     * @param param the request object
     */
    public listWorkspaceInvites(param: UserApiListWorkspaceInvitesRequest = {}, options?: Configuration): Promise<Array<WorkspaceInvite>> {
        return this.api.listWorkspaceInvites( options).toPromise();
    }

    /**
     * login with password
     * @param param the request object
     */
    public login(param: UserApiLoginRequest, options?: Configuration): Promise<string> {
        return this.api.login(param.login,  options).toPromise();
    }

    /**
     * logout
     * @param param the request object
     */
    public logout(param: UserApiLogoutRequest = {}, options?: Configuration): Promise<string> {
        return this.api.logout( options).toPromise();
    }

    /**
     * set password
     * @param param the request object
     */
    public setPassword(param: UserApiSetPasswordRequest, options?: Configuration): Promise<string> {
        return this.api.setPassword(param.inlineObject,  options).toPromise();
    }

    /**
     * update user (require admin privilege)
     * @param param the request object
     */
    public updateUser(param: UserApiUpdateUserRequest, options?: Configuration): Promise<string> {
        return this.api.updateUser(param.workspace, param.username, param.editWorkspaceUser,  options).toPromise();
    }

    /**
     * whoami
     * @param param the request object
     */
    public whoami(param: UserApiWhoamiRequest, options?: Configuration): Promise<User> {
        return this.api.whoami(param.workspace,  options).toPromise();
    }

    /**
     * whois
     * @param param the request object
     */
    public whois(param: UserApiWhoisRequest, options?: Configuration): Promise<User> {
        return this.api.whois(param.workspace, param.username,  options).toPromise();
    }

}

import { ObservableVariableApi } from "./ObservableAPI.ts";
import { VariableApiRequestFactory, VariableApiResponseProcessor} from "../apis/VariableApi.ts";

export interface VariableApiCreateVariableRequest {
    /**
     * 
     * @type string
     * @memberof VariableApicreateVariable
     */
    workspace: string
    /**
     * new variable
     * @type CreateVariable
     * @memberof VariableApicreateVariable
     */
    createVariable: CreateVariable
}

export interface VariableApiDeleteVariableRequest {
    /**
     * 
     * @type string
     * @memberof VariableApideleteVariable
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof VariableApideleteVariable
     */
    path: string
}

export interface VariableApiGetVariableRequest {
    /**
     * 
     * @type string
     * @memberof VariableApigetVariable
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof VariableApigetVariable
     */
    path: string
    /**
     * ask to decrypt secret if this variable is secret (if not secret no effect, default: true) 
     * @type boolean
     * @memberof VariableApigetVariable
     */
    decryptSecret?: boolean
}

export interface VariableApiListContextualVariablesRequest {
    /**
     * 
     * @type string
     * @memberof VariableApilistContextualVariables
     */
    workspace: string
}

export interface VariableApiListVariableRequest {
    /**
     * 
     * @type string
     * @memberof VariableApilistVariable
     */
    workspace: string
}

export interface VariableApiUpdateVariableRequest {
    /**
     * 
     * @type string
     * @memberof VariableApiupdateVariable
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof VariableApiupdateVariable
     */
    path: string
    /**
     * updated variable
     * @type EditVariable
     * @memberof VariableApiupdateVariable
     */
    editVariable: EditVariable
}

export class ObjectVariableApi {
    private api: ObservableVariableApi

    public constructor(configuration: Configuration, requestFactory?: VariableApiRequestFactory, responseProcessor?: VariableApiResponseProcessor) {
        this.api = new ObservableVariableApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * create variable
     * @param param the request object
     */
    public createVariable(param: VariableApiCreateVariableRequest, options?: Configuration): Promise<string> {
        return this.api.createVariable(param.workspace, param.createVariable,  options).toPromise();
    }

    /**
     * delete variable
     * @param param the request object
     */
    public deleteVariable(param: VariableApiDeleteVariableRequest, options?: Configuration): Promise<string> {
        return this.api.deleteVariable(param.workspace, param.path,  options).toPromise();
    }

    /**
     * get variable
     * @param param the request object
     */
    public getVariable(param: VariableApiGetVariableRequest, options?: Configuration): Promise<ListableVariable> {
        return this.api.getVariable(param.workspace, param.path, param.decryptSecret,  options).toPromise();
    }

    /**
     * list contextual variables
     * @param param the request object
     */
    public listContextualVariables(param: VariableApiListContextualVariablesRequest, options?: Configuration): Promise<Array<ContextualVariable>> {
        return this.api.listContextualVariables(param.workspace,  options).toPromise();
    }

    /**
     * list variables
     * @param param the request object
     */
    public listVariable(param: VariableApiListVariableRequest, options?: Configuration): Promise<Array<ListableVariable>> {
        return this.api.listVariable(param.workspace,  options).toPromise();
    }

    /**
     * update variable
     * @param param the request object
     */
    public updateVariable(param: VariableApiUpdateVariableRequest, options?: Configuration): Promise<string> {
        return this.api.updateVariable(param.workspace, param.path, param.editVariable,  options).toPromise();
    }

}

import { ObservableWorkerApi } from "./ObservableAPI.ts";
import { WorkerApiRequestFactory, WorkerApiResponseProcessor} from "../apis/WorkerApi.ts";

export interface WorkerApiListWorkersRequest {
    /**
     * which page to return (start at 1, default 1)
     * @type number
     * @memberof WorkerApilistWorkers
     */
    page?: number
    /**
     * number of items to return for a given page (default 30, max 100)
     * @type number
     * @memberof WorkerApilistWorkers
     */
    perPage?: number
}

export class ObjectWorkerApi {
    private api: ObservableWorkerApi

    public constructor(configuration: Configuration, requestFactory?: WorkerApiRequestFactory, responseProcessor?: WorkerApiResponseProcessor) {
        this.api = new ObservableWorkerApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * list workers
     * @param param the request object
     */
    public listWorkers(param: WorkerApiListWorkersRequest = {}, options?: Configuration): Promise<Array<WorkerPing>> {
        return this.api.listWorkers(param.page, param.perPage,  options).toPromise();
    }

}

import { ObservableWorkspaceApi } from "./ObservableAPI.ts";
import { WorkspaceApiRequestFactory, WorkspaceApiResponseProcessor} from "../apis/WorkspaceApi.ts";

export interface WorkspaceApiCreateWorkspaceRequest {
    /**
     * new token
     * @type CreateWorkspace
     * @memberof WorkspaceApicreateWorkspace
     */
    createWorkspace: CreateWorkspace
}

export interface WorkspaceApiDeleteInviteRequest {
    /**
     * 
     * @type string
     * @memberof WorkspaceApideleteInvite
     */
    workspace: string
    /**
     * 
     * @type InlineObject8
     * @memberof WorkspaceApideleteInvite
     */
    inlineObject8: InlineObject8
}

export interface WorkspaceApiDeleteWorkspaceRequest {
    /**
     * 
     * @type string
     * @memberof WorkspaceApideleteWorkspace
     */
    workspace: string
}

export interface WorkspaceApiDisconnectClientRequest {
    /**
     * 
     * @type string
     * @memberof WorkspaceApidisconnectClient
     */
    workspace: string
    /**
     * 
     * @type string
     * @memberof WorkspaceApidisconnectClient
     */
    clientName: string
}

export interface WorkspaceApiEditSlackCommandRequest {
    /**
     * 
     * @type string
     * @memberof WorkspaceApieditSlackCommand
     */
    workspace: string
    /**
     * 
     * @type InlineObject9
     * @memberof WorkspaceApieditSlackCommand
     */
    inlineObject9: InlineObject9
}

export interface WorkspaceApiGetSettingsRequest {
    /**
     * 
     * @type string
     * @memberof WorkspaceApigetSettings
     */
    workspace: string
}

export interface WorkspaceApiInviteUserRequest {
    /**
     * 
     * @type string
     * @memberof WorkspaceApiinviteUser
     */
    workspace: string
    /**
     * 
     * @type InlineObject7
     * @memberof WorkspaceApiinviteUser
     */
    inlineObject7: InlineObject7
}

export interface WorkspaceApiListPendingInvitesRequest {
    /**
     * 
     * @type string
     * @memberof WorkspaceApilistPendingInvites
     */
    workspace: string
}

export interface WorkspaceApiListUserWorkspacesRequest {
}

export interface WorkspaceApiListWorkspacesRequest {
}

export interface WorkspaceApiListWorkspacesAsSuperAdminRequest {
    /**
     * which page to return (start at 1, default 1)
     * @type number
     * @memberof WorkspaceApilistWorkspacesAsSuperAdmin
     */
    page?: number
    /**
     * number of items to return for a given page (default 30, max 100)
     * @type number
     * @memberof WorkspaceApilistWorkspacesAsSuperAdmin
     */
    perPage?: number
}

export interface WorkspaceApiValidateIdRequest {
    /**
     * 
     * @type InlineObject3
     * @memberof WorkspaceApivalidateId
     */
    inlineObject3: InlineObject3
}

export interface WorkspaceApiValidateUsernameRequest {
    /**
     * 
     * @type InlineObject4
     * @memberof WorkspaceApivalidateUsername
     */
    inlineObject4: InlineObject4
}

export class ObjectWorkspaceApi {
    private api: ObservableWorkspaceApi

    public constructor(configuration: Configuration, requestFactory?: WorkspaceApiRequestFactory, responseProcessor?: WorkspaceApiResponseProcessor) {
        this.api = new ObservableWorkspaceApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * create workspace
     * @param param the request object
     */
    public createWorkspace(param: WorkspaceApiCreateWorkspaceRequest, options?: Configuration): Promise<string> {
        return this.api.createWorkspace(param.createWorkspace,  options).toPromise();
    }

    /**
     * delete user invite
     * @param param the request object
     */
    public deleteInvite(param: WorkspaceApiDeleteInviteRequest, options?: Configuration): Promise<string> {
        return this.api.deleteInvite(param.workspace, param.inlineObject8,  options).toPromise();
    }

    /**
     * delete workspace
     * @param param the request object
     */
    public deleteWorkspace(param: WorkspaceApiDeleteWorkspaceRequest, options?: Configuration): Promise<string> {
        return this.api.deleteWorkspace(param.workspace,  options).toPromise();
    }

    /**
     * disconnect client
     * @param param the request object
     */
    public disconnectClient(param: WorkspaceApiDisconnectClientRequest, options?: Configuration): Promise<string> {
        return this.api.disconnectClient(param.workspace, param.clientName,  options).toPromise();
    }

    /**
     * edit slack command
     * @param param the request object
     */
    public editSlackCommand(param: WorkspaceApiEditSlackCommandRequest, options?: Configuration): Promise<string> {
        return this.api.editSlackCommand(param.workspace, param.inlineObject9,  options).toPromise();
    }

    /**
     * get settings
     * @param param the request object
     */
    public getSettings(param: WorkspaceApiGetSettingsRequest, options?: Configuration): Promise<InlineResponse200> {
        return this.api.getSettings(param.workspace,  options).toPromise();
    }

    /**
     * invite user to workspace
     * @param param the request object
     */
    public inviteUser(param: WorkspaceApiInviteUserRequest, options?: Configuration): Promise<string> {
        return this.api.inviteUser(param.workspace, param.inlineObject7,  options).toPromise();
    }

    /**
     * list pending invites for a workspace
     * @param param the request object
     */
    public listPendingInvites(param: WorkspaceApiListPendingInvitesRequest, options?: Configuration): Promise<Array<WorkspaceInvite>> {
        return this.api.listPendingInvites(param.workspace,  options).toPromise();
    }

    /**
     * list all workspaces visible to me with user info
     * @param param the request object
     */
    public listUserWorkspaces(param: WorkspaceApiListUserWorkspacesRequest = {}, options?: Configuration): Promise<UserWorkspaceList> {
        return this.api.listUserWorkspaces( options).toPromise();
    }

    /**
     * list all workspaces visible to me
     * @param param the request object
     */
    public listWorkspaces(param: WorkspaceApiListWorkspacesRequest = {}, options?: Configuration): Promise<Array<Workspace>> {
        return this.api.listWorkspaces( options).toPromise();
    }

    /**
     * list all workspaces as super admin (require to be super amdin)
     * @param param the request object
     */
    public listWorkspacesAsSuperAdmin(param: WorkspaceApiListWorkspacesAsSuperAdminRequest = {}, options?: Configuration): Promise<Array<Workspace>> {
        return this.api.listWorkspacesAsSuperAdmin(param.page, param.perPage,  options).toPromise();
    }

    /**
     * validate id
     * @param param the request object
     */
    public validateId(param: WorkspaceApiValidateIdRequest, options?: Configuration): Promise<string> {
        return this.api.validateId(param.inlineObject3,  options).toPromise();
    }

    /**
     * validate username
     * @param param the request object
     */
    public validateUsername(param: WorkspaceApiValidateUsernameRequest, options?: Configuration): Promise<string> {
        return this.api.validateUsername(param.inlineObject4,  options).toPromise();
    }

}
