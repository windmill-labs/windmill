from wmill import get_resource
import smtplib


def main(smtp_config: dict, to: str, subject: str, body: str):
    server = smtplib.SMTP(host=smtp_config["host"], port=smtp_config["port"])
    server.ehlo()
    server.starttls()
    server.login(smtp_config["user"], smtp_config["password"])

    server.sendmail(
    smtp_config["user"],
    to,
    f"""Subject: {subject}

{body}
    """)


