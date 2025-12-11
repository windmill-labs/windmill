# Compiled Languages (C#, Java)

## C#

The script must contain a public static `Main` method inside a class:

```csharp
public class Script
{
    public static object Main(string name, int count)
    {
        return new { Name = name, Count = count };
    }
}
```

**Important:**
- Class name is irrelevant
- Method must be `public static`
- Return type can be `object` or specific type

### NuGet Packages

Add packages using the `#r` directive at the top:

```csharp
#r "nuget: Newtonsoft.Json, 13.0.3"
#r "nuget: RestSharp, 110.2.0"

using Newtonsoft.Json;
using RestSharp;

public class Script
{
    public static object Main(string url)
    {
        var client = new RestClient(url);
        var request = new RestRequest();
        var response = client.Get(request);
        return JsonConvert.DeserializeObject(response.Content);
    }
}
```

## Java

The script must contain a Main public class with a `public static main()` method:

```java
public class Main {
    public static Object main(String name, int count) {
        java.util.Map<String, Object> result = new java.util.HashMap<>();
        result.put("name", name);
        result.put("count", count);
        return result;
    }
}
```

**Important:**
- Class must be named `Main`
- Method must be `public static Object main(...)`
- Return type is `Object` or `void`

### Maven Dependencies

Add dependencies using comments at the top:

```java
//requirements:
//com.google.code.gson:gson:2.10.1
//org.apache.httpcomponents:httpclient:4.5.14

import com.google.gson.Gson;

public class Main {
    public static Object main(String input) {
        Gson gson = new Gson();
        return gson.fromJson(input, Object.class);
    }
}
```
