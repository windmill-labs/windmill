# Java

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

## Maven Dependencies

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
