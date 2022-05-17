import { ResponseContext, RequestContext, HttpFile } from '../http/http.ts';
import * as models from '../models/all.ts';
import { Configuration} from '../configuration.ts'
import { Observable, of, from } from '../rxjsStub.ts';
import {mergeMap, map} from  '../rxjsStub.ts';
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

import { AdminApiRequestFactory, AdminApiResponseProcessor} from "../apis/AdminApi.ts";
export class ObservableAdminApi {
    private requestFactory: AdminApiRequestFactory;
    private responseProcessor: AdminApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: AdminApiRequestFactory,
        responseProcessor?: AdminApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new AdminApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new AdminApiResponseProcessor();
    }

    /**
     * create user (require admin privilege)
     * @param workspace 
     * @param newUser new user
     */
    public createUser(workspace: string, newUser: NewUser, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createUser(workspace, newUser, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createUser(rsp)));
            }));
    }

    /**
     * delete user (require admin privilege)
     * @param workspace 
     * @param username 
     */
    public deleteUser(workspace: string, username: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.deleteUser(workspace, username, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.deleteUser(rsp)));
            }));
    }

    /**
     * update user (require admin privilege)
     * @param workspace 
     * @param username 
     * @param editWorkspaceUser new user
     */
    public updateUser(workspace: string, username: string, editWorkspaceUser: EditWorkspaceUser, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.updateUser(workspace, username, editWorkspaceUser, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.updateUser(rsp)));
            }));
    }

}

import { AuditApiRequestFactory, AuditApiResponseProcessor} from "../apis/AuditApi.ts";
export class ObservableAuditApi {
    private requestFactory: AuditApiRequestFactory;
    private responseProcessor: AuditApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: AuditApiRequestFactory,
        responseProcessor?: AuditApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new AuditApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new AuditApiResponseProcessor();
    }

    /**
     * get audit log (requires admin privilege)
     * @param workspace 
     * @param id 
     */
    public getAuditLog(workspace: string, id: number, _options?: Configuration): Observable<AuditLog> {
        const requestContextPromise = this.requestFactory.getAuditLog(workspace, id, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getAuditLog(rsp)));
            }));
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
    public listAuditLogs(workspace: string, page?: number, perPage?: number, before?: Date, after?: Date, username?: string, operation?: string, resource?: string, actionKind?: 'Create' | 'Update' | 'Delete' | 'Execute', _options?: Configuration): Observable<Array<AuditLog>> {
        const requestContextPromise = this.requestFactory.listAuditLogs(workspace, page, perPage, before, after, username, operation, resource, actionKind, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listAuditLogs(rsp)));
            }));
    }

}

import { FlowApiRequestFactory, FlowApiResponseProcessor} from "../apis/FlowApi.ts";
export class ObservableFlowApi {
    private requestFactory: FlowApiRequestFactory;
    private responseProcessor: FlowApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: FlowApiRequestFactory,
        responseProcessor?: FlowApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new FlowApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new FlowApiResponseProcessor();
    }

    /**
     * archive flow by path
     * @param workspace 
     * @param path 
     */
    public archiveFlowByPath(workspace: string, path: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.archiveFlowByPath(workspace, path, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.archiveFlowByPath(rsp)));
            }));
    }

    /**
     * create flow
     * @param workspace 
     * @param inlineObject11 
     */
    public createFlow(workspace: string, inlineObject11: InlineObject11, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createFlow(workspace, inlineObject11, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createFlow(rsp)));
            }));
    }

    /**
     * get flow by path
     * @param workspace 
     * @param path 
     */
    public getFlowByPath(workspace: string, path: string, _options?: Configuration): Observable<Flow> {
        const requestContextPromise = this.requestFactory.getFlowByPath(workspace, path, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getFlowByPath(rsp)));
            }));
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
    public listFlows(workspace: string, page?: number, perPage?: number, orderDesc?: boolean, createdBy?: string, pathStart?: string, pathExact?: string, showArchived?: boolean, _options?: Configuration): Observable<Array<Flow>> {
        const requestContextPromise = this.requestFactory.listFlows(workspace, page, perPage, orderDesc, createdBy, pathStart, pathExact, showArchived, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listFlows(rsp)));
            }));
    }

    /**
     * update flow
     * @param workspace 
     * @param path 
     * @param inlineObject12 
     */
    public updateFlow(workspace: string, path: string, inlineObject12: InlineObject12, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.updateFlow(workspace, path, inlineObject12, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.updateFlow(rsp)));
            }));
    }

}

import { GranularAclApiRequestFactory, GranularAclApiResponseProcessor} from "../apis/GranularAclApi.ts";
export class ObservableGranularAclApi {
    private requestFactory: GranularAclApiRequestFactory;
    private responseProcessor: GranularAclApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: GranularAclApiRequestFactory,
        responseProcessor?: GranularAclApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new GranularAclApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new GranularAclApiResponseProcessor();
    }

    /**
     * add granular acls
     * @param workspace 
     * @param path 
     * @param kind 
     * @param inlineObject20 
     */
    public addGranularAcls(workspace: string, path: string, kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow', inlineObject20: InlineObject20, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.addGranularAcls(workspace, path, kind, inlineObject20, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.addGranularAcls(rsp)));
            }));
    }

    /**
     * get granular acls
     * @param workspace 
     * @param path 
     * @param kind 
     */
    public getGranularAcls(workspace: string, path: string, kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow', _options?: Configuration): Observable<{ [key: string]: boolean; }> {
        const requestContextPromise = this.requestFactory.getGranularAcls(workspace, path, kind, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getGranularAcls(rsp)));
            }));
    }

    /**
     * remove granular acls
     * @param workspace 
     * @param path 
     * @param kind 
     * @param inlineObject21 
     */
    public removeGranularAcls(workspace: string, path: string, kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow', inlineObject21: InlineObject21, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.removeGranularAcls(workspace, path, kind, inlineObject21, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.removeGranularAcls(rsp)));
            }));
    }

}

import { GroupApiRequestFactory, GroupApiResponseProcessor} from "../apis/GroupApi.ts";
export class ObservableGroupApi {
    private requestFactory: GroupApiRequestFactory;
    private responseProcessor: GroupApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: GroupApiRequestFactory,
        responseProcessor?: GroupApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new GroupApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new GroupApiResponseProcessor();
    }

    /**
     * add user to group
     * @param workspace 
     * @param name 
     * @param inlineObject18 
     */
    public addUserToGroup(workspace: string, name: string, inlineObject18: InlineObject18, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.addUserToGroup(workspace, name, inlineObject18, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.addUserToGroup(rsp)));
            }));
    }

    /**
     * create group
     * @param workspace 
     * @param inlineObject16 
     */
    public createGroup(workspace: string, inlineObject16: InlineObject16, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createGroup(workspace, inlineObject16, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createGroup(rsp)));
            }));
    }

    /**
     * delete group
     * @param workspace 
     * @param name 
     */
    public deleteGroup(workspace: string, name: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.deleteGroup(workspace, name, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.deleteGroup(rsp)));
            }));
    }

    /**
     * get group
     * @param workspace 
     * @param name 
     */
    public getGroup(workspace: string, name: string, _options?: Configuration): Observable<Group> {
        const requestContextPromise = this.requestFactory.getGroup(workspace, name, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getGroup(rsp)));
            }));
    }

    /**
     * list group names
     * @param workspace 
     */
    public listGroupNames(workspace: string, _options?: Configuration): Observable<Array<string>> {
        const requestContextPromise = this.requestFactory.listGroupNames(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listGroupNames(rsp)));
            }));
    }

    /**
     * list groups
     * @param workspace 
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     */
    public listGroups(workspace: string, page?: number, perPage?: number, _options?: Configuration): Observable<Array<Group>> {
        const requestContextPromise = this.requestFactory.listGroups(workspace, page, perPage, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listGroups(rsp)));
            }));
    }

    /**
     * remove user to group
     * @param workspace 
     * @param name 
     * @param inlineObject19 
     */
    public removeUserToGroup(workspace: string, name: string, inlineObject19: InlineObject19, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.removeUserToGroup(workspace, name, inlineObject19, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.removeUserToGroup(rsp)));
            }));
    }

    /**
     * update group
     * @param workspace 
     * @param name 
     * @param inlineObject17 
     */
    public updateGroup(workspace: string, name: string, inlineObject17: InlineObject17, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.updateGroup(workspace, name, inlineObject17, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.updateGroup(rsp)));
            }));
    }

}

import { JobApiRequestFactory, JobApiResponseProcessor} from "../apis/JobApi.ts";
export class ObservableJobApi {
    private requestFactory: JobApiRequestFactory;
    private responseProcessor: JobApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: JobApiRequestFactory,
        responseProcessor?: JobApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new JobApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new JobApiResponseProcessor();
    }

    /**
     * cancel queued job
     * @param workspace 
     * @param id 
     * @param inlineObject13 
     */
    public cancelQueuedJob(workspace: string, id: string, inlineObject13: InlineObject13, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.cancelQueuedJob(workspace, id, inlineObject13, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.cancelQueuedJob(rsp)));
            }));
    }

    /**
     * delete completed job (erase content but keep run id)
     * @param workspace 
     * @param id 
     */
    public deleteCompletedJob(workspace: string, id: string, _options?: Configuration): Observable<CompletedJob> {
        const requestContextPromise = this.requestFactory.deleteCompletedJob(workspace, id, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.deleteCompletedJob(rsp)));
            }));
    }

    /**
     * get completed job
     * @param workspace 
     * @param id 
     */
    public getCompletedJob(workspace: string, id: string, _options?: Configuration): Observable<CompletedJob> {
        const requestContextPromise = this.requestFactory.getCompletedJob(workspace, id, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getCompletedJob(rsp)));
            }));
    }

    /**
     * get job
     * @param workspace 
     * @param id 
     */
    public getJob(workspace: string, id: string, _options?: Configuration): Observable<Job> {
        const requestContextPromise = this.requestFactory.getJob(workspace, id, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getJob(rsp)));
            }));
    }

    /**
     * get job updates
     * @param workspace 
     * @param id 
     * @param running 
     * @param logOffset 
     */
    public getJobUpdates(workspace: string, id: string, running?: boolean, logOffset?: number, _options?: Configuration): Observable<InlineResponse2002> {
        const requestContextPromise = this.requestFactory.getJobUpdates(workspace, id, running, logOffset, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getJobUpdates(rsp)));
            }));
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
    public listCompletedJobs(workspace: string, orderDesc?: boolean, createdBy?: string, parentJob?: string, scriptPathExact?: string, scriptPathStart?: string, scriptHash?: string, createdBefore?: Date, createdAfter?: Date, success?: boolean, jobKinds?: string, _options?: Configuration): Observable<Array<CompletedJob>> {
        const requestContextPromise = this.requestFactory.listCompletedJobs(workspace, orderDesc, createdBy, parentJob, scriptPathExact, scriptPathStart, scriptHash, createdBefore, createdAfter, success, jobKinds, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listCompletedJobs(rsp)));
            }));
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
    public listJobs(workspace: string, createdBy?: string, parentJob?: string, scriptPathExact?: string, scriptPathStart?: string, scriptHash?: string, createdBefore?: Date, createdAfter?: Date, jobKinds?: string, success?: boolean, _options?: Configuration): Observable<Array<Job>> {
        const requestContextPromise = this.requestFactory.listJobs(workspace, createdBy, parentJob, scriptPathExact, scriptPathStart, scriptHash, createdBefore, createdAfter, jobKinds, success, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listJobs(rsp)));
            }));
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
    public listQueue(workspace: string, orderDesc?: boolean, createdBy?: string, parentJob?: string, scriptPathExact?: string, scriptPathStart?: string, scriptHash?: string, createdBefore?: Date, createdAfter?: Date, success?: boolean, jobKinds?: string, _options?: Configuration): Observable<Array<QueuedJob>> {
        const requestContextPromise = this.requestFactory.listQueue(workspace, orderDesc, createdBy, parentJob, scriptPathExact, scriptPathStart, scriptHash, createdBefore, createdAfter, success, jobKinds, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listQueue(rsp)));
            }));
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
    public runFlowByPath(workspace: string, path: string, requestBody: { [key: string]: any; }, scheduledFor?: Date, scheduledInSecs?: number, parentJob?: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.runFlowByPath(workspace, path, requestBody, scheduledFor, scheduledInSecs, parentJob, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.runFlowByPath(rsp)));
            }));
    }

    /**
     * run flow preview
     * @param workspace 
     * @param flowPreview preview
     */
    public runFlowPreview(workspace: string, flowPreview: FlowPreview, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.runFlowPreview(workspace, flowPreview, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.runFlowPreview(rsp)));
            }));
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
    public runScriptByHash(workspace: string, hash: string, body: any, scheduledFor?: Date, scheduledInSecs?: number, parentJob?: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.runScriptByHash(workspace, hash, body, scheduledFor, scheduledInSecs, parentJob, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.runScriptByHash(rsp)));
            }));
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
    public runScriptByPath(workspace: string, path: string, requestBody: { [key: string]: any; }, scheduledFor?: Date, scheduledInSecs?: number, parentJob?: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.runScriptByPath(workspace, path, requestBody, scheduledFor, scheduledInSecs, parentJob, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.runScriptByPath(rsp)));
            }));
    }

    /**
     * run script preview
     * @param workspace 
     * @param preview previw
     */
    public runScriptPreview(workspace: string, preview: Preview, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.runScriptPreview(workspace, preview, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.runScriptPreview(rsp)));
            }));
    }

}

import { ResourceApiRequestFactory, ResourceApiResponseProcessor} from "../apis/ResourceApi.ts";
export class ObservableResourceApi {
    private requestFactory: ResourceApiRequestFactory;
    private responseProcessor: ResourceApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: ResourceApiRequestFactory,
        responseProcessor?: ResourceApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new ResourceApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new ResourceApiResponseProcessor();
    }

    /**
     * create resource
     * @param workspace 
     * @param createResource new resource
     */
    public createResource(workspace: string, createResource: CreateResource, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createResource(workspace, createResource, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createResource(rsp)));
            }));
    }

    /**
     * create resource_type
     * @param workspace 
     * @param resourceType new resource_type
     */
    public createResourceType(workspace: string, resourceType: ResourceType, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createResourceType(workspace, resourceType, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createResourceType(rsp)));
            }));
    }

    /**
     * delete resource
     * @param workspace 
     * @param path 
     */
    public deleteResource(workspace: string, path: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.deleteResource(workspace, path, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.deleteResource(rsp)));
            }));
    }

    /**
     * delete resource_type
     * @param workspace 
     * @param path 
     */
    public deleteResourceType(workspace: string, path: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.deleteResourceType(workspace, path, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.deleteResourceType(rsp)));
            }));
    }

    /**
     * get resource
     * @param workspace 
     * @param path 
     */
    public getResource(workspace: string, path: string, _options?: Configuration): Observable<Resource> {
        const requestContextPromise = this.requestFactory.getResource(workspace, path, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getResource(rsp)));
            }));
    }

    /**
     * get resource_type
     * @param workspace 
     * @param path 
     */
    public getResourceType(workspace: string, path: string, _options?: Configuration): Observable<ResourceType> {
        const requestContextPromise = this.requestFactory.getResourceType(workspace, path, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getResourceType(rsp)));
            }));
    }

    /**
     * list resources
     * @param workspace 
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     * @param resourceType resource_type to list from
     */
    public listResource(workspace: string, page?: number, perPage?: number, resourceType?: string, _options?: Configuration): Observable<Array<Resource>> {
        const requestContextPromise = this.requestFactory.listResource(workspace, page, perPage, resourceType, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listResource(rsp)));
            }));
    }

    /**
     * list resource_types
     * @param workspace 
     */
    public listResourceType(workspace: string, _options?: Configuration): Observable<Array<ResourceType>> {
        const requestContextPromise = this.requestFactory.listResourceType(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listResourceType(rsp)));
            }));
    }

    /**
     * list resource_types names
     * @param workspace 
     */
    public listResourceTypeNames(workspace: string, _options?: Configuration): Observable<Array<string>> {
        const requestContextPromise = this.requestFactory.listResourceTypeNames(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listResourceTypeNames(rsp)));
            }));
    }

    /**
     * update resource
     * @param workspace 
     * @param path 
     * @param editResource updated resource
     */
    public updateResource(workspace: string, path: string, editResource: EditResource, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.updateResource(workspace, path, editResource, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.updateResource(rsp)));
            }));
    }

    /**
     * update resource_type
     * @param workspace 
     * @param path 
     * @param editResourceType updated resource_type
     */
    public updateResourceType(workspace: string, path: string, editResourceType: EditResourceType, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.updateResourceType(workspace, path, editResourceType, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.updateResourceType(rsp)));
            }));
    }

}

import { ScheduleApiRequestFactory, ScheduleApiResponseProcessor} from "../apis/ScheduleApi.ts";
export class ObservableScheduleApi {
    private requestFactory: ScheduleApiRequestFactory;
    private responseProcessor: ScheduleApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: ScheduleApiRequestFactory,
        responseProcessor?: ScheduleApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new ScheduleApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new ScheduleApiResponseProcessor();
    }

    /**
     * create schedule
     * @param workspace 
     * @param newSchedule new schedule
     */
    public createSchedule(workspace: string, newSchedule: NewSchedule, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createSchedule(workspace, newSchedule, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createSchedule(rsp)));
            }));
    }

    /**
     * get schedule
     * @param workspace 
     * @param path 
     */
    public getSchedule(workspace: string, path: string, _options?: Configuration): Observable<Schedule> {
        const requestContextPromise = this.requestFactory.getSchedule(workspace, path, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getSchedule(rsp)));
            }));
    }

    /**
     * list schedules
     * @param workspace 
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     */
    public listSchedules(workspace: string, page?: number, perPage?: number, _options?: Configuration): Observable<Array<Schedule>> {
        const requestContextPromise = this.requestFactory.listSchedules(workspace, page, perPage, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listSchedules(rsp)));
            }));
    }

    /**
     * preview schedule
     * @param inlineObject14 
     */
    public previewSchedule(inlineObject14: InlineObject14, _options?: Configuration): Observable<Array<Date>> {
        const requestContextPromise = this.requestFactory.previewSchedule(inlineObject14, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.previewSchedule(rsp)));
            }));
    }

    /**
     * set enabled schedule
     * @param workspace 
     * @param path 
     * @param inlineObject15 
     */
    public setScheduleEnabled(workspace: string, path: string, inlineObject15: InlineObject15, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.setScheduleEnabled(workspace, path, inlineObject15, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.setScheduleEnabled(rsp)));
            }));
    }

    /**
     * update schedule
     * @param workspace 
     * @param path 
     * @param editSchedule updated schedule
     */
    public updateSchedule(workspace: string, path: string, editSchedule: EditSchedule, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.updateSchedule(workspace, path, editSchedule, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.updateSchedule(rsp)));
            }));
    }

}

import { ScriptApiRequestFactory, ScriptApiResponseProcessor} from "../apis/ScriptApi.ts";
export class ObservableScriptApi {
    private requestFactory: ScriptApiRequestFactory;
    private responseProcessor: ScriptApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: ScriptApiRequestFactory,
        responseProcessor?: ScriptApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new ScriptApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new ScriptApiResponseProcessor();
    }

    /**
     * archive script by hash
     * @param workspace 
     * @param hash 
     */
    public archiveScriptByHash(workspace: string, hash: string, _options?: Configuration): Observable<Script> {
        const requestContextPromise = this.requestFactory.archiveScriptByHash(workspace, hash, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.archiveScriptByHash(rsp)));
            }));
    }

    /**
     * archive script by path
     * @param workspace 
     * @param path 
     */
    public archiveScriptByPath(workspace: string, path: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.archiveScriptByPath(workspace, path, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.archiveScriptByPath(rsp)));
            }));
    }

    /**
     * create script
     * @param workspace 
     * @param inlineObject10 
     */
    public createScript(workspace: string, inlineObject10: InlineObject10, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createScript(workspace, inlineObject10, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createScript(rsp)));
            }));
    }

    /**
     * delete script by hash (erase content but keep hash)
     * @param workspace 
     * @param hash 
     */
    public deleteScriptByHash(workspace: string, hash: string, _options?: Configuration): Observable<Script> {
        const requestContextPromise = this.requestFactory.deleteScriptByHash(workspace, hash, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.deleteScriptByHash(rsp)));
            }));
    }

    /**
     * inspect deno code to infer jsonschema of arguments
     * @param body deno code with the main function
     */
    public denoToJsonschema(body: string, _options?: Configuration): Observable<MainArgSignature> {
        const requestContextPromise = this.requestFactory.denoToJsonschema(body, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.denoToJsonschema(rsp)));
            }));
    }

    /**
     * get script by hash
     * @param workspace 
     * @param hash 
     */
    public getScriptByHash(workspace: string, hash: string, _options?: Configuration): Observable<Script> {
        const requestContextPromise = this.requestFactory.getScriptByHash(workspace, hash, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getScriptByHash(rsp)));
            }));
    }

    /**
     * get script by path
     * @param workspace 
     * @param path 
     */
    public getScriptByPath(workspace: string, path: string, _options?: Configuration): Observable<Script> {
        const requestContextPromise = this.requestFactory.getScriptByPath(workspace, path, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getScriptByPath(rsp)));
            }));
    }

    /**
     * get script deployment status
     * @param workspace 
     * @param hash 
     */
    public getScriptDeploymentStatus(workspace: string, hash: string, _options?: Configuration): Observable<InlineResponse2001> {
        const requestContextPromise = this.requestFactory.getScriptDeploymentStatus(workspace, hash, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getScriptDeploymentStatus(rsp)));
            }));
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
    public listScripts(workspace: string, page?: number, perPage?: number, orderDesc?: boolean, createdBy?: string, pathStart?: string, pathExact?: string, firstParentHash?: string, lastParentHash?: string, parentHash?: string, showArchived?: boolean, isTemplate?: boolean, _options?: Configuration): Observable<Array<Script>> {
        const requestContextPromise = this.requestFactory.listScripts(workspace, page, perPage, orderDesc, createdBy, pathStart, pathExact, firstParentHash, lastParentHash, parentHash, showArchived, isTemplate, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listScripts(rsp)));
            }));
    }

    /**
     * inspect python code to infer jsonschema of arguments
     * @param body python code with the main function
     */
    public pythonToJsonschema(body: string, _options?: Configuration): Observable<MainArgSignature> {
        const requestContextPromise = this.requestFactory.pythonToJsonschema(body, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.pythonToJsonschema(rsp)));
            }));
    }

}

import { SettingsApiRequestFactory, SettingsApiResponseProcessor} from "../apis/SettingsApi.ts";
export class ObservableSettingsApi {
    private requestFactory: SettingsApiRequestFactory;
    private responseProcessor: SettingsApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: SettingsApiRequestFactory,
        responseProcessor?: SettingsApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new SettingsApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new SettingsApiResponseProcessor();
    }

    /**
     * get backend version
     */
    public backendVersion(_options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.backendVersion(_options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.backendVersion(rsp)));
            }));
    }

    /**
     * get openapi yaml spec
     */
    public getOpenApiYaml(_options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.getOpenApiYaml(_options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getOpenApiYaml(rsp)));
            }));
    }

}

import { UserApiRequestFactory, UserApiResponseProcessor} from "../apis/UserApi.ts";
export class ObservableUserApi {
    private requestFactory: UserApiRequestFactory;
    private responseProcessor: UserApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: UserApiRequestFactory,
        responseProcessor?: UserApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new UserApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new UserApiResponseProcessor();
    }

    /**
     * accept invite to workspace
     * @param inlineObject5 
     */
    public acceptInvite(inlineObject5: InlineObject5, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.acceptInvite(inlineObject5, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.acceptInvite(rsp)));
            }));
    }

    /**
     * create token
     * @param newToken new token
     */
    public createToken(newToken: NewToken, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createToken(newToken, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createToken(rsp)));
            }));
    }

    /**
     * create user (require admin privilege)
     * @param workspace 
     * @param newUser new user
     */
    public createUser(workspace: string, newUser: NewUser, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createUser(workspace, newUser, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createUser(rsp)));
            }));
    }

    /**
     * create user
     * @param inlineObject1 
     */
    public createUserGlobally(inlineObject1: InlineObject1, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createUserGlobally(inlineObject1, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createUserGlobally(rsp)));
            }));
    }

    /**
     * decline invite to workspace
     * @param inlineObject6 
     */
    public declineInvite(inlineObject6: InlineObject6, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.declineInvite(inlineObject6, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.declineInvite(rsp)));
            }));
    }

    /**
     * delete token
     * @param tokenPrefix 
     */
    public deleteToken(tokenPrefix: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.deleteToken(tokenPrefix, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.deleteToken(rsp)));
            }));
    }

    /**
     * delete user (require admin privilege)
     * @param workspace 
     * @param username 
     */
    public deleteUser(workspace: string, username: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.deleteUser(workspace, username, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.deleteUser(rsp)));
            }));
    }

    /**
     * get current user email (if logged in)
     */
    public getCurrentEmail(_options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.getCurrentEmail(_options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getCurrentEmail(rsp)));
            }));
    }

    /**
     * global update user (require super admin)
     * @param email 
     * @param inlineObject2 
     */
    public globalUserUpdate(email: string, inlineObject2: InlineObject2, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.globalUserUpdate(email, inlineObject2, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.globalUserUpdate(rsp)));
            }));
    }

    /**
     * get current global whoami (if logged in)
     */
    public globalWhoami(_options?: Configuration): Observable<GlobalUserInfo> {
        const requestContextPromise = this.requestFactory.globalWhoami(_options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.globalWhoami(rsp)));
            }));
    }

    /**
     * leave workspace
     * @param workspace 
     */
    public leaveWorkspace(workspace: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.leaveWorkspace(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.leaveWorkspace(rsp)));
            }));
    }

    /**
     * list token
     */
    public listTokens(_options?: Configuration): Observable<Array<TruncatedToken>> {
        const requestContextPromise = this.requestFactory.listTokens(_options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listTokens(rsp)));
            }));
    }

    /**
     * list usernames
     * @param workspace 
     */
    public listUsernames(workspace: string, _options?: Configuration): Observable<Array<string>> {
        const requestContextPromise = this.requestFactory.listUsernames(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listUsernames(rsp)));
            }));
    }

    /**
     * list users
     * @param workspace 
     */
    public listUsers(workspace: string, _options?: Configuration): Observable<Array<User>> {
        const requestContextPromise = this.requestFactory.listUsers(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listUsers(rsp)));
            }));
    }

    /**
     * list all users as super admin (require to be super amdin)
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     */
    public listUsersAsSuperAdmin(page?: number, perPage?: number, _options?: Configuration): Observable<Array<GlobalUserInfo>> {
        const requestContextPromise = this.requestFactory.listUsersAsSuperAdmin(page, perPage, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listUsersAsSuperAdmin(rsp)));
            }));
    }

    /**
     * list all workspace invites
     */
    public listWorkspaceInvites(_options?: Configuration): Observable<Array<WorkspaceInvite>> {
        const requestContextPromise = this.requestFactory.listWorkspaceInvites(_options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listWorkspaceInvites(rsp)));
            }));
    }

    /**
     * login with password
     * @param login Partially filled script
     */
    public login(login: Login, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.login(login, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.login(rsp)));
            }));
    }

    /**
     * logout
     */
    public logout(_options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.logout(_options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.logout(rsp)));
            }));
    }

    /**
     * set password
     * @param inlineObject 
     */
    public setPassword(inlineObject: InlineObject, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.setPassword(inlineObject, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.setPassword(rsp)));
            }));
    }

    /**
     * update user (require admin privilege)
     * @param workspace 
     * @param username 
     * @param editWorkspaceUser new user
     */
    public updateUser(workspace: string, username: string, editWorkspaceUser: EditWorkspaceUser, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.updateUser(workspace, username, editWorkspaceUser, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.updateUser(rsp)));
            }));
    }

    /**
     * whoami
     * @param workspace 
     */
    public whoami(workspace: string, _options?: Configuration): Observable<User> {
        const requestContextPromise = this.requestFactory.whoami(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.whoami(rsp)));
            }));
    }

    /**
     * whois
     * @param workspace 
     * @param username 
     */
    public whois(workspace: string, username: string, _options?: Configuration): Observable<User> {
        const requestContextPromise = this.requestFactory.whois(workspace, username, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.whois(rsp)));
            }));
    }

}

import { VariableApiRequestFactory, VariableApiResponseProcessor} from "../apis/VariableApi.ts";
export class ObservableVariableApi {
    private requestFactory: VariableApiRequestFactory;
    private responseProcessor: VariableApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: VariableApiRequestFactory,
        responseProcessor?: VariableApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new VariableApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new VariableApiResponseProcessor();
    }

    /**
     * create variable
     * @param workspace 
     * @param createVariable new variable
     */
    public createVariable(workspace: string, createVariable: CreateVariable, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createVariable(workspace, createVariable, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createVariable(rsp)));
            }));
    }

    /**
     * delete variable
     * @param workspace 
     * @param path 
     */
    public deleteVariable(workspace: string, path: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.deleteVariable(workspace, path, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.deleteVariable(rsp)));
            }));
    }

    /**
     * get variable
     * @param workspace 
     * @param path 
     * @param decryptSecret ask to decrypt secret if this variable is secret (if not secret no effect, default: true) 
     */
    public getVariable(workspace: string, path: string, decryptSecret?: boolean, _options?: Configuration): Observable<ListableVariable> {
        const requestContextPromise = this.requestFactory.getVariable(workspace, path, decryptSecret, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getVariable(rsp)));
            }));
    }

    /**
     * list contextual variables
     * @param workspace 
     */
    public listContextualVariables(workspace: string, _options?: Configuration): Observable<Array<ContextualVariable>> {
        const requestContextPromise = this.requestFactory.listContextualVariables(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listContextualVariables(rsp)));
            }));
    }

    /**
     * list variables
     * @param workspace 
     */
    public listVariable(workspace: string, _options?: Configuration): Observable<Array<ListableVariable>> {
        const requestContextPromise = this.requestFactory.listVariable(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listVariable(rsp)));
            }));
    }

    /**
     * update variable
     * @param workspace 
     * @param path 
     * @param editVariable updated variable
     */
    public updateVariable(workspace: string, path: string, editVariable: EditVariable, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.updateVariable(workspace, path, editVariable, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.updateVariable(rsp)));
            }));
    }

}

import { WorkerApiRequestFactory, WorkerApiResponseProcessor} from "../apis/WorkerApi.ts";
export class ObservableWorkerApi {
    private requestFactory: WorkerApiRequestFactory;
    private responseProcessor: WorkerApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: WorkerApiRequestFactory,
        responseProcessor?: WorkerApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new WorkerApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new WorkerApiResponseProcessor();
    }

    /**
     * list workers
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     */
    public listWorkers(page?: number, perPage?: number, _options?: Configuration): Observable<Array<WorkerPing>> {
        const requestContextPromise = this.requestFactory.listWorkers(page, perPage, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listWorkers(rsp)));
            }));
    }

}

import { WorkspaceApiRequestFactory, WorkspaceApiResponseProcessor} from "../apis/WorkspaceApi.ts";
export class ObservableWorkspaceApi {
    private requestFactory: WorkspaceApiRequestFactory;
    private responseProcessor: WorkspaceApiResponseProcessor;
    private configuration: Configuration;

    public constructor(
        configuration: Configuration,
        requestFactory?: WorkspaceApiRequestFactory,
        responseProcessor?: WorkspaceApiResponseProcessor
    ) {
        this.configuration = configuration;
        this.requestFactory = requestFactory || new WorkspaceApiRequestFactory(configuration);
        this.responseProcessor = responseProcessor || new WorkspaceApiResponseProcessor();
    }

    /**
     * create workspace
     * @param createWorkspace new token
     */
    public createWorkspace(createWorkspace: CreateWorkspace, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.createWorkspace(createWorkspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.createWorkspace(rsp)));
            }));
    }

    /**
     * delete user invite
     * @param workspace 
     * @param inlineObject8 
     */
    public deleteInvite(workspace: string, inlineObject8: InlineObject8, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.deleteInvite(workspace, inlineObject8, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.deleteInvite(rsp)));
            }));
    }

    /**
     * delete workspace
     * @param workspace 
     */
    public deleteWorkspace(workspace: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.deleteWorkspace(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.deleteWorkspace(rsp)));
            }));
    }

    /**
     * disconnect client
     * @param workspace 
     * @param clientName 
     */
    public disconnectClient(workspace: string, clientName: string, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.disconnectClient(workspace, clientName, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.disconnectClient(rsp)));
            }));
    }

    /**
     * edit slack command
     * @param workspace 
     * @param inlineObject9 
     */
    public editSlackCommand(workspace: string, inlineObject9: InlineObject9, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.editSlackCommand(workspace, inlineObject9, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.editSlackCommand(rsp)));
            }));
    }

    /**
     * get settings
     * @param workspace 
     */
    public getSettings(workspace: string, _options?: Configuration): Observable<InlineResponse200> {
        const requestContextPromise = this.requestFactory.getSettings(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.getSettings(rsp)));
            }));
    }

    /**
     * invite user to workspace
     * @param workspace 
     * @param inlineObject7 
     */
    public inviteUser(workspace: string, inlineObject7: InlineObject7, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.inviteUser(workspace, inlineObject7, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.inviteUser(rsp)));
            }));
    }

    /**
     * list pending invites for a workspace
     * @param workspace 
     */
    public listPendingInvites(workspace: string, _options?: Configuration): Observable<Array<WorkspaceInvite>> {
        const requestContextPromise = this.requestFactory.listPendingInvites(workspace, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listPendingInvites(rsp)));
            }));
    }

    /**
     * list all workspaces visible to me with user info
     */
    public listUserWorkspaces(_options?: Configuration): Observable<UserWorkspaceList> {
        const requestContextPromise = this.requestFactory.listUserWorkspaces(_options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listUserWorkspaces(rsp)));
            }));
    }

    /**
     * list all workspaces visible to me
     */
    public listWorkspaces(_options?: Configuration): Observable<Array<Workspace>> {
        const requestContextPromise = this.requestFactory.listWorkspaces(_options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listWorkspaces(rsp)));
            }));
    }

    /**
     * list all workspaces as super admin (require to be super amdin)
     * @param page which page to return (start at 1, default 1)
     * @param perPage number of items to return for a given page (default 30, max 100)
     */
    public listWorkspacesAsSuperAdmin(page?: number, perPage?: number, _options?: Configuration): Observable<Array<Workspace>> {
        const requestContextPromise = this.requestFactory.listWorkspacesAsSuperAdmin(page, perPage, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.listWorkspacesAsSuperAdmin(rsp)));
            }));
    }

    /**
     * validate id
     * @param inlineObject3 
     */
    public validateId(inlineObject3: InlineObject3, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.validateId(inlineObject3, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.validateId(rsp)));
            }));
    }

    /**
     * validate username
     * @param inlineObject4 
     */
    public validateUsername(inlineObject4: InlineObject4, _options?: Configuration): Observable<string> {
        const requestContextPromise = this.requestFactory.validateUsername(inlineObject4, _options);

        // build promise chain
        let middlewarePreObservable = from<RequestContext>(requestContextPromise);
        for (let middleware of this.configuration.middleware) {
            middlewarePreObservable = middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => middleware.pre(ctx)));
        }

        return middlewarePreObservable.pipe(mergeMap((ctx: RequestContext) => this.configuration.httpApi.send(ctx))).
            pipe(mergeMap((response: ResponseContext) => {
                let middlewarePostObservable = of(response);
                for (let middleware of this.configuration.middleware) {
                    middlewarePostObservable = middlewarePostObservable.pipe(mergeMap((rsp: ResponseContext) => middleware.post(rsp)));
                }
                return middlewarePostObservable.pipe(map((rsp: ResponseContext) => this.responseProcessor.validateUsername(rsp)));
            }));
    }

}
