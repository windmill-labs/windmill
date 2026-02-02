---
name: triggers
description: MUST use when configuring triggers.
---

# Windmill Triggers

Triggers allow external events to invoke your scripts and flows.

## File Naming

Trigger configuration files use the pattern: `{path}.{trigger_type}_trigger.yaml`

Examples:
- `u/user/webhook.http_trigger.yaml`
- `f/data/kafka_consumer.kafka_trigger.yaml`
- `f/sync/postgres_cdc.postgres_trigger.yaml`

## CLI Commands

```bash
# Push trigger configuration
wmill sync push

# Pull triggers from Windmill
wmill sync pull
```


## HttpTrigger (`*.http_trigger.yaml`)

Must be a YAML file that adheres to the following schema:

```json
{
  "type": "object",
  "properties": {
    "script_path": {
      "type": "string",
      "description": "Path to the script or flow to execute when triggered"
    },
    "is_flow": {
      "type": "boolean",
      "description": "True if script_path points to a flow, false if it points to a script"
    },
    "route_path": {
      "type": "string",
      "description": "The URL route path that will trigger this endpoint (e.g., '/api/myendpoint')"
    },
    "static_asset_config": {
      "type": "object",
      "properties": {
        "s3": {
          "type": "string",
          "description": "S3 bucket path for static assets"
        },
        "storage": {
          "type": "string",
          "description": "Storage path for static assets"
        },
        "filename": {
          "type": "string",
          "description": "Filename for the static asset"
        }
      },
      "description": "Configuration for serving static assets (s3 bucket, storage path, filename)"
    },
    "http_method": {
      "type": "string",
      "enum": [
        "get",
        "post",
        "put",
        "delete",
        "patch"
      ]
    },
    "authentication_resource_path": {
      "type": "string",
      "description": "Path to the resource containing authentication configuration (for api_key, basic_http, custom_script, signature methods)"
    },
    "summary": {
      "type": "string",
      "description": "Short summary describing the purpose of this trigger"
    },
    "description": {
      "type": "string",
      "description": "Detailed description of what this trigger does"
    },
    "request_type": {
      "type": "string",
      "enum": [
        "sync",
        "async",
        "sync_sse"
      ]
    },
    "authentication_method": {
      "type": "string",
      "enum": [
        "none",
        "windmill",
        "api_key",
        "basic_http",
        "custom_script",
        "signature"
      ]
    },
    "is_static_website": {
      "type": "boolean",
      "description": "If true, serves static files from S3/storage instead of running a script"
    },
    "workspaced_route": {
      "type": "boolean",
      "description": "If true, the route includes the workspace ID in the path"
    },
    "wrap_body": {
      "type": "boolean",
      "description": "If true, wraps the request body in a 'body' parameter"
    },
    "raw_string": {
      "type": "boolean",
      "description": "If true, passes the request body as a raw string instead of parsing as JSON"
    },
    "error_handler_path": {
      "type": "string",
      "description": "Path to a script or flow to run when the triggered job fails"
    },
    "error_handler_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "retry": {
      "type": "object",
      "properties": {
        "constant": {
          "type": "object",
          "description": "Retry with constant delay between attempts",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "seconds": {
              "type": "integer",
              "description": "Seconds to wait between retries"
            }
          }
        },
        "exponential": {
          "type": "object",
          "description": "Retry with exponential backoff (delay doubles each time)",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "multiplier": {
              "type": "integer",
              "description": "Multiplier for exponential backoff"
            },
            "seconds": {
              "type": "integer",
              "minimum": 1,
              "description": "Initial delay in seconds"
            },
            "random_factor": {
              "type": "integer",
              "minimum": 0,
              "maximum": 100,
              "description": "Random jitter percentage (0-100) to avoid thundering herd"
            }
          }
        },
        "retry_if": {
          "$ref": "#/components/schemas/RetryIf"
        }
      },
      "description": "Retry configuration for failed module executions"
    }
  },
  "required": [
    "script_path",
    "is_flow",
    "route_path",
    "request_type",
    "authentication_method",
    "http_method",
    "is_static_website",
    "workspaced_route",
    "wrap_body",
    "raw_string"
  ]
}
```

## WebsocketTrigger (`*.websocket_trigger.yaml`)

Must be a YAML file that adheres to the following schema:

```json
{
  "type": "object",
  "properties": {
    "script_path": {
      "type": "string",
      "description": "Path to the script or flow to execute when triggered"
    },
    "is_flow": {
      "type": "boolean",
      "description": "True if script_path points to a flow, false if it points to a script"
    },
    "url": {
      "type": "string",
      "description": "The WebSocket URL to connect to (can be a static URL or computed by a runnable)"
    },
    "filters": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "key": {
            "type": "string"
          },
          "value": {}
        }
      },
      "description": "Array of key-value filters to match incoming messages (only matching messages trigger the script)"
    },
    "initial_messages": {
      "type": "array",
      "items": {
        "type": "object"
      },
      "description": "Messages to send immediately after connecting (can be raw strings or computed by runnables)"
    },
    "url_runnable_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "can_return_message": {
      "type": "boolean",
      "description": "If true, the script can return a message to send back through the WebSocket"
    },
    "can_return_error_result": {
      "type": "boolean",
      "description": "If true, error results are sent back through the WebSocket"
    },
    "error_handler_path": {
      "type": "string",
      "description": "Path to a script or flow to run when the triggered job fails"
    },
    "error_handler_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "retry": {
      "type": "object",
      "properties": {
        "constant": {
          "type": "object",
          "description": "Retry with constant delay between attempts",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "seconds": {
              "type": "integer",
              "description": "Seconds to wait between retries"
            }
          }
        },
        "exponential": {
          "type": "object",
          "description": "Retry with exponential backoff (delay doubles each time)",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "multiplier": {
              "type": "integer",
              "description": "Multiplier for exponential backoff"
            },
            "seconds": {
              "type": "integer",
              "minimum": 1,
              "description": "Initial delay in seconds"
            },
            "random_factor": {
              "type": "integer",
              "minimum": 0,
              "maximum": 100,
              "description": "Random jitter percentage (0-100) to avoid thundering herd"
            }
          }
        },
        "retry_if": {
          "$ref": "#/components/schemas/RetryIf"
        }
      },
      "description": "Retry configuration for failed module executions"
    }
  },
  "required": [
    "script_path",
    "is_flow",
    "url",
    "filters",
    "can_return_message",
    "can_return_error_result"
  ]
}
```

## KafkaTrigger (`*.kafka_trigger.yaml`)

Must be a YAML file that adheres to the following schema:

```json
{
  "type": "object",
  "properties": {
    "script_path": {
      "type": "string",
      "description": "Path to the script or flow to execute when triggered"
    },
    "is_flow": {
      "type": "boolean",
      "description": "True if script_path points to a flow, false if it points to a script"
    },
    "kafka_resource_path": {
      "type": "string",
      "description": "Path to the Kafka resource containing connection configuration"
    },
    "group_id": {
      "type": "string",
      "description": "Kafka consumer group ID for this trigger"
    },
    "topics": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "description": "Array of Kafka topic names to subscribe to"
    },
    "error_handler_path": {
      "type": "string",
      "description": "Path to a script or flow to run when the triggered job fails"
    },
    "error_handler_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "retry": {
      "type": "object",
      "properties": {
        "constant": {
          "type": "object",
          "description": "Retry with constant delay between attempts",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "seconds": {
              "type": "integer",
              "description": "Seconds to wait between retries"
            }
          }
        },
        "exponential": {
          "type": "object",
          "description": "Retry with exponential backoff (delay doubles each time)",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "multiplier": {
              "type": "integer",
              "description": "Multiplier for exponential backoff"
            },
            "seconds": {
              "type": "integer",
              "minimum": 1,
              "description": "Initial delay in seconds"
            },
            "random_factor": {
              "type": "integer",
              "minimum": 0,
              "maximum": 100,
              "description": "Random jitter percentage (0-100) to avoid thundering herd"
            }
          }
        },
        "retry_if": {
          "$ref": "#/components/schemas/RetryIf"
        }
      },
      "description": "Retry configuration for failed module executions"
    }
  },
  "required": [
    "script_path",
    "is_flow",
    "kafka_resource_path",
    "group_id",
    "topics"
  ]
}
```

## NatsTrigger (`*.nats_trigger.yaml`)

Must be a YAML file that adheres to the following schema:

```json
{
  "type": "object",
  "properties": {
    "script_path": {
      "type": "string",
      "description": "Path to the script or flow to execute when triggered"
    },
    "is_flow": {
      "type": "boolean",
      "description": "True if script_path points to a flow, false if it points to a script"
    },
    "nats_resource_path": {
      "type": "string",
      "description": "Path to the NATS resource containing connection configuration"
    },
    "use_jetstream": {
      "type": "boolean",
      "description": "If true, uses NATS JetStream for durable message delivery"
    },
    "stream_name": {
      "type": "string",
      "description": "JetStream stream name (required when use_jetstream is true)"
    },
    "consumer_name": {
      "type": "string",
      "description": "JetStream consumer name (required when use_jetstream is true)"
    },
    "subjects": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "description": "Array of NATS subjects to subscribe to"
    },
    "error_handler_path": {
      "type": "string",
      "description": "Path to a script or flow to run when the triggered job fails"
    },
    "error_handler_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "retry": {
      "type": "object",
      "properties": {
        "constant": {
          "type": "object",
          "description": "Retry with constant delay between attempts",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "seconds": {
              "type": "integer",
              "description": "Seconds to wait between retries"
            }
          }
        },
        "exponential": {
          "type": "object",
          "description": "Retry with exponential backoff (delay doubles each time)",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "multiplier": {
              "type": "integer",
              "description": "Multiplier for exponential backoff"
            },
            "seconds": {
              "type": "integer",
              "minimum": 1,
              "description": "Initial delay in seconds"
            },
            "random_factor": {
              "type": "integer",
              "minimum": 0,
              "maximum": 100,
              "description": "Random jitter percentage (0-100) to avoid thundering herd"
            }
          }
        },
        "retry_if": {
          "$ref": "#/components/schemas/RetryIf"
        }
      },
      "description": "Retry configuration for failed module executions"
    }
  },
  "required": [
    "script_path",
    "is_flow",
    "nats_resource_path",
    "use_jetstream",
    "subjects"
  ]
}
```

## PostgresTrigger (`*.postgres_trigger.yaml`)

Must be a YAML file that adheres to the following schema:

```json
{
  "type": "object",
  "properties": {
    "script_path": {
      "type": "string",
      "description": "Path to the script or flow to execute when triggered"
    },
    "is_flow": {
      "type": "boolean",
      "description": "True if script_path points to a flow, false if it points to a script"
    },
    "postgres_resource_path": {
      "type": "string",
      "description": "Path to the PostgreSQL resource containing connection configuration"
    },
    "publication_name": {
      "type": "string",
      "description": "Name of the PostgreSQL publication to subscribe to for change data capture"
    },
    "replication_slot_name": {
      "type": "string",
      "description": "Name of the PostgreSQL logical replication slot to use"
    },
    "error_handler_path": {
      "type": "string",
      "description": "Path to a script or flow to run when the triggered job fails"
    },
    "error_handler_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "retry": {
      "type": "object",
      "properties": {
        "constant": {
          "type": "object",
          "description": "Retry with constant delay between attempts",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "seconds": {
              "type": "integer",
              "description": "Seconds to wait between retries"
            }
          }
        },
        "exponential": {
          "type": "object",
          "description": "Retry with exponential backoff (delay doubles each time)",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "multiplier": {
              "type": "integer",
              "description": "Multiplier for exponential backoff"
            },
            "seconds": {
              "type": "integer",
              "minimum": 1,
              "description": "Initial delay in seconds"
            },
            "random_factor": {
              "type": "integer",
              "minimum": 0,
              "maximum": 100,
              "description": "Random jitter percentage (0-100) to avoid thundering herd"
            }
          }
        },
        "retry_if": {
          "$ref": "#/components/schemas/RetryIf"
        }
      },
      "description": "Retry configuration for failed module executions"
    }
  },
  "required": [
    "script_path",
    "is_flow",
    "postgres_resource_path",
    "replication_slot_name",
    "publication_name"
  ]
}
```

## MqttTrigger (`*.mqtt_trigger.yaml`)

Must be a YAML file that adheres to the following schema:

```json
{
  "type": "object",
  "properties": {
    "script_path": {
      "type": "string",
      "description": "Path to the script or flow to execute when triggered"
    },
    "is_flow": {
      "type": "boolean",
      "description": "True if script_path points to a flow, false if it points to a script"
    },
    "mqtt_resource_path": {
      "type": "string",
      "description": "Path to the MQTT resource containing broker connection configuration"
    },
    "subscribe_topics": {
      "type": "array",
      "items": {
        "type": "object"
      },
      "description": "Array of MQTT topics to subscribe to, each with topic name and QoS level"
    },
    "v3_config": {
      "type": "object",
      "properties": {
        "clean_session": {
          "type": "boolean"
        }
      }
    },
    "v5_config": {
      "type": "object",
      "properties": {
        "clean_start": {
          "type": "boolean"
        },
        "topic_alias_maximum": {
          "type": "number"
        },
        "session_expiry_interval": {
          "type": "number"
        }
      }
    },
    "client_id": {
      "type": "string",
      "description": "MQTT client ID for this connection"
    },
    "client_version": {
      "type": "string",
      "enum": [
        "v3",
        "v5"
      ]
    },
    "error_handler_path": {
      "type": "string",
      "description": "Path to a script or flow to run when the triggered job fails"
    },
    "error_handler_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "retry": {
      "type": "object",
      "properties": {
        "constant": {
          "type": "object",
          "description": "Retry with constant delay between attempts",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "seconds": {
              "type": "integer",
              "description": "Seconds to wait between retries"
            }
          }
        },
        "exponential": {
          "type": "object",
          "description": "Retry with exponential backoff (delay doubles each time)",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "multiplier": {
              "type": "integer",
              "description": "Multiplier for exponential backoff"
            },
            "seconds": {
              "type": "integer",
              "minimum": 1,
              "description": "Initial delay in seconds"
            },
            "random_factor": {
              "type": "integer",
              "minimum": 0,
              "maximum": 100,
              "description": "Random jitter percentage (0-100) to avoid thundering herd"
            }
          }
        },
        "retry_if": {
          "$ref": "#/components/schemas/RetryIf"
        }
      },
      "description": "Retry configuration for failed module executions"
    }
  },
  "required": [
    "script_path",
    "is_flow",
    "subscribe_topics",
    "mqtt_resource_path"
  ]
}
```

## SqsTrigger (`*.sqs_trigger.yaml`)

Must be a YAML file that adheres to the following schema:

```json
{
  "type": "object",
  "properties": {
    "script_path": {
      "type": "string",
      "description": "Path to the script or flow to execute when triggered"
    },
    "is_flow": {
      "type": "boolean",
      "description": "True if script_path points to a flow, false if it points to a script"
    },
    "queue_url": {
      "type": "string",
      "description": "The full URL of the AWS SQS queue to poll for messages"
    },
    "aws_auth_resource_type": {
      "type": "string",
      "enum": [
        "oidc",
        "credentials"
      ]
    },
    "aws_resource_path": {
      "type": "string",
      "description": "Path to the AWS resource containing credentials or OIDC configuration"
    },
    "message_attributes": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "description": "Array of SQS message attribute names to include with each message"
    },
    "error_handler_path": {
      "type": "string",
      "description": "Path to a script or flow to run when the triggered job fails"
    },
    "error_handler_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "retry": {
      "type": "object",
      "properties": {
        "constant": {
          "type": "object",
          "description": "Retry with constant delay between attempts",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "seconds": {
              "type": "integer",
              "description": "Seconds to wait between retries"
            }
          }
        },
        "exponential": {
          "type": "object",
          "description": "Retry with exponential backoff (delay doubles each time)",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "multiplier": {
              "type": "integer",
              "description": "Multiplier for exponential backoff"
            },
            "seconds": {
              "type": "integer",
              "minimum": 1,
              "description": "Initial delay in seconds"
            },
            "random_factor": {
              "type": "integer",
              "minimum": 0,
              "maximum": 100,
              "description": "Random jitter percentage (0-100) to avoid thundering herd"
            }
          }
        },
        "retry_if": {
          "$ref": "#/components/schemas/RetryIf"
        }
      },
      "description": "Retry configuration for failed module executions"
    }
  },
  "required": [
    "script_path",
    "is_flow",
    "queue_url",
    "aws_resource_path",
    "aws_auth_resource_type"
  ]
}
```

## GcpTrigger (`*.gcp_trigger.yaml`)

Must be a YAML file that adheres to the following schema:

```json
{
  "type": "object",
  "properties": {
    "script_path": {
      "type": "string",
      "description": "Path to the script or flow to execute when triggered"
    },
    "is_flow": {
      "type": "boolean",
      "description": "True if script_path points to a flow, false if it points to a script"
    },
    "gcp_resource_path": {
      "type": "string",
      "description": "Path to the GCP resource containing service account credentials for authentication."
    },
    "topic_id": {
      "type": "string",
      "description": "Google Cloud Pub/Sub topic ID to subscribe to."
    },
    "subscription_id": {
      "type": "string",
      "description": "Google Cloud Pub/Sub subscription ID."
    },
    "delivery_type": {
      "type": "string",
      "enum": [
        "push",
        "pull"
      ],
      "description": "Delivery mode for messages. 'push' for HTTP push delivery where messages are sent to a webhook endpoint, 'pull' for polling where the trigger actively fetches messages."
    },
    "delivery_config": {
      "type": "object",
      "properties": {
        "audience": {
          "type": "string",
          "description": "The audience claim for OIDC tokens used in push authentication."
        },
        "authenticate": {
          "type": "boolean",
          "description": "If true, push messages will include OIDC authentication tokens."
        }
      },
      "description": "Configuration for push delivery mode."
    },
    "subscription_mode": {
      "type": "string",
      "enum": [
        "existing",
        "create_update"
      ],
      "description": "The mode of subscription. 'existing' means using an existing GCP subscription, while 'create_update' involves creating or updating a new subscription."
    },
    "error_handler_path": {
      "type": "string",
      "description": "Path to a script or flow to run when the triggered job fails."
    },
    "error_handler_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "retry": {
      "type": "object",
      "properties": {
        "constant": {
          "type": "object",
          "description": "Retry with constant delay between attempts",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "seconds": {
              "type": "integer",
              "description": "Seconds to wait between retries"
            }
          }
        },
        "exponential": {
          "type": "object",
          "description": "Retry with exponential backoff (delay doubles each time)",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "multiplier": {
              "type": "integer",
              "description": "Multiplier for exponential backoff"
            },
            "seconds": {
              "type": "integer",
              "minimum": 1,
              "description": "Initial delay in seconds"
            },
            "random_factor": {
              "type": "integer",
              "minimum": 0,
              "maximum": 100,
              "description": "Random jitter percentage (0-100) to avoid thundering herd"
            }
          }
        },
        "retry_if": {
          "$ref": "#/components/schemas/RetryIf"
        }
      },
      "description": "Retry configuration for failed module executions"
    }
  },
  "required": [
    "script_path",
    "is_flow",
    "gcp_resource_path",
    "topic_id",
    "subscription_id",
    "delivery_type",
    "subscription_mode"
  ]
}
```