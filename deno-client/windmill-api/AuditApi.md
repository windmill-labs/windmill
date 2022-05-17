# .AuditApi

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**getAuditLog**](AuditApi.md#getAuditLog) | **GET** /w/{workspace}/audit/get/{id} | get audit log (requires admin privilege)
[**listAuditLogs**](AuditApi.md#listAuditLogs) | **GET** /w/{workspace}/audit/list | list audit logs (requires admin privilege)


# **getAuditLog**
> AuditLog getAuditLog()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .AuditApi(configuration);

let body:.AuditApiGetAuditLogRequest = {
  // string
  workspace: "workspace_example",
  // number
  id: 1,
};

apiInstance.getAuditLog(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **id** | [**number**] |  | defaults to undefined


### Return type

**AuditLog**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | an audit log |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)

# **listAuditLogs**
> Array<AuditLog> listAuditLogs()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .AuditApi(configuration);

let body:.AuditApiListAuditLogsRequest = {
  // string
  workspace: "workspace_example",
  // number | which page to return (start at 1, default 1) (optional)
  page: 1,
  // number | number of items to return for a given page (default 30, max 100) (optional)
  perPage: 1,
  // Date | filter on created before (exclusive) timestamp (optional)
  before: new Date('1970-01-01T00:00:00.00Z'),
  // Date | filter on created after (exclusive) timestamp (optional)
  after: new Date('1970-01-01T00:00:00.00Z'),
  // string | filter on exact username of user (optional)
  username: "username_example",
  // string | filter on exact or prefix name of operation (optional)
  operation: "operation_example",
  // string | filter on exact or prefix name of resource (optional)
  resource: "resource_example",
  // 'Create' | 'Update' | 'Delete' | 'Execute' | filter on type of operation (optional)
  actionKind: "Create",
};

apiInstance.listAuditLogs(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **workspace** | [**string**] |  | defaults to undefined
 **page** | [**number**] | which page to return (start at 1, default 1) | (optional) defaults to undefined
 **perPage** | [**number**] | number of items to return for a given page (default 30, max 100) | (optional) defaults to undefined
 **before** | [**Date**] | filter on created before (exclusive) timestamp | (optional) defaults to undefined
 **after** | [**Date**] | filter on created after (exclusive) timestamp | (optional) defaults to undefined
 **username** | [**string**] | filter on exact username of user | (optional) defaults to undefined
 **operation** | [**string**] | filter on exact or prefix name of operation | (optional) defaults to undefined
 **resource** | [**string**] | filter on exact or prefix name of resource | (optional) defaults to undefined
 **actionKind** | [**&#39;Create&#39; | &#39;Update&#39; | &#39;Delete&#39; | &#39;Execute&#39;**]**Array<&#39;Create&#39; &#124; &#39;Update&#39; &#124; &#39;Delete&#39; &#124; &#39;Execute&#39;>** | filter on type of operation | (optional) defaults to undefined


### Return type

**Array<AuditLog>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | a list of audit logs |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)


