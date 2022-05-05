import requests
import os

def main(
    my_value: str
):
    headers={'Authorization': "Bearer: {}".format(os.environ.get("WM_TOKEN"))}
    r = requests.post('https://httpbin.org/post', data={'key': my_value})
    return r.text
