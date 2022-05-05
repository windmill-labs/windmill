import os

from slack_sdk.web.client import WebClient
from slack_sdk.webhook.client import WebhookClient

client = WebClient(token=os.environ.get("G_ALL_SLACK_TOKEN"))

def main(response_url: str, text: str):
    webhook = WebhookClient(response_url)
    # Send a reply
    webhook.send(text=f"You said '{text}'")

    # Send a message to a slack channel
    client.chat_postMessage(
        channel="demobot",
        text="{} sent the message {}".format(os.environ.get("WM_USERNAME"), text)
    )