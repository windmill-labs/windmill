export * from './AuditLog.ts';
export * from './CompletedJob.ts';
export * from './ContextualVariable.ts';
export * from './CreateResource.ts';
export * from './CreateVariable.ts';
export * from './CreateWorkspace.ts';
export * from './EditResource.ts';
export * from './EditResourceType.ts';
export * from './EditSchedule.ts';
export * from './EditVariable.ts';
export * from './EditWorkspaceUser.ts';
export * from './Flow.ts';
export * from './FlowModule.ts';
export * from './FlowModuleValue.ts';
export * from './FlowPreview.ts';
export * from './FlowStatus.ts';
export * from './FlowStatusModule.ts';
export * from './FlowValue.ts';
export * from './GlobalUserInfo.ts';
export * from './Group.ts';
export * from './InlineObject.ts';
export * from './InlineObject1.ts';
export * from './InlineObject10.ts';
export * from './InlineObject11.ts';
export * from './InlineObject12.ts';
export * from './InlineObject13.ts';
export * from './InlineObject14.ts';
export * from './InlineObject15.ts';
export * from './InlineObject16.ts';
export * from './InlineObject17.ts';
export * from './InlineObject18.ts';
export * from './InlineObject19.ts';
export * from './InlineObject2.ts';
export * from './InlineObject20.ts';
export * from './InlineObject21.ts';
export * from './InlineObject3.ts';
export * from './InlineObject4.ts';
export * from './InlineObject5.ts';
export * from './InlineObject6.ts';
export * from './InlineObject7.ts';
export * from './InlineObject8.ts';
export * from './InlineObject9.ts';
export * from './InlineResponse200.ts';
export * from './InlineResponse2001.ts';
export * from './InlineResponse2002.ts';
export * from './InputTransform.ts';
export * from './Job.ts';
export * from './JobAllOf.ts';
export * from './ListableVariable.ts';
export * from './Login.ts';
export * from './MainArgSignature.ts';
export * from './MainArgSignatureArgs.ts';
export * from './NewSchedule.ts';
export * from './NewToken.ts';
export * from './NewUser.ts';
export * from './Preview.ts';
export * from './QueuedJob.ts';
export * from './Resource.ts';
export * from './ResourceType.ts';
export * from './Schedule.ts';
export * from './Script.ts';
export * from './TruncatedToken.ts';
export * from './User.ts';
export * from './UserWorkspaceList.ts';
export * from './UserWorkspaceListWorkspaces.ts';
export * from './WorkerPing.ts';
export * from './Workspace.ts';
export * from './WorkspaceInvite.ts';

import { AuditLog   , AuditLogOperationEnum  , AuditLogActionKindEnum     } from './AuditLog.ts';
import { CompletedJob                  , CompletedJobJobKindEnum       , CompletedJobLanguageEnum   } from './CompletedJob.ts';
import { ContextualVariable } from './ContextualVariable.ts';
import { CreateResource } from './CreateResource.ts';
import { CreateVariable } from './CreateVariable.ts';
import { CreateWorkspace } from './CreateWorkspace.ts';
import { EditResource } from './EditResource.ts';
import { EditResourceType } from './EditResourceType.ts';
import { EditSchedule } from './EditSchedule.ts';
import { EditVariable } from './EditVariable.ts';
import { EditWorkspaceUser } from './EditWorkspaceUser.ts';
import { Flow } from './Flow.ts';
import { FlowModule } from './FlowModule.ts';
import { FlowModuleValue , FlowModuleValueTypeEnum   } from './FlowModuleValue.ts';
import { FlowPreview } from './FlowPreview.ts';
import { FlowStatus } from './FlowStatus.ts';
import { FlowStatusModule, FlowStatusModuleTypeEnum     } from './FlowStatusModule.ts';
import { FlowValue } from './FlowValue.ts';
import { GlobalUserInfo , GlobalUserInfoLoginTypeEnum       } from './GlobalUserInfo.ts';
import { Group } from './Group.ts';
import { InlineObject } from './InlineObject.ts';
import { InlineObject1 } from './InlineObject1.ts';
import { InlineObject10        , InlineObject10LanguageEnum   } from './InlineObject10.ts';
import { InlineObject11 } from './InlineObject11.ts';
import { InlineObject12 } from './InlineObject12.ts';
import { InlineObject13 } from './InlineObject13.ts';
import { InlineObject14 } from './InlineObject14.ts';
import { InlineObject15 } from './InlineObject15.ts';
import { InlineObject16 } from './InlineObject16.ts';
import { InlineObject17 } from './InlineObject17.ts';
import { InlineObject18 } from './InlineObject18.ts';
import { InlineObject19 } from './InlineObject19.ts';
import { InlineObject2 } from './InlineObject2.ts';
import { InlineObject20 } from './InlineObject20.ts';
import { InlineObject21 } from './InlineObject21.ts';
import { InlineObject3 } from './InlineObject3.ts';
import { InlineObject4 } from './InlineObject4.ts';
import { InlineObject5 } from './InlineObject5.ts';
import { InlineObject6 } from './InlineObject6.ts';
import { InlineObject7 } from './InlineObject7.ts';
import { InlineObject8 } from './InlineObject8.ts';
import { InlineObject9 } from './InlineObject9.ts';
import { InlineResponse200 } from './InlineResponse200.ts';
import { InlineResponse2001 } from './InlineResponse2001.ts';
import { InlineResponse2002 } from './InlineResponse2002.ts';
import { InputTransform, InputTransformTypeEnum      } from './InputTransform.ts';
import { Job, JobTypeEnum                    , JobJobKindEnum       , JobLanguageEnum      } from './Job.ts';
import { JobAllOf, JobAllOfTypeEnum   } from './JobAllOf.ts';
import { ListableVariable } from './ListableVariable.ts';
import { Login } from './Login.ts';
import { MainArgSignature } from './MainArgSignature.ts';
import { MainArgSignatureArgs , MainArgSignatureArgsTypEnum     } from './MainArgSignatureArgs.ts';
import { NewSchedule } from './NewSchedule.ts';
import { NewToken } from './NewToken.ts';
import { NewUser } from './NewUser.ts';
import { Preview   , PreviewLanguageEnum   } from './Preview.ts';
import { QueuedJob                 , QueuedJobJobKindEnum       , QueuedJobLanguageEnum   } from './QueuedJob.ts';
import { Resource } from './Resource.ts';
import { ResourceType } from './ResourceType.ts';
import { Schedule } from './Schedule.ts';
import { Script                , ScriptLanguageEnum   } from './Script.ts';
import { TruncatedToken } from './TruncatedToken.ts';
import { User } from './User.ts';
import { UserWorkspaceList } from './UserWorkspaceList.ts';
import { UserWorkspaceListWorkspaces } from './UserWorkspaceListWorkspaces.ts';
import { WorkerPing } from './WorkerPing.ts';
import { Workspace } from './Workspace.ts';
import { WorkspaceInvite } from './WorkspaceInvite.ts';

/* tslint:disable:no-unused-variable */
let primitives = [
                    "string",
                    "boolean",
                    "double",
                    "integer",
                    "long",
                    "float",
                    "number",
                    "any"
                 ];

const supportedMediaTypes: { [mediaType: string]: number } = {
  "application/json": Infinity,
  "application/octet-stream": 0,
  "application/x-www-form-urlencoded": 0
}


let enumsMap: Set<string> = new Set<string>([
    "AuditLogOperationEnum",
    "AuditLogActionKindEnum",
    "CompletedJobJobKindEnum",
    "CompletedJobLanguageEnum",
    "FlowModuleValueTypeEnum",
    "FlowStatusModuleTypeEnum",
    "GlobalUserInfoLoginTypeEnum",
    "InlineObject10LanguageEnum",
    "InputTransformTypeEnum",
    "JobTypeEnum",
    "JobJobKindEnum",
    "JobLanguageEnum",
    "JobAllOfTypeEnum",
    "MainArgSignatureArgsTypEnum",
    "PreviewLanguageEnum",
    "QueuedJobJobKindEnum",
    "QueuedJobLanguageEnum",
    "ScriptLanguageEnum",
]);

let typeMap: {[index: string]: any} = {
    "AuditLog": AuditLog,
    "CompletedJob": CompletedJob,
    "ContextualVariable": ContextualVariable,
    "CreateResource": CreateResource,
    "CreateVariable": CreateVariable,
    "CreateWorkspace": CreateWorkspace,
    "EditResource": EditResource,
    "EditResourceType": EditResourceType,
    "EditSchedule": EditSchedule,
    "EditVariable": EditVariable,
    "EditWorkspaceUser": EditWorkspaceUser,
    "Flow": Flow,
    "FlowModule": FlowModule,
    "FlowModuleValue": FlowModuleValue,
    "FlowPreview": FlowPreview,
    "FlowStatus": FlowStatus,
    "FlowStatusModule": FlowStatusModule,
    "FlowValue": FlowValue,
    "GlobalUserInfo": GlobalUserInfo,
    "Group": Group,
    "InlineObject": InlineObject,
    "InlineObject1": InlineObject1,
    "InlineObject10": InlineObject10,
    "InlineObject11": InlineObject11,
    "InlineObject12": InlineObject12,
    "InlineObject13": InlineObject13,
    "InlineObject14": InlineObject14,
    "InlineObject15": InlineObject15,
    "InlineObject16": InlineObject16,
    "InlineObject17": InlineObject17,
    "InlineObject18": InlineObject18,
    "InlineObject19": InlineObject19,
    "InlineObject2": InlineObject2,
    "InlineObject20": InlineObject20,
    "InlineObject21": InlineObject21,
    "InlineObject3": InlineObject3,
    "InlineObject4": InlineObject4,
    "InlineObject5": InlineObject5,
    "InlineObject6": InlineObject6,
    "InlineObject7": InlineObject7,
    "InlineObject8": InlineObject8,
    "InlineObject9": InlineObject9,
    "InlineResponse200": InlineResponse200,
    "InlineResponse2001": InlineResponse2001,
    "InlineResponse2002": InlineResponse2002,
    "InputTransform": InputTransform,
    "Job": Job,
    "JobAllOf": JobAllOf,
    "ListableVariable": ListableVariable,
    "Login": Login,
    "MainArgSignature": MainArgSignature,
    "MainArgSignatureArgs": MainArgSignatureArgs,
    "NewSchedule": NewSchedule,
    "NewToken": NewToken,
    "NewUser": NewUser,
    "Preview": Preview,
    "QueuedJob": QueuedJob,
    "Resource": Resource,
    "ResourceType": ResourceType,
    "Schedule": Schedule,
    "Script": Script,
    "TruncatedToken": TruncatedToken,
    "User": User,
    "UserWorkspaceList": UserWorkspaceList,
    "UserWorkspaceListWorkspaces": UserWorkspaceListWorkspaces,
    "WorkerPing": WorkerPing,
    "Workspace": Workspace,
    "WorkspaceInvite": WorkspaceInvite,
}

export class ObjectSerializer {
    public static findCorrectType(data: any, expectedType: string) {
        if (data == undefined) {
            return expectedType;
        } else if (primitives.indexOf(expectedType.toLowerCase()) !== -1) {
            return expectedType;
        } else if (expectedType === "Date") {
            return expectedType;
        } else {
            if (enumsMap.has(expectedType)) {
                return expectedType;
            }

            if (!typeMap[expectedType]) {
                return expectedType; // w/e we don't know the type
            }

            // Check the discriminator
            let discriminatorProperty = typeMap[expectedType].discriminator;
            if (discriminatorProperty == null) {
                return expectedType; // the type does not have a discriminator. use it.
            } else {
                if (data[discriminatorProperty]) {
                    var discriminatorType = data[discriminatorProperty];
                    if(typeMap[discriminatorType]){
                        return discriminatorType; // use the type given in the discriminator
                    } else {
                        return expectedType; // discriminator did not map to a type
                    }
                } else {
                    return expectedType; // discriminator was not present (or an empty string)
                }
            }
        }
    }

    public static serialize(data: any, type: string, format: string) {
        if (data == undefined) {
            return data;
        } else if (primitives.indexOf(type.toLowerCase()) !== -1) {
            return data;
        } else if (type.lastIndexOf("Array<", 0) === 0) { // string.startsWith pre es6
            let subType: string = type.replace("Array<", ""); // Array<Type> => Type>
            subType = subType.substring(0, subType.length - 1); // Type> => Type
            let transformedData: any[] = [];
            for (let index in data) {
                let date = data[index];
                transformedData.push(ObjectSerializer.serialize(date, subType, format));
            }
            return transformedData;
        } else if (type === "Date") {
            if (format == "date") {
                let month = data.getMonth()+1
                month = month < 10 ? "0" + month.toString() : month.toString()
                let day = data.getDate();
                day = day < 10 ? "0" + day.toString() : day.toString();

                return data.getFullYear() + "-" + month + "-" + day;
            } else {
                return data.toISOString();
            }
        } else {
            if (enumsMap.has(type)) {
                return data;
            }
            if (!typeMap[type]) { // in case we dont know the type
                return data;
            }

            // Get the actual type of this object
            type = this.findCorrectType(data, type);

            // get the map for the correct type.
            let attributeTypes = typeMap[type].getAttributeTypeMap();
            let instance: {[index: string]: any} = {};
            for (let index in attributeTypes) {
                let attributeType = attributeTypes[index];
                instance[attributeType.baseName] = ObjectSerializer.serialize(data[attributeType.name], attributeType.type, attributeType.format);
            }
            return instance;
        }
    }

    public static deserialize(data: any, type: string, format: string) {
        // polymorphism may change the actual type.
        type = ObjectSerializer.findCorrectType(data, type);
        if (data == undefined) {
            return data;
        } else if (primitives.indexOf(type.toLowerCase()) !== -1) {
            return data;
        } else if (type.lastIndexOf("Array<", 0) === 0) { // string.startsWith pre es6
            let subType: string = type.replace("Array<", ""); // Array<Type> => Type>
            subType = subType.substring(0, subType.length - 1); // Type> => Type
            let transformedData: any[] = [];
            for (let index in data) {
                let date = data[index];
                transformedData.push(ObjectSerializer.deserialize(date, subType, format));
            }
            return transformedData;
        } else if (type === "Date") {
            return new Date(data);
        } else {
            if (enumsMap.has(type)) {// is Enum
                return data;
            }

            if (!typeMap[type]) { // dont know the type
                return data;
            }
            let instance = new typeMap[type]();
            let attributeTypes = typeMap[type].getAttributeTypeMap();
            for (let index in attributeTypes) {
                let attributeType = attributeTypes[index];
                let value = ObjectSerializer.deserialize(data[attributeType.baseName], attributeType.type, attributeType.format);
                if (value !== undefined) {
                    instance[attributeType.name] = value;
                }
            }
            return instance;
        }
    }


    /**
     * Normalize media type
     *
     * We currently do not handle any media types attributes, i.e. anything
     * after a semicolon. All content is assumed to be UTF-8 compatible.
     */
    public static normalizeMediaType(mediaType: string | undefined): string | undefined {
        if (mediaType === undefined) {
            return undefined;
        }
        return mediaType.split(";")[0].trim().toLowerCase();
    }

    /**
     * From a list of possible media types, choose the one we can handle best.
     *
     * The order of the given media types does not have any impact on the choice
     * made.
     */
    public static getPreferredMediaType(mediaTypes: Array<string>): string {
        /** According to OAS 3 we should default to json */
        if (!mediaTypes) {
            return "application/json";
        }

        const normalMediaTypes = mediaTypes.map(this.normalizeMediaType);
        let selectedMediaType: string | undefined = undefined;
        let selectedRank: number = -Infinity;
        for (const mediaType of normalMediaTypes) {
            if (supportedMediaTypes[mediaType!] > selectedRank) {
                selectedMediaType = mediaType;
                selectedRank = supportedMediaTypes[mediaType!];
            }
        }

        if (selectedMediaType === undefined) {
            throw new Error("None of the given media types are supported: " + mediaTypes.join(", "));
        }

        return selectedMediaType!;
    }

    /**
     * Convert data to a string according the given media type
     */
    public static stringify(data: any, mediaType: string): string {
        if (mediaType === "application/json") {
            return JSON.stringify(data);
        }

        throw new Error("The mediaType " + mediaType + " is not supported by ObjectSerializer.stringify.");
    }

    /**
     * Parse data from a string according to the given media type
     */
    public static parse(rawData: string, mediaType: string | undefined) {
        if (mediaType === undefined) {
            throw new Error("Cannot parse content. No Content-Type defined.");
        }

        if (mediaType === "application/json") {
            return JSON.parse(rawData);
        }

        throw new Error("The mediaType " + mediaType + " is not supported by ObjectSerializer.parse.");
    }
}
