# .JobApi

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cancelQueuedJob**](JobApi.md#cancelQueuedJob) | **POST** /w/{workspace}/jobs/queue/cancel/{id} | cancel queued job
[**deleteCompletedJob**](JobApi.md#deleteCompletedJob) | **POST** /w/{workspace}/jobs/completed/delete/{id} | delete completed job (erase content but keep run id)
[**getCompletedJob**](JobApi.md#getCompletedJob) | **GET** /w/{workspace}/jobs/completed/get/{id} | get completed job
[**getJob**](JobApi.md#getJob) | **GET** /w/{workspace}/jobs/get/{id} | get job
[**getJobUpdates**](JobApi.md#getJobUpdates) | **GET** /w/{workspace}/jobs/getupdate/{id} | get job updates
[**listCompletedJobs**](JobApi.md#listCompletedJobs) | **GET** /w/{workspace}/jobs/completed/list | list all available completed jobs
[**listJobs**](JobApi.md#listJobs) | **GET** /w/{workspace}/jobs/list | list all available jobs
[**listQueue**](JobApi.md#listQueue) | **GET** /w/{workspace}/jobs/queue/list | list all available queued jobs
[**runFlowByPath**](JobApi.md#runFlowByPath) | **POST** /w/{workspace}/jobs/run/f/{path} | run flow by path
[**runFlowPreview**](JobApi.md#runFlowPreview) | **POST** /w/{workspace}/jobs/run/preview_flow | run flow preview
[**runScriptByHash**](JobApi.md#runScriptByHash) | **POST** /w/{workspace}/jobs/run/h/{hash} | run script by hash
[**runScriptByPath**](JobApi.md#runScriptByPath) | **POST** /w/{workspace}/jobs/run/p/{path} | run script by path
[**runScriptPreview**](JobApi.md#runScriptPreview) | **POST** /w/{workspace}/jobs/run/preview | run script preview


# **cancelQueuedJob**
> string cancelQueuedJob(inlineObject13)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiCancelQueuedJobRequest = {
  // string
  workspace: "workspace_example",
  // string
  id: "id_example",
  // InlineObject13
  inlineObject13: {
    reason: "reason_example",
  },
};

apiInstance.cancelQueuedJob(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject13** | **InlineObject13**|  |
 **workspace** | [**string**] |  | defaults to undefined
 **id** | [**string**] |  | defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | job canceled |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **deleteCompletedJob**
> CompletedJob deleteCompletedJob()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiDeleteCompletedJobRequest = {
  // string
  workspace: "workspace_example",
  // string
  id: "id_example",
};

apiInstance.deleteCompletedJob(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **id** | [**string**] |  | defaults to undefined


### Return type

**CompletedJob**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | job details |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getCompletedJob**
> CompletedJob getCompletedJob()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiGetCompletedJobRequest = {
  // string
  workspace: "workspace_example",
  // string
  id: "id_example",
};

apiInstance.getCompletedJob(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **id** | [**string**] |  | defaults to undefined


### Return type

**CompletedJob**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | job details |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getJob**
> Job getJob()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiGetJobRequest = {
  // string
  workspace: "workspace_example",
  // string
  id: "id_example",
};

apiInstance.getJob(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **id** | [**string**] |  | defaults to undefined


### Return type

**Job**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | job details |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getJobUpdates**
> InlineResponse2002 getJobUpdates()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiGetJobUpdatesRequest = {
  // string
  workspace: "workspace_example",
  // string
  id: "id_example",
  // boolean (optional)
  running: true,
  // number (optional)
  logOffset: 3.14,
};

apiInstance.getJobUpdates(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **id** | [**string**] |  | defaults to undefined
 **running** | [**boolean**] |  | (optional) defaults to undefined
 **logOffset** | [**number**] |  | (optional) defaults to undefined


### Return type

**InlineResponse2002**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | job details |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listCompletedJobs**
> Array<CompletedJob> listCompletedJobs()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiListCompletedJobsRequest = {
  // string
  workspace: "workspace_example",
  // boolean | order by desc order (default true) (optional)
  orderDesc: true,
  // string | mask to filter exact matching user creator (optional)
  createdBy: "created_by_example",
  // string | The parent job that is at the origin and responsible for the execution of this script if any (optional)
  parentJob: "parent_job_example",
  // string | mask to filter exact matching path (optional)
  scriptPathExact: "script_path_exact_example",
  // string | mask to filter matching starting path (optional)
  scriptPathStart: "script_path_start_example",
  // string | mask to filter exact matching path (optional)
  scriptHash: "script_hash_example",
  // Date | filter on created before (inclusive) timestamp (optional)
  createdBefore: new Date('1970-01-01T00:00:00.00Z'),
  // Date | filter on created after (exclusive) timestamp (optional)
  createdAfter: new Date('1970-01-01T00:00:00.00Z'),
  // boolean | filter on successful jobs (optional)
  success: true,
  // string | filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by, (optional)
  jobKinds: "job_kinds_example",
};

apiInstance.listCompletedJobs(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **orderDesc** | [**boolean**] | order by desc order (default true) | (optional) defaults to undefined
 **createdBy** | [**string**] | mask to filter exact matching user creator | (optional) defaults to undefined
 **parentJob** | [**string**] | The parent job that is at the origin and responsible for the execution of this script if any | (optional) defaults to undefined
 **scriptPathExact** | [**string**] | mask to filter exact matching path | (optional) defaults to undefined
 **scriptPathStart** | [**string**] | mask to filter matching starting path | (optional) defaults to undefined
 **scriptHash** | [**string**] | mask to filter exact matching path | (optional) defaults to undefined
 **createdBefore** | [**Date**] | filter on created before (inclusive) timestamp | (optional) defaults to undefined
 **createdAfter** | [**Date**] | filter on created after (exclusive) timestamp | (optional) defaults to undefined
 **success** | [**boolean**] | filter on successful jobs | (optional) defaults to undefined
 **jobKinds** | [**string**] | filter on job kind (values &#39;preview&#39;, &#39;script&#39;, &#39;dependencies&#39;, &#39;flow&#39;) separated by, | (optional) defaults to undefined


### Return type

**Array<CompletedJob>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | All available completed jobs |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listJobs**
> Array<Job> listJobs()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiListJobsRequest = {
  // string
  workspace: "workspace_example",
  // string | mask to filter exact matching user creator (optional)
  createdBy: "created_by_example",
  // string | The parent job that is at the origin and responsible for the execution of this script if any (optional)
  parentJob: "parent_job_example",
  // string | mask to filter exact matching path (optional)
  scriptPathExact: "script_path_exact_example",
  // string | mask to filter matching starting path (optional)
  scriptPathStart: "script_path_start_example",
  // string | mask to filter exact matching path (optional)
  scriptHash: "script_hash_example",
  // Date | filter on created before (inclusive) timestamp (optional)
  createdBefore: new Date('1970-01-01T00:00:00.00Z'),
  // Date | filter on created after (exclusive) timestamp (optional)
  createdAfter: new Date('1970-01-01T00:00:00.00Z'),
  // string | filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by, (optional)
  jobKinds: "job_kinds_example",
  // boolean | filter on successful jobs (optional)
  success: true,
};

apiInstance.listJobs(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **createdBy** | [**string**] | mask to filter exact matching user creator | (optional) defaults to undefined
 **parentJob** | [**string**] | The parent job that is at the origin and responsible for the execution of this script if any | (optional) defaults to undefined
 **scriptPathExact** | [**string**] | mask to filter exact matching path | (optional) defaults to undefined
 **scriptPathStart** | [**string**] | mask to filter matching starting path | (optional) defaults to undefined
 **scriptHash** | [**string**] | mask to filter exact matching path | (optional) defaults to undefined
 **createdBefore** | [**Date**] | filter on created before (inclusive) timestamp | (optional) defaults to undefined
 **createdAfter** | [**Date**] | filter on created after (exclusive) timestamp | (optional) defaults to undefined
 **jobKinds** | [**string**] | filter on job kind (values &#39;preview&#39;, &#39;script&#39;, &#39;dependencies&#39;, &#39;flow&#39;) separated by, | (optional) defaults to undefined
 **success** | [**boolean**] | filter on successful jobs | (optional) defaults to undefined


### Return type

**Array<Job>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | All jobs |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listQueue**
> Array<QueuedJob> listQueue()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiListQueueRequest = {
  // string
  workspace: "workspace_example",
  // boolean | order by desc order (default true) (optional)
  orderDesc: true,
  // string | mask to filter exact matching user creator (optional)
  createdBy: "created_by_example",
  // string | The parent job that is at the origin and responsible for the execution of this script if any (optional)
  parentJob: "parent_job_example",
  // string | mask to filter exact matching path (optional)
  scriptPathExact: "script_path_exact_example",
  // string | mask to filter matching starting path (optional)
  scriptPathStart: "script_path_start_example",
  // string | mask to filter exact matching path (optional)
  scriptHash: "script_hash_example",
  // Date | filter on created before (inclusive) timestamp (optional)
  createdBefore: new Date('1970-01-01T00:00:00.00Z'),
  // Date | filter on created after (exclusive) timestamp (optional)
  createdAfter: new Date('1970-01-01T00:00:00.00Z'),
  // boolean | filter on successful jobs (optional)
  success: true,
  // string | filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by, (optional)
  jobKinds: "job_kinds_example",
};

apiInstance.listQueue(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **orderDesc** | [**boolean**] | order by desc order (default true) | (optional) defaults to undefined
 **createdBy** | [**string**] | mask to filter exact matching user creator | (optional) defaults to undefined
 **parentJob** | [**string**] | The parent job that is at the origin and responsible for the execution of this script if any | (optional) defaults to undefined
 **scriptPathExact** | [**string**] | mask to filter exact matching path | (optional) defaults to undefined
 **scriptPathStart** | [**string**] | mask to filter matching starting path | (optional) defaults to undefined
 **scriptHash** | [**string**] | mask to filter exact matching path | (optional) defaults to undefined
 **createdBefore** | [**Date**] | filter on created before (inclusive) timestamp | (optional) defaults to undefined
 **createdAfter** | [**Date**] | filter on created after (exclusive) timestamp | (optional) defaults to undefined
 **success** | [**boolean**] | filter on successful jobs | (optional) defaults to undefined
 **jobKinds** | [**string**] | filter on job kind (values &#39;preview&#39;, &#39;script&#39;, &#39;dependencies&#39;, &#39;flow&#39;) separated by, | (optional) defaults to undefined


### Return type

**Array<QueuedJob>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | All available queued jobs |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **runFlowByPath**
> string runFlowByPath(requestBody)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiRunFlowByPathRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
  // { [key: string]: any; } | flow args
  requestBody: {
    "key": null,
  },
  // Date | when to schedule this job (leave empty for immediate run) (optional)
  scheduledFor: new Date('1970-01-01T00:00:00.00Z'),
  // number | schedule the script to execute in the number of seconds starting now (optional)
  scheduledInSecs: 3.14,
  // string | The parent job that is at the origin and responsible for the execution of this script if any (optional)
  parentJob: "parent_job_example",
};

apiInstance.runFlowByPath(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **requestBody** | **{ [key: string]: any; }**| flow args |
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined
 **scheduledFor** | [**Date**] | when to schedule this job (leave empty for immediate run) | (optional) defaults to undefined
 **scheduledInSecs** | [**number**] | schedule the script to execute in the number of seconds starting now | (optional) defaults to undefined
 **parentJob** | [**string**] | The parent job that is at the origin and responsible for the execution of this script if any | (optional) defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | job created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **runFlowPreview**
> string runFlowPreview(flowPreview)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiRunFlowPreviewRequest = {
  // string
  workspace: "workspace_example",
  // FlowPreview | preview
  flowPreview: {
    value: {
      modules: [
        {
          inputTransform: {
            "key": {
              type: "static",
              step: 3.14,
              value: null,
              expr: "expr_example",
            },
          },
          value: {
            path: "path_example",
            type: "script",
          },
        },
      ],
      failureModule: {
        inputTransform: {
          "key": {
            type: "static",
            step: 3.14,
            value: null,
            expr: "expr_example",
          },
        },
        value: {
          path: "path_example",
          type: "script",
        },
      },
    },
    path: "path_example",
    args: {
      "key": null,
    },
  },
};

apiInstance.runFlowPreview(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **flowPreview** | **FlowPreview**| preview |
 **workspace** | [**string**] |  | defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | job created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **runScriptByHash**
> string runScriptByHash(body)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiRunScriptByHashRequest = {
  // string
  workspace: "workspace_example",
  // string
  hash: "hash_example",
  // any | Partially filled args
  body: {},
  // Date | when to schedule this job (leave empty for immediate run) (optional)
  scheduledFor: new Date('1970-01-01T00:00:00.00Z'),
  // number | schedule the script to execute in the number of seconds starting now (optional)
  scheduledInSecs: 3.14,
  // string | The parent job that is at the origin and responsible for the execution of this script if any (optional)
  parentJob: "parent_job_example",
};

apiInstance.runScriptByHash(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **body** | **any**| Partially filled args |
 **workspace** | [**string**] |  | defaults to undefined
 **hash** | [**string**] |  | defaults to undefined
 **scheduledFor** | [**Date**] | when to schedule this job (leave empty for immediate run) | (optional) defaults to undefined
 **scheduledInSecs** | [**number**] | schedule the script to execute in the number of seconds starting now | (optional) defaults to undefined
 **parentJob** | [**string**] | The parent job that is at the origin and responsible for the execution of this script if any | (optional) defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | job created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **runScriptByPath**
> string runScriptByPath(requestBody)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiRunScriptByPathRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
  // { [key: string]: any; } | script args
  requestBody: {
    "key": null,
  },
  // Date | when to schedule this job (leave empty for immediate run) (optional)
  scheduledFor: new Date('1970-01-01T00:00:00.00Z'),
  // number | schedule the script to execute in the number of seconds starting now (optional)
  scheduledInSecs: 3.14,
  // string | The parent job that is at the origin and responsible for the execution of this script if any (optional)
  parentJob: "parent_job_example",
};

apiInstance.runScriptByPath(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **requestBody** | **{ [key: string]: any; }**| script args |
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined
 **scheduledFor** | [**Date**] | when to schedule this job (leave empty for immediate run) | (optional) defaults to undefined
 **scheduledInSecs** | [**number**] | schedule the script to execute in the number of seconds starting now | (optional) defaults to undefined
 **parentJob** | [**string**] | The parent job that is at the origin and responsible for the execution of this script if any | (optional) defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | job created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **runScriptPreview**
> string runScriptPreview(preview)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .JobApi(configuration);

let body:.JobApiRunScriptPreviewRequest = {
  // string
  workspace: "workspace_example",
  // Preview | previw
  preview: {
    content: "content_example",
    path: "path_example",
    args: {
      "key": null,
    },
    language: "python3",
  },
};

apiInstance.runScriptPreview(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **preview** | **Preview**| previw |
 **workspace** | [**string**] |  | defaults to undefined


### Return type

**string**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: text/plain


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | job created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)


