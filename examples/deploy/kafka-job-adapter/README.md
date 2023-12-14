Processing Kafka events in Windmill
===================================

This example shows how to leverage Windmill to listen to a Kafka topic and trigger actions when a new message is published.

The logic is simple. We will create a Windmill Perpetual script that will listen for events coming from the Kafka topic. On every event received, the listener will spin off a Windmill job with the content of the event.

For the purpose of the demo, here the consumer of the event will only print the event content, but you can make it do whatever you want with it (ping a Slack channel, update a database table, etc.)

### Setup

First, we're going to setup a stack with the following:
- Kafka + Zookeeper to have a working Kafka instance to play with
- A Windmill cluster composed of one server and 2 workers. We need 2 workers here to be able to run at least 2 jobs in parallel (the listener and the producer). If you are fine writing to Kafka using the CLI, then one worker will be enough.

We provide a [docker-compose.yml](./docker-compose.yml) to easily build this stack:
```
docker compose up -d
```

### Create a Kafka topic

The easiest is to do it via Windmill, but you can do it with Kafka CLI if you're familiar with it. Go to [your local Windmill](http://localhost:8000) and create a python script "kafka_create_topic" with the following content:

```python
from confluent_kafka.admin import AdminClient, NewTopic

def main(topic_name:str = "windmill-events" ):
    admin_client = AdminClient({'bootstrap.servers': 'kafka:9092'})

    new_topic = NewTopic(topic_name)
    topic_created = admin_client.create_topics([new_topic])

    for topic, response in topic_created.items():
        try:
            response.result()
            print("Topic {} created".format(topic))
        except Exception as e:
            raise Exception("Failed to create topic {}: {}".format(topic, e))
```
You can then run this script with topic name of your choice. For the rest of this page, we will use the topic: `windmill-events`.

<details>
<summary>Want to do it from the terminal?</summary>
Run the following command to create the topic from within the Kafka container:

```bash
docker exec -it $KAFKA_CONTAINER_ID kafka-topics.sh --create --topic windmill-events --bootstrap-server localhost:9092
```

</details>

### Create a topic listener in Windmill

As said in the intro, the purpose of this script is to listen to the `windmill-events` topic and trigger another Windmill job when a message is received. The content is quite simple:

```python
import wmill
from confluent_kafka import Consumer

def main(kafka_topic: str = "windmill-events"):
    client = Consumer({
        'bootstrap.servers': 'kafka:9092',
        'group.id': 'windmill',
        'auto.offset.reset': 'earliest'
    })

    client.subscribe([kafka_topic])
    msg = client.poll(timeout=60 * 5) # 5 minutes
    client.close()

    if msg is None:
        print("No message after timeout. Returning")
        return
    if msg.error():
        raise Exception("Consumer error: {}".format(msg.error()))

    payload = msg.value().decode('utf-8')
    print('Message received. Scheduling consuming job'.format(payload))
    wmill.run_script_async(hash_or_path="u/admin/consume_message", args={"msg": payload})
    return 
```

Lastly, before deploying the script, be sure to toggle the "Perpetual Script" toggle in the script settings. As a Perpetual script, Windmill will make sure to restart a new job every time one finishes. Every time a message is received (of the poll times out), the script will trigger the consuming job and Windmill will take care of starting another one to listen to the next event.

Lastly, we need to create `u/admin/consume_message` script> As said previously, for the purpose of the demo it only prints the message content:

```python
def main(
    msg: str
):
    print("A message has been received: {}".format(msg))
```

You can start the listiner script.

### Publish messages to the Kafka topic

Finally, we need to publish messages to the Kafka topic. It can be done with the Kafka CLI, but why not doing it in Windmill? Here is the script:

```python
from confluent_kafka import Producer

def main(kafka_topic: str = "windmill-events", msg: str = "Hello World!"):
    client = Producer({
        'bootstrap.servers': 'kafka:9092',
    })

    client.poll(0)
    client.produce(kafka_topic, msg.encode('utf-8'), callback=delivery_callback)
    client.flush()
    return

def delivery_callback(err, msg):
    if err is not None:
        print('Publishing message failed: {}'.format(err))
    else:
        print('Message delivered')
```

<details>
<summary>Want to do it from the terminal?</summary>
Run the following log into the Kafka container and run the `kafka-console-producer.sh` helper:

```bash
docker exec -it $KAFKA_CONTAINER_ID kafka-console-producer.sh --topic windmill-events --bootstrap-server localhost:9092
```

One line is one message sent to the topic
</details>

Every time you execute this script, the listener will trigger the consuming script with the message payload, and Windmill will restart it immediately!
