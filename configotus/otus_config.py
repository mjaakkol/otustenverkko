import logging
import os
import sys

from pathlib import Path

from firebase_admin import initialize_app, credentials

from giot_manager import GotusManager

from firebase_manager import add_device, delete_device

logger = logging.getLogger(__name__)

class OtusManager:
    def __init__(self, project_id: str, cloud_region: str, registry_id: str):
        self._gotus = GotusManager(project_id, cloud_region, registry_id)

    def create_registry(self, pubsub_topics):
        self._gotus.create_registry(pubsub_topics)

    def delete_registry(self):
        for d in self._gotus.enumerate_devices():
            self._gotus.delete_device(d)

        self._gotus.delete_device_registry()

    def create_device(
        self,
        device_id: str,
        country:str,
        province:str,
        locality_name: str,
        organization: str,
        common_name:str,
        validity_in_days: int,
        name: str,
        characteristics: int,
        user: str,
        destination: Path
        ) -> bool:
        result = self._gotus.create_device(
            device_id, country, province, locality_name, organization, common_name,
            validity_in_days, destination)

        if result:
            add_device(device_id, name, characteristics, user)

        return result


    def delete_device(self, device_id:str):
        self._gotus.delete_device(device_id)
        delete_device(device_id)


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Google IoT Manager provisioning")

    # Mandatory parameters
    parser.add_argument("-p", "--project", required=True, help='Project name')
    parser.add_argument("-l", "--location", required=True, help='Region name')
    parser.add_argument("-r", "--registry", required=True, help='Registry name')

    # Optional per command specific parameters
    parser.add_argument("-i", "--id", help="Device id")
    parser.add_argument("-t", "--topics", help="Topic. If multiple use to comma to separate")
    parser.add_argument("-s", "--service_account_json", help="Path to the service account json file")
    parser.add_argument("-f", "--firebase_json", required=True, help='Firebase Admin JSON file')
    parser.add_argument("-u", "--user", help='Owner Firebase ID')
    parser.add_argument("-c", "--characteristics", help='Device characteristics')
    parser.add_argument("-n", "--name", help='Friendly name of IoT device')
    parser.add_argument("-d", "--destination", default=".", help="The destination path for client certificates")

    command = parser.add_subparsers(dest="command")
    command.add_parser("create", help="Creates new registry")
    command.add_parser("add", help="Adds new device into registry")
    command.add_parser("remove", help="Remove device from registry")
    command.add_parser("delete", help="Delete registry")
    args = parser.parse_args()

    # Service account is one (but not the only) way of authenticating with the cloud
    if args.service_account_json:
        os.environ.set("GOOGLE_APPLICATION_CREDENTIALS", args.service_account_json)

    initialize_app(
        credentials.Certificate(args.firebase_json),
    )

    manager = OtusManager(args.project, args.location, args.registry)

    if args.command == "create":
        manager.create_registry(args.topics)
    elif args.command == "add":
        result = manager.create_device(
                args.id, "US", "CA", "San Diego", "Team Otus", "otus.com", 2000,
                args.name, args.characteristics, args.user, Path(args.destination)
                )
    elif args.command == "remove":
        manager.delete_device(args.id)
    elif args.command == "delete":
        manager.delete_registry()
    else:
        manager.logger("Invalid command found")


if __name__ == "__main__":
    main()
