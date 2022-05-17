# .ScheduleApi

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**createSchedule**](ScheduleApi.md#createSchedule) | **POST** /w/{workspace}/schedules/create | create schedule
[**getSchedule**](ScheduleApi.md#getSchedule) | **GET** /w/{workspace}/schedules/get/{path} | get schedule
[**listSchedules**](ScheduleApi.md#listSchedules) | **GET** /w/{workspace}/schedules/list | list schedules
[**previewSchedule**](ScheduleApi.md#previewSchedule) | **POST** /schedules/preview | preview schedule
[**setScheduleEnabled**](ScheduleApi.md#setScheduleEnabled) | **POST** /w/{workspace}/schedules/setenabled/{path} | set enabled schedule
[**updateSchedule**](ScheduleApi.md#updateSchedule) | **POST** /w/{workspace}/schedules/update/{path} | update schedule


# **createSchedule**
> string createSchedule(newSchedule)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScheduleApi(configuration);

let body:.ScheduleApiCreateScheduleRequest = {
  // string
  workspace: "workspace_example",
  // NewSchedule | new schedule
  newSchedule: {
    path: "path_example",
    schedule: "schedule_example",
    offset: 1,
    scriptPath: "scriptPath_example",
    isFlow: true,
    args: {
      "key": null,
    },
  },
};

apiInstance.createSchedule(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **newSchedule** | **NewSchedule**| new schedule |
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
**201** | schedule created |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **getSchedule**
> Schedule getSchedule()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScheduleApi(configuration);

let body:.ScheduleApiGetScheduleRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
};

apiInstance.getSchedule(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined


### Return type

**Schedule**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | schedule deleted |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listSchedules**
> Array<Schedule> listSchedules()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScheduleApi(configuration);

let body:.ScheduleApiListSchedulesRequest = {
  // string
  workspace: "workspace_example",
  // number | which page to return (start at 1, default 1) (optional)
  page: 1,
  // number | number of items to return for a given page (default 30, max 100) (optional)
  perPage: 1,
};

apiInstance.listSchedules(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **page** | [**number**] | which page to return (start at 1, default 1) | (optional) defaults to undefined
 **perPage** | [**number**] | number of items to return for a given page (default 30, max 100) | (optional) defaults to undefined


### Return type

**Array<Schedule>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | schedule list |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **previewSchedule**
> Array<Date> previewSchedule(inlineObject14)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScheduleApi(configuration);

let body:.ScheduleApiPreviewScheduleRequest = {
  // InlineObject14
  inlineObject14: {
    schedule: "schedule_example",
    offset: 1,
  },
};

apiInstance.previewSchedule(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject14** | **InlineObject14**|  |


### Return type

**Array<Date>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | the preview of the next 10 time this schedule would apply to |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **setScheduleEnabled**
> string setScheduleEnabled(inlineObject15)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScheduleApi(configuration);

let body:.ScheduleApiSetScheduleEnabledRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
  // InlineObject15
  inlineObject15: {
    enabled: true,
  },
};

apiInstance.setScheduleEnabled(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **inlineObject15** | **InlineObject15**|  |
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined


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
**200** | schedule enabled set |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **updateSchedule**
> string updateSchedule(editSchedule)


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .ScheduleApi(configuration);

let body:.ScheduleApiUpdateScheduleRequest = {
  // string
  workspace: "workspace_example",
  // string
  path: "path_example",
  // EditSchedule | updated schedule
  editSchedule: {
    schedule: "schedule_example",
    scriptPath: "scriptPath_example",
    isFlow: true,
    args: {
      "key": null,
    },
  },
};

apiInstance.updateSchedule(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **editSchedule** | **EditSchedule**| updated schedule |
 **workspace** | [**string**] |  | defaults to undefined
 **path** | [**string**] |  | defaults to undefined


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
**200** | schedule updated |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)


