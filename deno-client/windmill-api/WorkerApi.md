# .WorkerApi

All URIs are relative to */api*

Method | HTTP request | Description
------------- | ------------- | -------------
[**listWorkers**](WorkerApi.md#listWorkers) | **GET** /workers/list | list workers


# **listWorkers**
> Array<WorkerPing> listWorkers()


### Example


```typescript
import {  } from '';
import * as fs from 'fs';

const configuration = .createConfiguration();
const apiInstance = new .WorkerApi(configuration);

let body:.WorkerApiListWorkersRequest = {
  // number | which page to return (start at 1, default 1) (optional)
  page: 1,
  // number | number of items to return for a given page (default 30, max 100) (optional)
  perPage: 1,
};

apiInstance.listWorkers(body).then((data:any) => {
  console.log('API called successfully. Returned data: ' + data);
}).catch((error:any) => console.error(error));
```


### Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **page** | [**number**] | which page to return (start at 1, default 1) | (optional) defaults to undefined
 **perPage** | [**number**] | number of items to return for a given page (default 30, max 100) | (optional) defaults to undefined


### Return type

**Array<WorkerPing>**

### Authorization

[bearerAuth](README.md#bearerAuth), [cookieAuth](README.md#cookieAuth)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | a list of workers |  -  |

[[Back to top]](#) [[Back to API list]](README.md#documentation-for-api-endpoints) [[Back to Model list]](README.md#documentation-for-models) [[Back to README]](README.md)


