import os
import wmill
from slack_sdk.web.client import WebClient


def main(
    slack_resource: dict,
    text: str,
    channel: str = None,
    user: str = None,
):
    slack_client = WebClient(token=slack_resource["token"])

    if channel == "":
        channel = None
    if user == "":
        user = None
        
    if channel is None and user is None or (channel is not None and user is not None):
        raise Exception("one and only one of channel or user need to be set")

    if user is not None:
        channel = "@{}".format(user)    
    slack_client.chat_postMessage(channel=channel, text=text)