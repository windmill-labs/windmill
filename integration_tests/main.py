import datetime


def main():
    datestr = "2023-12-26T14:28:20.100189Z".strip("Z") + "+00:00"
    last_run = datetime.datetime.fromisoformat(datestr).astimezone(
        datetime.timezone.utc
    )
    now = datetime.datetime.now(datetime.timezone.utc)
    print(now)
    print(last_run)


if __name__ == "__main__":
    main()
