Service script pattern in Windmill
==================================

This example shows how to use a perpetual script to implement a service in Windmill leveraging Kafka. Services are processes listening on certain events and triggering actions based on the events it received.

The implementation in Windmill is quite simple. First we need a messaging service to listen to. Here we will use Kafka, but it can easily be adapted for others. In Windmill, we are going to implement a Perpetual script that will listen for events coming from a Kafka topic. On every event received, the perpetual script will spin off a Windmill job with the content of the event being passed as an argument to the job.

For the purpose of the demo, here the consumer of the event will only print the event content, but you can make it do whatever you want with it (ping a Slack channel, update a database table, etc.)

### Setup

First, we're going to setup a stack with the following:
- Kafka + Zookeeper to have a working Kafka instance to play with
- A Windmill cluster composed of one server and 2 workers. We need 3 workers here to be able to run multiple jobs in parallel (the listener and the producer). If you are fine sending messages to Kafka using the CLI, then one worker will be enough.

We provide a [docker-compose.yml](./docker-compose.yml) to easily build this stack:
```
docker compose up -d
```

### Create a Kafka topic

The easiest is to do it via Windmill, but you can also do it with Kafka CLI. Go to [your local Windmill](http://localhost:8000) and create a Python script "kafka_create_topic" with the following. it simply creates the topic in Kafka and immediately returns.

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
You can then run this script with topic name of your choice. For the rest of this page, we will use the topic `windmill-events`.

<details>
<summary>Want to do it from the terminal?</summary>
Run the following command to create the topic from within the Kafka container:

```bash
docker exec -it $KAFKA_CONTAINER_ID kafka-topics.sh --create --topic windmill-events --bootstrap-server localhost:9092
```

</details>

### Create a topic listener in Windmill

As said in the intro, the purpose of this perpetual script is to listen to the `windmill-events` topic and trigger another Windmill job when a message is received. The content is quite simple:

```python
from confluent_kafka import Consumer
import wmill

MSG_CONSUMING_JOB_PATH = "u/admin/consume_message"

def main(kafka_topic: str = "windmill-events"):
    client = Consumer({
        'bootstrap.servers': 'kafka:9092',
        'group.id': 'windmill',
        'auto.offset.reset': 'earliest'
    })

    client.subscribe([kafka_topic])

    # The counter i is here to force the perpetual script to exit (and be auto-restarted by 
    # Windmill) after some time, no matter how many messages it has processed. It's a good 
    # practice time-bound jobs in general, and it this particular case it will avoid hitting
    # the maximum logs size
    i = 0
    while i < 10000:
        i += 1
        msg = client.poll(timeout=30) # timeout of 60 seconds

        if msg is None:
            # print("No message after timeout. Looping")
            continue
        if msg.error():
            raise Exception("Consumer error: {}".format(msg.error()))

        payload = msg.value().decode('utf-8')
        print('Message received ({}). Scheduling consuming job'.format(payload))
        wmill.run_script_async(hash_or_path=MSG_CONSUMING_JOB_PATH, args={"msg": payload})

    client.close()
    return 
```

Before deploying the script, don't forget to toggle the "Perpetual Script" toggle in the script settings. As a Perpetual script, Windmill will make sure to restart a new job every time one finishes.

Lastly, we need to create `u/admin/consume_message` script. As said previously, for the purpose of the demo it only prints the message content:

```python
def main(
    msg: str
):
    print("A message has been received: {}".format(msg))
```

The listener script can now be started. It will run perpetually.

### Publish messages to the Kafka topic

Finally, to prove that the above works, we need to publish messages to the Kafka topic. It can be done with the Kafka CLI, but why not doing it in Windmill? Here is a script that will publish 10 messages with random sleep in between:

```python
from confluent_kafka import Producer
import wmill
import random
import time

NUMBER_OR_MSGS = 10
MAX_SLEEP_SECS = 10

def main(kafka_topic: str = "windmill-events", msg: str = "Hello World!"):
    for i in range(NUMBER_OR_MSGS):
        sleep_secs = random.randint(0, MAX_SLEEP_SECS)
        print("Sleeping for {}s".format(sleep_secs))
        time.sleep(sleep_secs)

        client = Producer({
            'bootstrap.servers': 'kafka:9092',
        })

        client.poll(0)
        client.produce(kafka_topic, msg.encode('utf-8'), callback=delivery_callback)
        client.flush()
    return

def delivery_callback(err, msg):
    if err is not None:
        raise Exception('Publishing message failed: {}'.format(err))
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

On every message, the listener will trigger the consuming script with the message payload, and Windmill will restart it immediately!
