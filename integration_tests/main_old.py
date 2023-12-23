import docker
import os
import argparse

DEFAULT_VERSION = "main"


def start(args):
    version = args.version
    os.system("WM_VERSION={} docker-compose up -d".format(version))


def upgrade(args):
    version = args.version
    os.system("WM_VERSION={} docker-compose up -d".format(version))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        prog="WindmillDeployctl",
        description="Start and Upgrade a Windmill stack",
        epilog="See Windmill docs at https://www.windmill.dev/docs/intro",
    )

    subparsers = parser.add_subparsers()

    start_p = subparsers.add_parser("start")
    start_p.add_argument(
        "--version",
        default=DEFAULT_VERSION,
        help="Version to start, defaults to latest",
    )
    start_p.set_defaults(func=start)

    upgrade_p = subparsers.add_parser("upgrade")
    upgrade_p.add_argument(
        "--version",
        default=DEFAULT_VERSION,
        help="Version to upgrade to, defaults to latest",
    )
    upgrade_p.set_defaults(func=upgrade)

    args = parser.parse_args()
    if hasattr(args, "func"):
        args.func(args)
    else:
        parser.print_help()
