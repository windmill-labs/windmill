import os
from slack_sdk.web.client import WebClient
from datetime import date, datetime


def main(
    slack_resource: dict,
    img_data: bytes,
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

    tmp_image = "image.png"
    with open(tmp_image, "wb") as fh:
        fh.write(img_data)
    slack_client.files_upload(
        file=tmp_image, initial_comment="Weekly report", channels=channel
    )
    
    now = datetime.now()
    current_time = now.strftime("%H:%M")
    today = date.today()
    print("Sent to slack successfully on", today, current_time)
    
    
