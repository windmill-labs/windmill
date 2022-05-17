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
import { ObservableAdminApi } from './ObservableAPI.ts';

import { AdminApiRequestFactory, AdminApiResponseProcessor} from "../apis/AdminApi.ts";
export class PromiseAdminApi {
    private api: ObservableAdminApi

    public constructor(
        configuration: Configuration,
        requestFactory?: AdminApiRequestFactory,
        responseProcessor?: AdminApiResponseProcessor
    ) {
        this.api = new ObservableAdminApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * create user (require admin privilege)
     * @param workspace 
     * @param newUser new user
     */
    public createUser(workspace: string, newUser: NewUser, _options?: Configuration): Promise<string> {
        const result = this.api.createUser(workspace, newUser, _options);
        return result.toPromise();
    }

    /**
     * delete user (require admin privilege)
     * @param workspace 
     * @param username 
     */
    public deleteUser(workspace: string, username: string, _options?: Configuration): Promise<string> {
        const result = this.api.deleteUser(workspace, username, _options);
        return result.toPromise();
    }

    /**
     * update user (require admin privilege)
     * @param workspace 
     * @param username 
     * @param editWorkspaceUser new user
     */
    public updateUser(workspace: string, username: string, editWorkspaceUser: EditWorkspaceUser, _options?: Configuration): Promise<string> {
        const result = this.api.updateUser(workspace, username, editWorkspaceUser, _options);
        return result.toPromise();
    }


}



import { ObservableAuditApi } from './ObservableAPI.ts';

import { AuditApiRequestFactory, AuditApiResponseProcessor} from "../apis/AuditApi.ts";
export class PromiseAuditApi {
    private api: ObservableAuditApi

    public constructor(
        configuration: Configuration,
        requestFactory?: AuditApiRequestFactory,
        responseProcessor?: AuditApiResponseProcessor
    ) {
        this.api = new ObservableAuditApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * get audit log (requires admin privilege)
     * @param workspace 
     * @param id 
     */
    public getAuditLog(workspace: string, id: number, _options?: Configuration): Promise<AuditLog> {
        const result = this.api.getAuditLog(workspace, id, _options);
        return result.toPromise();
    }

    /**
     * list audit logs (requires admin privilege)
     * @param workspace 
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     * @param before filter on created before (exclusive) timestamp
     * @param after filter on created after (exclusive) timestamp
     * @param username filter on exact username of user
     * @param operation filter on exact or prefix name of operation
     * @param resource filter on exact or prefix name of resource
     * @param actionKind filter on type of operation
     */
    public listAuditLogs(workspace: string, page?: number, perPage?: number, before?: Date, after?: Date, username?: string, operation?: string, resource?: string, actionKind?: 'Create' | 'Update' | 'Delete' | 'Execute', _options?: Configuration): Promise<Array<AuditLog>> {
        const result = this.api.listAuditLogs(workspace, page, perPage, before, after, username, operation, resource, actionKind, _options);
        return result.toPromise();
    }


}



import { ObservableFlowApi } from './ObservableAPI.ts';

import { FlowApiRequestFactory, FlowApiResponseProcessor} from "../apis/FlowApi.ts";
export class PromiseFlowApi {
    private api: ObservableFlowApi

    public constructor(
        configuration: Configuration,
        requestFactory?: FlowApiRequestFactory,
        responseProcessor?: FlowApiResponseProcessor
    ) {
        this.api = new ObservableFlowApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * archive flow by path
     * @param workspace 
     * @param path 
     */
    public archiveFlowByPath(workspace: string, path: string, _options?: Configuration): Promise<string> {
        const result = this.api.archiveFlowByPath(workspace, path, _options);
        return result.toPromise();
    }

    /**
     * create flow
     * @param workspace 
     * @param inlineObject11 
     */
    public createFlow(workspace: string, inlineObject11: InlineObject11, _options?: Configuration): Promise<string> {
        const result = this.api.createFlow(workspace, inlineObject11, _options);
        return result.toPromise();
    }

    /**
     * get flow by path
     * @param workspace 
     * @param path 
     */
    public getFlowByPath(workspace: string, path: string, _options?: Configuration): Promise<Flow> {
        const result = this.api.getFlowByPath(workspace, path, _options);
        return result.toPromise();
    }

    /**
     * list all available flows
     * @param workspace 
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     * @param orderDesc order by desc order (default true)
     * @param createdBy mask to filter exact matching user creator
     * @param pathStart mask to filter matching starting parh
     * @param pathExact mask to filter exact matching path
     * @param showArchived (default false) show also the archived files. when multiple archived hash share the same path, only the ones with the latest create_at are displayed. 
     */
    public listFlows(workspace: string, page?: number, perPage?: number, orderDesc?: boolean, createdBy?: string, pathStart?: string, pathExact?: string, showArchived?: boolean, _options?: Configuration): Promise<Array<Flow>> {
        const result = this.api.listFlows(workspace, page, perPage, orderDesc, createdBy, pathStart, pathExact, showArchived, _options);
        return result.toPromise();
    }

    /**
     * update flow
     * @param workspace 
     * @param path 
     * @param inlineObject12 
     */
    public updateFlow(workspace: string, path: string, inlineObject12: InlineObject12, _options?: Configuration): Promise<string> {
        const result = this.api.updateFlow(workspace, path, inlineObject12, _options);
        return result.toPromise();
    }


}



import { ObservableGranularAclApi } from './ObservableAPI.ts';

import { GranularAclApiRequestFactory, GranularAclApiResponseProcessor} from "../apis/GranularAclApi.ts";
export class PromiseGranularAclApi {
    private api: ObservableGranularAclApi

    public constructor(
        configuration: Configuration,
        requestFactory?: GranularAclApiRequestFactory,
        responseProcessor?: GranularAclApiResponseProcessor
    ) {
        this.api = new ObservableGranularAclApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * add granular acls
     * @param workspace 
     * @param path 
     * @param kind 
     * @param inlineObject20 
     */
    public addGranularAcls(workspace: string, path: string, kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow', inlineObject20: InlineObject20, _options?: Configuration): Promise<string> {
        const result = this.api.addGranularAcls(workspace, path, kind, inlineObject20, _options);
        return result.toPromise();
    }

    /**
     * get granular acls
     * @param workspace 
     * @param path 
     * @param kind 
     */
    public getGranularAcls(workspace: string, path: string, kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow', _options?: Configuration): Promise<{ [key: string]: boolean; }> {
        const result = this.api.getGranularAcls(workspace, path, kind, _options);
        return result.toPromise();
    }

    /**
     * remove granular acls
     * @param workspace 
     * @param path 
     * @param kind 
     * @param inlineObject21 
     */
    public removeGranularAcls(workspace: string, path: string, kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow', inlineObject21: InlineObject21, _options?: Configuration): Promise<string> {
        const result = this.api.removeGranularAcls(workspace, path, kind, inlineObject21, _options);
        return result.toPromise();
    }


}



import { ObservableGroupApi } from './ObservableAPI.ts';

import { GroupApiRequestFactory, GroupApiResponseProcessor} from "../apis/GroupApi.ts";
export class PromiseGroupApi {
    private api: ObservableGroupApi

    public constructor(
        configuration: Configuration,
        requestFactory?: GroupApiRequestFactory,
        responseProcessor?: GroupApiResponseProcessor
    ) {
        this.api = new ObservableGroupApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * add user to group
     * @param workspace 
     * @param name 
     * @param inlineObject18 
     */
    public addUserToGroup(workspace: string, name: string, inlineObject18: InlineObject18, _options?: Configuration): Promise<string> {
        const result = this.api.addUserToGroup(workspace, name, inlineObject18, _options);
        return result.toPromise();
    }

    /**
     * create group
     * @param workspace 
     * @param inlineObject16 
     */
    public createGroup(workspace: string, inlineObject16: InlineObject16, _options?: Configuration): Promise<string> {
        const result = this.api.createGroup(workspace, inlineObject16, _options);
        return result.toPromise();
    }

    /**
     * delete group
     * @param workspace 
     * @param name 
     */
    public deleteGroup(workspace: string, name: string, _options?: Configuration): Promise<string> {
        const result = this.api.deleteGroup(workspace, name, _options);
        return result.toPromise();
    }

    /**
     * get group
     * @param workspace 
     * @param name 
     */
    public getGroup(workspace: string, name: string, _options?: Configuration): Promise<Group> {
        const result = this.api.getGroup(workspace, name, _options);
        return result.toPromise();
    }

    /**
     * list group names
     * @param workspace 
     */
    public listGroupNames(workspace: string, _options?: Configuration): Promise<Array<string>> {
        const result = this.api.listGroupNames(workspace, _options);
        return result.toPromise();
    }

    /**
     * list groups
     * @param workspace 
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     */
    public listGroups(workspace: string, page?: number, perPage?: number, _options?: Configuration): Promise<Array<Group>> {
        const result = this.api.listGroups(workspace, page, perPage, _options);
        return result.toPromise();
    }

    /**
     * remove user to group
     * @param workspace 
     * @param name 
     * @param inlineObject19 
     */
    public removeUserToGroup(workspace: string, name: string, inlineObject19: InlineObject19, _options?: Configuration): Promise<string> {
        const result = this.api.removeUserToGroup(workspace, name, inlineObject19, _options);
        return result.toPromise();
    }

    /**
     * update group
     * @param workspace 
     * @param name 
     * @param inlineObject17 
     */
    public updateGroup(workspace: string, name: string, inlineObject17: InlineObject17, _options?: Configuration): Promise<string> {
        const result = this.api.updateGroup(workspace, name, inlineObject17, _options);
        return result.toPromise();
    }


}



import { ObservableJobApi } from './ObservableAPI.ts';

import { JobApiRequestFactory, JobApiResponseProcessor} from "../apis/JobApi.ts";
export class PromiseJobApi {
    private api: ObservableJobApi

    public constructor(
        configuration: Configuration,
        requestFactory?: JobApiRequestFactory,
        responseProcessor?: JobApiResponseProcessor
    ) {
        this.api = new ObservableJobApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * cancel queued job
     * @param workspace 
     * @param id 
     * @param inlineObject13 
     */
    public cancelQueuedJob(workspace: string, id: string, inlineObject13: InlineObject13, _options?: Configuration): Promise<string> {
        const result = this.api.cancelQueuedJob(workspace, id, inlineObject13, _options);
        return result.toPromise();
    }

    /**
     * delete completed job (erase content but keep run id)
     * @param workspace 
     * @param id 
     */
    public deleteCompletedJob(workspace: string, id: string, _options?: Configuration): Promise<CompletedJob> {
        const result = this.api.deleteCompletedJob(workspace, id, _options);
        return result.toPromise();
    }

    /**
     * get completed job
     * @param workspace 
     * @param id 
     */
    public getCompletedJob(workspace: string, id: string, _options?: Configuration): Promise<CompletedJob> {
        const result = this.api.getCompletedJob(workspace, id, _options);
        return result.toPromise();
    }

    /**
     * get job
     * @param workspace 
     * @param id 
     */
    public getJob(workspace: string, id: string, _options?: Configuration): Promise<Job> {
        const result = this.api.getJob(workspace, id, _options);
        return result.toPromise();
    }

    /**
     * get job updates
     * @param workspace 
     * @param id 
     * @param running 
     * @param logOffset 
     */
    public getJobUpdates(workspace: string, id: string, running?: boolean, logOffset?: number, _options?: Configuration): Promise<InlineResponse2002> {
        const result = this.api.getJobUpdates(workspace, id, running, logOffset, _options);
        return result.toPromise();
    }

    /**
     * list all available completed jobs
     * @param workspace 
     * @param orderDesc order by desc order (default true)
     * @param createdBy mask to filter exact matching user creator
     * @param parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param scriptPathExact mask to filter exact matching path
     * @param scriptPathStart mask to filter matching starting path
     * @param scriptHash mask to filter exact matching path
     * @param createdBefore filter on created before (inclusive) timestamp
     * @param createdAfter filter on created after (exclusive) timestamp
     * @param success filter on successful jobs
     * @param jobKinds filter on job kind (values &#39;preview&#39;, &#39;script&#39;, &#39;dependencies&#39;, &#39;flow&#39;) separated by,
     */
    public listCompletedJobs(workspace: string, orderDesc?: boolean, createdBy?: string, parentJob?: string, scriptPathExact?: string, scriptPathStart?: string, scriptHash?: string, createdBefore?: Date, createdAfter?: Date, success?: boolean, jobKinds?: string, _options?: Configuration): Promise<Array<CompletedJob>> {
        const result = this.api.listCompletedJobs(workspace, orderDesc, createdBy, parentJob, scriptPathExact, scriptPathStart, scriptHash, createdBefore, createdAfter, success, jobKinds, _options);
        return result.toPromise();
    }

    /**
     * list all available jobs
     * @param workspace 
     * @param createdBy mask to filter exact matching user creator
     * @param parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param scriptPathExact mask to filter exact matching path
     * @param scriptPathStart mask to filter matching starting path
     * @param scriptHash mask to filter exact matching path
     * @param createdBefore filter on created before (inclusive) timestamp
     * @param createdAfter filter on created after (exclusive) timestamp
     * @param jobKinds filter on job kind (values &#39;preview&#39;, &#39;script&#39;, &#39;dependencies&#39;, &#39;flow&#39;) separated by,
     * @param success filter on successful jobs
     */
    public listJobs(workspace: string, createdBy?: string, parentJob?: string, scriptPathExact?: string, scriptPathStart?: string, scriptHash?: string, createdBefore?: Date, createdAfter?: Date, jobKinds?: string, success?: boolean, _options?: Configuration): Promise<Array<Job>> {
        const result = this.api.listJobs(workspace, createdBy, parentJob, scriptPathExact, scriptPathStart, scriptHash, createdBefore, createdAfter, jobKinds, success, _options);
        return result.toPromise();
    }

    /**
     * list all available queued jobs
     * @param workspace 
     * @param orderDesc order by desc order (default true)
     * @param createdBy mask to filter exact matching user creator
     * @param parentJob The parent job that is at the origin and responsible for the execution of this script if any
     * @param scriptPathExact mask to filter exact matching path
     * @param scriptPathStart mask to filter matching starting path
     * @param scriptHash mask to filter exact matching path
     * @param createdBefore filter on created before (inclusive) timestamp
     * @param createdAfter filter on created after (exclusive) timestamp
     * @param success filter on successful jobs
     * @param jobKinds filter on job kind (values &#39;preview&#39;, &#39;script&#39;, &#39;dependencies&#39;, &#39;flow&#39;) separated by,
     */
    public listQueue(workspace: string, orderDesc?: boolean, createdBy?: string, parentJob?: string, scriptPathExact?: string, scriptPathStart?: string, scriptHash?: string, createdBefore?: Date, createdAfter?: Date, success?: boolean, jobKinds?: string, _options?: Configuration): Promise<Array<QueuedJob>> {
        const result = this.api.listQueue(workspace, orderDesc, createdBy, parentJob, scriptPathExact, scriptPathStart, scriptHash, createdBefore, createdAfter, success, jobKinds, _options);
        return result.toPromise();
    }

    /**
     * run flow by path
     * @param workspace 
     * @param path 
     * @param requestBody flow args
     * @param scheduledFor when to schedule this job (leave empty for immediate run)
     * @param scheduledInSecs schedule the script to execute in the number of seconds starting now
     * @param parentJob The parent job that is at the origin and responsible for the execution of this script if any
     */
    public runFlowByPath(workspace: string, path: string, requestBody: { [key: string]: any; }, scheduledFor?: Date, scheduledInSecs?: number, parentJob?: string, _options?: Configuration): Promise<string> {
        const result = this.api.runFlowByPath(workspace, path, requestBody, scheduledFor, scheduledInSecs, parentJob, _options);
        return result.toPromise();
    }

    /**
     * run flow preview
     * @param workspace 
     * @param flowPreview preview
     */
    public runFlowPreview(workspace: string, flowPreview: FlowPreview, _options?: Configuration): Promise<string> {
        const result = this.api.runFlowPreview(workspace, flowPreview, _options);
        return result.toPromise();
    }

    /**
     * run script by hash
     * @param workspace 
     * @param hash 
     * @param body Partially filled args
     * @param scheduledFor when to schedule this job (leave empty for immediate run)
     * @param scheduledInSecs schedule the script to execute in the number of seconds starting now
     * @param parentJob The parent job that is at the origin and responsible for the execution of this script if any
     */
    public runScriptByHash(workspace: string, hash: string, body: any, scheduledFor?: Date, scheduledInSecs?: number, parentJob?: string, _options?: Configuration): Promise<string> {
        const result = this.api.runScriptByHash(workspace, hash, body, scheduledFor, scheduledInSecs, parentJob, _options);
        return result.toPromise();
    }

    /**
     * run script by path
     * @param workspace 
     * @param path 
     * @param requestBody script args
     * @param scheduledFor when to schedule this job (leave empty for immediate run)
     * @param scheduledInSecs schedule the script to execute in the number of seconds starting now
     * @param parentJob The parent job that is at the origin and responsible for the execution of this script if any
     */
    public runScriptByPath(workspace: string, path: string, requestBody: { [key: string]: any; }, scheduledFor?: Date, scheduledInSecs?: number, parentJob?: string, _options?: Configuration): Promise<string> {
        const result = this.api.runScriptByPath(workspace, path, requestBody, scheduledFor, scheduledInSecs, parentJob, _options);
        return result.toPromise();
    }

    /**
     * run script preview
     * @param workspace 
     * @param preview previw
     */
    public runScriptPreview(workspace: string, preview: Preview, _options?: Configuration): Promise<string> {
        const result = this.api.runScriptPreview(workspace, preview, _options);
        return result.toPromise();
    }


}



import { ObservableResourceApi } from './ObservableAPI.ts';

import { ResourceApiRequestFactory, ResourceApiResponseProcessor} from "../apis/ResourceApi.ts";
export class PromiseResourceApi {
    private api: ObservableResourceApi

    public constructor(
        configuration: Configuration,
        requestFactory?: ResourceApiRequestFactory,
        responseProcessor?: ResourceApiResponseProcessor
    ) {
        this.api = new ObservableResourceApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * create resource
     * @param workspace 
     * @param createResource new resource
     */
    public createResource(workspace: string, createResource: CreateResource, _options?: Configuration): Promise<string> {
        const result = this.api.createResource(workspace, createResource, _options);
        return result.toPromise();
    }

    /**
     * create resource_type
     * @param workspace 
     * @param resourceType new resource_type
     */
    public createResourceType(workspace: string, resourceType: ResourceType, _options?: Configuration): Promise<string> {
        const result = this.api.createResourceType(workspace, resourceType, _options);
        return result.toPromise();
    }

    /**
     * delete resource
     * @param workspace 
     * @param path 
     */
    public deleteResource(workspace: string, path: string, _options?: Configuration): Promise<string> {
        const result = this.api.deleteResource(workspace, path, _options);
        return result.toPromise();
    }

    /**
     * delete resource_type
     * @param workspace 
     * @param path 
     */
    public deleteResourceType(workspace: string, path: string, _options?: Configuration): Promise<string> {
        const result = this.api.deleteResourceType(workspace, path, _options);
        return result.toPromise();
    }

    /**
     * get resource
     * @param workspace 
     * @param path 
     */
    public getResource(workspace: string, path: string, _options?: Configuration): Promise<Resource> {
        const result = this.api.getResource(workspace, path, _options);
        return result.toPromise();
    }

    /**
     * get resource_type
     * @param workspace 
     * @param path 
     */
    public getResourceType(workspace: string, path: string, _options?: Configuration): Promise<ResourceType> {
        const result = this.api.getResourceType(workspace, path, _options);
        return result.toPromise();
    }

    /**
     * list resources
     * @param workspace 
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     * @param resourceType resource_type to list from
     */
    public listResource(workspace: string, page?: number, perPage?: number, resourceType?: string, _options?: Configuration): Promise<Array<Resource>> {
        const result = this.api.listResource(workspace, page, perPage, resourceType, _options);
        return result.toPromise();
    }

    /**
     * list resource_types
     * @param workspace 
     */
    public listResourceType(workspace: string, _options?: Configuration): Promise<Array<ResourceType>> {
        const result = this.api.listResourceType(workspace, _options);
        return result.toPromise();
    }

    /**
     * list resource_types names
     * @param workspace 
     */
    public listResourceTypeNames(workspace: string, _options?: Configuration): Promise<Array<string>> {
        const result = this.api.listResourceTypeNames(workspace, _options);
        return result.toPromise();
    }

    /**
     * update resource
     * @param workspace 
     * @param path 
     * @param editResource updated resource
     */
    public updateResource(workspace: string, path: string, editResource: EditResource, _options?: Configuration): Promise<string> {
        const result = this.api.updateResource(workspace, path, editResource, _options);
        return result.toPromise();
    }

    /**
     * update resource_type
     * @param workspace 
     * @param path 
     * @param editResourceType updated resource_type
     */
    public updateResourceType(workspace: string, path: string, editResourceType: EditResourceType, _options?: Configuration): Promise<string> {
        const result = this.api.updateResourceType(workspace, path, editResourceType, _options);
        return result.toPromise();
    }


}



import { ObservableScheduleApi } from './ObservableAPI.ts';

import { ScheduleApiRequestFactory, ScheduleApiResponseProcessor} from "../apis/ScheduleApi.ts";
export class PromiseScheduleApi {
    private api: ObservableScheduleApi

    public constructor(
        configuration: Configuration,
        requestFactory?: ScheduleApiRequestFactory,
        responseProcessor?: ScheduleApiResponseProcessor
    ) {
        this.api = new ObservableScheduleApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * create schedule
     * @param workspace 
     * @param newSchedule new schedule
     */
    public createSchedule(workspace: string, newSchedule: NewSchedule, _options?: Configuration): Promise<string> {
        const result = this.api.createSchedule(workspace, newSchedule, _options);
        return result.toPromise();
    }

    /**
     * get schedule
     * @param workspace 
     * @param path 
     */
    public getSchedule(workspace: string, path: string, _options?: Configuration): Promise<Schedule> {
        const result = this.api.getSchedule(workspace, path, _options);
        return result.toPromise();
    }

    /**
     * list schedules
     * @param workspace 
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     */
    public listSchedules(workspace: string, page?: number, perPage?: number, _options?: Configuration): Promise<Array<Schedule>> {
        const result = this.api.listSchedules(workspace, page, perPage, _options);
        return result.toPromise();
    }

    /**
     * preview schedule
     * @param inlineObject14 
     */
    public previewSchedule(inlineObject14: InlineObject14, _options?: Configuration): Promise<Array<Date>> {
        const result = this.api.previewSchedule(inlineObject14, _options);
        return result.toPromise();
    }

    /**
     * set enabled schedule
     * @param workspace 
     * @param path 
     * @param inlineObject15 
     */
    public setScheduleEnabled(workspace: string, path: string, inlineObject15: InlineObject15, _options?: Configuration): Promise<string> {
        const result = this.api.setScheduleEnabled(workspace, path, inlineObject15, _options);
        return result.toPromise();
    }

    /**
     * update schedule
     * @param workspace 
     * @param path 
     * @param editSchedule updated schedule
     */
    public updateSchedule(workspace: string, path: string, editSchedule: EditSchedule, _options?: Configuration): Promise<string> {
        const result = this.api.updateSchedule(workspace, path, editSchedule, _options);
        return result.toPromise();
    }


}



import { ObservableScriptApi } from './ObservableAPI.ts';

import { ScriptApiRequestFactory, ScriptApiResponseProcessor} from "../apis/ScriptApi.ts";
export class PromiseScriptApi {
    private api: ObservableScriptApi

    public constructor(
        configuration: Configuration,
        requestFactory?: ScriptApiRequestFactory,
        responseProcessor?: ScriptApiResponseProcessor
    ) {
        this.api = new ObservableScriptApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * archive script by hash
     * @param workspace 
     * @param hash 
     */
    public archiveScriptByHash(workspace: string, hash: string, _options?: Configuration): Promise<Script> {
        const result = this.api.archiveScriptByHash(workspace, hash, _options);
        return result.toPromise();
    }

    /**
     * archive script by path
     * @param workspace 
     * @param path 
     */
    public archiveScriptByPath(workspace: string, path: string, _options?: Configuration): Promise<string> {
        const result = this.api.archiveScriptByPath(workspace, path, _options);
        return result.toPromise();
    }

    /**
     * create script
     * @param workspace 
     * @param inlineObject10 
     */
    public createScript(workspace: string, inlineObject10: InlineObject10, _options?: Configuration): Promise<string> {
        const result = this.api.createScript(workspace, inlineObject10, _options);
        return result.toPromise();
    }

    /**
     * delete script by hash (erase content but keep hash)
     * @param workspace 
     * @param hash 
     */
    public deleteScriptByHash(workspace: string, hash: string, _options?: Configuration): Promise<Script> {
        const result = this.api.deleteScriptByHash(workspace, hash, _options);
        return result.toPromise();
    }

    /**
     * inspect deno code to infer jsonschema of arguments
     * @param body deno code with the main function
     */
    public denoToJsonschema(body: string, _options?: Configuration): Promise<MainArgSignature> {
        const result = this.api.denoToJsonschema(body, _options);
        return result.toPromise();
    }

    /**
     * get script by hash
     * @param workspace 
     * @param hash 
     */
    public getScriptByHash(workspace: string, hash: string, _options?: Configuration): Promise<Script> {
        const result = this.api.getScriptByHash(workspace, hash, _options);
        return result.toPromise();
    }

    /**
     * get script by path
     * @param workspace 
     * @param path 
     */
    public getScriptByPath(workspace: string, path: string, _options?: Configuration): Promise<Script> {
        const result = this.api.getScriptByPath(workspace, path, _options);
        return result.toPromise();
    }

    /**
     * get script deployment status
     * @param workspace 
     * @param hash 
     */
    public getScriptDeploymentStatus(workspace: string, hash: string, _options?: Configuration): Promise<InlineResponse2001> {
        const result = this.api.getScriptDeploymentStatus(workspace, hash, _options);
        return result.toPromise();
    }

    /**
     * list all available scripts
     * @param workspace 
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     * @param orderDesc order by desc order (default true)
     * @param createdBy mask to filter exact matching user creator
     * @param pathStart mask to filter matching starting parh
     * @param pathExact mask to filter exact matching path
     * @param firstParentHash mask to filter scripts whom first direct parent has exact hash
     * @param lastParentHash mask to filter scripts whom last parent in the chain has exact hash.  Beware that each script stores only a limited number of parents. Hence the last parent hash for a script is not necessarily its top-most parent. To find the top-most parent you will have to jump from last to last hash  until finding the parent 
     * @param parentHash is the hash present in the array of stored parent hashes for this script. The same warning applies than for last_parent_hash. A script only store a limited number of direct parent 
     * @param showArchived (default false) show also the archived files. when multiple archived hash share the same path, only the ones with the latest create_at are displayed. 
     * @param isTemplate (default regardless) if true show only the templates if false show only the non templates if not defined, show all regardless of if the script is a template 
     */
    public listScripts(workspace: string, page?: number, perPage?: number, orderDesc?: boolean, createdBy?: string, pathStart?: string, pathExact?: string, firstParentHash?: string, lastParentHash?: string, parentHash?: string, showArchived?: boolean, isTemplate?: boolean, _options?: Configuration): Promise<Array<Script>> {
        const result = this.api.listScripts(workspace, page, perPage, orderDesc, createdBy, pathStart, pathExact, firstParentHash, lastParentHash, parentHash, showArchived, isTemplate, _options);
        return result.toPromise();
    }

    /**
     * inspect python code to infer jsonschema of arguments
     * @param body python code with the main function
     */
    public pythonToJsonschema(body: string, _options?: Configuration): Promise<MainArgSignature> {
        const result = this.api.pythonToJsonschema(body, _options);
        return result.toPromise();
    }


}



import { ObservableSettingsApi } from './ObservableAPI.ts';

import { SettingsApiRequestFactory, SettingsApiResponseProcessor} from "../apis/SettingsApi.ts";
export class PromiseSettingsApi {
    private api: ObservableSettingsApi

    public constructor(
        configuration: Configuration,
        requestFactory?: SettingsApiRequestFactory,
        responseProcessor?: SettingsApiResponseProcessor
    ) {
        this.api = new ObservableSettingsApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * get backend version
     */
    public backendVersion(_options?: Configuration): Promise<string> {
        const result = this.api.backendVersion(_options);
        return result.toPromise();
    }

    /**
     * get openapi yaml spec
     */
    public getOpenApiYaml(_options?: Configuration): Promise<string> {
        const result = this.api.getOpenApiYaml(_options);
        return result.toPromise();
    }


}



import { ObservableUserApi } from './ObservableAPI.ts';

import { UserApiRequestFactory, UserApiResponseProcessor} from "../apis/UserApi.ts";
export class PromiseUserApi {
    private api: ObservableUserApi

    public constructor(
        configuration: Configuration,
        requestFactory?: UserApiRequestFactory,
        responseProcessor?: UserApiResponseProcessor
    ) {
        this.api = new ObservableUserApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * accept invite to workspace
     * @param inlineObject5 
     */
    public acceptInvite(inlineObject5: InlineObject5, _options?: Configuration): Promise<string> {
        const result = this.api.acceptInvite(inlineObject5, _options);
        return result.toPromise();
    }

    /**
     * create token
     * @param newToken new token
     */
    public createToken(newToken: NewToken, _options?: Configuration): Promise<string> {
        const result = this.api.createToken(newToken, _options);
        return result.toPromise();
    }

    /**
     * create user (require admin privilege)
     * @param workspace 
     * @param newUser new user
     */
    public createUser(workspace: string, newUser: NewUser, _options?: Configuration): Promise<string> {
        const result = this.api.createUser(workspace, newUser, _options);
        return result.toPromise();
    }

    /**
     * create user
     * @param inlineObject1 
     */
    public createUserGlobally(inlineObject1: InlineObject1, _options?: Configuration): Promise<string> {
        const result = this.api.createUserGlobally(inlineObject1, _options);
        return result.toPromise();
    }

    /**
     * decline invite to workspace
     * @param inlineObject6 
     */
    public declineInvite(inlineObject6: InlineObject6, _options?: Configuration): Promise<string> {
        const result = this.api.declineInvite(inlineObject6, _options);
        return result.toPromise();
    }

    /**
     * delete token
     * @param tokenPrefix 
     */
    public deleteToken(tokenPrefix: string, _options?: Configuration): Promise<string> {
        const result = this.api.deleteToken(tokenPrefix, _options);
        return result.toPromise();
    }

    /**
     * delete user (require admin privilege)
     * @param workspace 
     * @param username 
     */
    public deleteUser(workspace: string, username: string, _options?: Configuration): Promise<string> {
        const result = this.api.deleteUser(workspace, username, _options);
        return result.toPromise();
    }

    /**
     * get current user email (if logged in)
     */
    public getCurrentEmail(_options?: Configuration): Promise<string> {
        const result = this.api.getCurrentEmail(_options);
        return result.toPromise();
    }

    /**
     * global update user (require super admin)
     * @param email 
     * @param inlineObject2 
     */
    public globalUserUpdate(email: string, inlineObject2: InlineObject2, _options?: Configuration): Promise<string> {
        const result = this.api.globalUserUpdate(email, inlineObject2, _options);
        return result.toPromise();
    }

    /**
     * get current global whoami (if logged in)
     */
    public globalWhoami(_options?: Configuration): Promise<GlobalUserInfo> {
        const result = this.api.globalWhoami(_options);
        return result.toPromise();
    }

    /**
     * leave workspace
     * @param workspace 
     */
    public leaveWorkspace(workspace: string, _options?: Configuration): Promise<string> {
        const result = this.api.leaveWorkspace(workspace, _options);
        return result.toPromise();
    }

    /**
     * list token
     */
    public listTokens(_options?: Configuration): Promise<Array<TruncatedToken>> {
        const result = this.api.listTokens(_options);
        return result.toPromise();
    }

    /**
     * list usernames
     * @param workspace 
     */
    public listUsernames(workspace: string, _options?: Configuration): Promise<Array<string>> {
        const result = this.api.listUsernames(workspace, _options);
        return result.toPromise();
    }

    /**
     * list users
     * @param workspace 
     */
    public listUsers(workspace: string, _options?: Configuration): Promise<Array<User>> {
        const result = this.api.listUsers(workspace, _options);
        return result.toPromise();
    }

    /**
     * list all users as super admin (require to be super amdin)
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     */
    public listUsersAsSuperAdmin(page?: number, perPage?: number, _options?: Configuration): Promise<Array<GlobalUserInfo>> {
        const result = this.api.listUsersAsSuperAdmin(page, perPage, _options);
        return result.toPromise();
    }

    /**
     * list all workspace invites
     */
    public listWorkspaceInvites(_options?: Configuration): Promise<Array<WorkspaceInvite>> {
        const result = this.api.listWorkspaceInvites(_options);
        return result.toPromise();
    }

    /**
     * login with password
     * @param login Partially filled script
     */
    public login(login: Login, _options?: Configuration): Promise<string> {
        const result = this.api.login(login, _options);
        return result.toPromise();
    }

    /**
     * logout
     */
    public logout(_options?: Configuration): Promise<string> {
        const result = this.api.logout(_options);
        return result.toPromise();
    }

    /**
     * set password
     * @param inlineObject 
     */
    public setPassword(inlineObject: InlineObject, _options?: Configuration): Promise<string> {
        const result = this.api.setPassword(inlineObject, _options);
        return result.toPromise();
    }

    /**
     * update user (require admin privilege)
     * @param workspace 
     * @param username 
     * @param editWorkspaceUser new user
     */
    public updateUser(workspace: string, username: string, editWorkspaceUser: EditWorkspaceUser, _options?: Configuration): Promise<string> {
        const result = this.api.updateUser(workspace, username, editWorkspaceUser, _options);
        return result.toPromise();
    }

    /**
     * whoami
     * @param workspace 
     */
    public whoami(workspace: string, _options?: Configuration): Promise<User> {
        const result = this.api.whoami(workspace, _options);
        return result.toPromise();
    }

    /**
     * whois
     * @param workspace 
     * @param username 
     */
    public whois(workspace: string, username: string, _options?: Configuration): Promise<User> {
        const result = this.api.whois(workspace, username, _options);
        return result.toPromise();
    }


}



import { ObservableVariableApi } from './ObservableAPI.ts';

import { VariableApiRequestFactory, VariableApiResponseProcessor} from "../apis/VariableApi.ts";
export class PromiseVariableApi {
    private api: ObservableVariableApi

    public constructor(
        configuration: Configuration,
        requestFactory?: VariableApiRequestFactory,
        responseProcessor?: VariableApiResponseProcessor
    ) {
        this.api = new ObservableVariableApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * create variable
     * @param workspace 
     * @param createVariable new variable
     */
    public createVariable(workspace: string, createVariable: CreateVariable, _options?: Configuration): Promise<string> {
        const result = this.api.createVariable(workspace, createVariable, _options);
        return result.toPromise();
    }

    /**
     * delete variable
     * @param workspace 
     * @param path 
     */
    public deleteVariable(workspace: string, path: string, _options?: Configuration): Promise<string> {
        const result = this.api.deleteVariable(workspace, path, _options);
        return result.toPromise();
    }

    /**
     * get variable
     * @param workspace 
     * @param path 
     * @param decryptSecret ask to decrypt secret if this variable is secret (if not secret no effect, default: true) 
     */
    public getVariable(workspace: string, path: string, decryptSecret?: boolean, _options?: Configuration): Promise<ListableVariable> {
        const result = this.api.getVariable(workspace, path, decryptSecret, _options);
        return result.toPromise();
    }

    /**
     * list contextual variables
     * @param workspace 
     */
    public listContextualVariables(workspace: string, _options?: Configuration): Promise<Array<ContextualVariable>> {
        const result = this.api.listContextualVariables(workspace, _options);
        return result.toPromise();
    }

    /**
     * list variables
     * @param workspace 
     */
    public listVariable(workspace: string, _options?: Configuration): Promise<Array<ListableVariable>> {
        const result = this.api.listVariable(workspace, _options);
        return result.toPromise();
    }

    /**
     * update variable
     * @param workspace 
     * @param path 
     * @param editVariable updated variable
     */
    public updateVariable(workspace: string, path: string, editVariable: EditVariable, _options?: Configuration): Promise<string> {
        const result = this.api.updateVariable(workspace, path, editVariable, _options);
        return result.toPromise();
    }


}



import { ObservableWorkerApi } from './ObservableAPI.ts';

import { WorkerApiRequestFactory, WorkerApiResponseProcessor} from "../apis/WorkerApi.ts";
export class PromiseWorkerApi {
    private api: ObservableWorkerApi

    public constructor(
        configuration: Configuration,
        requestFactory?: WorkerApiRequestFactory,
        responseProcessor?: WorkerApiResponseProcessor
    ) {
        this.api = new ObservableWorkerApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * list workers
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     */
    public listWorkers(page?: number, perPage?: number, _options?: Configuration): Promise<Array<WorkerPing>> {
        const result = this.api.listWorkers(page, perPage, _options);
        return result.toPromise();
    }


}



import { ObservableWorkspaceApi } from './ObservableAPI.ts';

import { WorkspaceApiRequestFactory, WorkspaceApiResponseProcessor} from "../apis/WorkspaceApi.ts";
export class PromiseWorkspaceApi {
    private api: ObservableWorkspaceApi

    public constructor(
        configuration: Configuration,
        requestFactory?: WorkspaceApiRequestFactory,
        responseProcessor?: WorkspaceApiResponseProcessor
    ) {
        this.api = new ObservableWorkspaceApi(configuration, requestFactory, responseProcessor);
    }

    /**
     * create workspace
     * @param createWorkspace new token
     */
    public createWorkspace(createWorkspace: CreateWorkspace, _options?: Configuration): Promise<string> {
        const result = this.api.createWorkspace(createWorkspace, _options);
        return result.toPromise();
    }

    /**
     * delete user invite
     * @param workspace 
     * @param inlineObject8 
     */
    public deleteInvite(workspace: string, inlineObject8: InlineObject8, _options?: Configuration): Promise<string> {
        const result = this.api.deleteInvite(workspace, inlineObject8, _options);
        return result.toPromise();
    }

    /**
     * delete workspace
     * @param workspace 
     */
    public deleteWorkspace(workspace: string, _options?: Configuration): Promise<string> {
        const result = this.api.deleteWorkspace(workspace, _options);
        return result.toPromise();
    }

    /**
     * disconnect client
     * @param workspace 
     * @param clientName 
     */
    public disconnectClient(workspace: string, clientName: string, _options?: Configuration): Promise<string> {
        const result = this.api.disconnectClient(workspace, clientName, _options);
        return result.toPromise();
    }

    /**
     * edit slack command
     * @param workspace 
     * @param inlineObject9 
     */
    public editSlackCommand(workspace: string, inlineObject9: InlineObject9, _options?: Configuration): Promise<string> {
        const result = this.api.editSlackCommand(workspace, inlineObject9, _options);
        return result.toPromise();
    }

    /**
     * get settings
     * @param workspace 
     */
    public getSettings(workspace: string, _options?: Configuration): Promise<InlineResponse200> {
        const result = this.api.getSettings(workspace, _options);
        return result.toPromise();
    }

    /**
     * invite user to workspace
     * @param workspace 
     * @param inlineObject7 
     */
    public inviteUser(workspace: string, inlineObject7: InlineObject7, _options?: Configuration): Promise<string> {
        const result = this.api.inviteUser(workspace, inlineObject7, _options);
        return result.toPromise();
    }

    /**
     * list pending invites for a workspace
     * @param workspace 
     */
    public listPendingInvites(workspace: string, _options?: Configuration): Promise<Array<WorkspaceInvite>> {
        const result = this.api.listPendingInvites(workspace, _options);
        return result.toPromise();
    }

    /**
     * list all workspaces visible to me with user info
     */
    public listUserWorkspaces(_options?: Configuration): Promise<UserWorkspaceList> {
        const result = this.api.listUserWorkspaces(_options);
        return result.toPromise();
    }

    /**
     * list all workspaces visible to me
     */
    public listWorkspaces(_options?: Configuration): Promise<Array<Workspace>> {
        const result = this.api.listWorkspaces(_options);
        return result.toPromise();
    }

    /**
     * list all workspaces as super admin (require to be super amdin)
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     */
    public listWorkspacesAsSuperAdmin(page?: number, perPage?: number, _options?: Configuration): Promise<Array<Workspace>> {
        const result = this.api.listWorkspacesAsSuperAdmin(page, perPage, _options);
        return result.toPromise();
    }

    /**
     * validate id
     * @param inlineObject3 
     */
    public validateId(inlineObject3: InlineObject3, _options?: Configuration): Promise<string> {
        const result = this.api.validateId(inlineObject3, _options);
        return result.toPromise();
    }

    /**
     * validate username
     * @param inlineObject4 
     */
    public validateUsername(inlineObject4: InlineObject4, _options?: Configuration): Promise<string> {
        const result = this.api.validateUsername(inlineObject4, _options);
        return result.toPromise();
    }


}



