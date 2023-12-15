Query gRPC service in Windmill
==============================

A gRPC service requires any client to provide the schema of the API (the .proto files) to be able to query it.

In Windmill, the proto file apyload payload can be saved into a a variable, and this variable can be loaded in any script wanting to query the remote service.

### Example

The provided [docker-compose](./docker-compose.yml) spins up a stack with a single Windmill instance and a dummy gRPC service (the code can be found 
[here](https://github.com/gbouv/grpc-quickstart-service/tree/main)). It exposes the following API:

```proto
syntax = "proto3";

option go_package = "github.com/gbouv/grpc-quickstart-service/protobuf";

package helloworld;

// The greeting service definition.
service Greeter {
  // Sends a greeting
  rpc SayHello (HelloRequest) returns (HelloReply) {}
}

// The request message containing the user's name.
message HelloRequest {
  string name = 1;
}

// The response message containing the greetings
message HelloReply {
  string message = 1;
}
```

### gRPC query in Windmill

The easiest is to use Bun to query the gRPC service. Javacript has the huge advantage of being able to read the proto file to dynamically create a service client (see [gRPC dynamic codegen tutorial](https://grpc.io/docs/languages/node/quickstart/)).

First, save the content of the above proto into a Windmill variable (named `service_proto` in this tutorial).

Create a new Bun script in windmill with the following content:

```js
import * as wmill from "windmill-client"
import * as grpc from "@grpc/grpc-js"
import * as protoLoader from "@grpc/proto-loader"

const SERVICE_NAME = 'helloworld'

export async function main() {
  await writeProto()
  let service = await loadService()

  let client = new service.Greeter("localhost:1353", grpc.credentials.createInsecure());
  return await query(client, "SayHello", { name: "Windmill!" })
}

async function query(client, method, args): Promise<string> {
  return new Promise((resolve, reject) => {
    client[method](args, function(err, resp) {
      if (resp) {
        resolve(resp)
      } else {
        reject(err)
      }
    });
  })
}

async function loadService() {
  var serviceDefinition = protoLoader.loadSync(
    "./service.proto",
    {
      keepCase: true,
      longs: String,
      enums: String,
      defaults: true,
      oneofs: true
    });
  return grpc.loadPackageDefinition(serviceDefinition)[SERVICE_NAME];
}

async function writeProto() {
  const proto = await wmill.getVariable('u/admin/service_proto');
  await Bun.write("./service.proto", proto);
}
```

The script will load the proto files to generate a service client, then query the `SayHello` endpoint with `{ "name": "Windmill!" }` payload.
When you execute this script, it return:
```
{
    "message": "Hello Windmill!"
}
```
