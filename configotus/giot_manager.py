import logging
import os

from pathlib import Path

from google.cloud import pubsub
from google.cloud import iot_v1
from google.api_core.exceptions import GoogleAPICallError, AlreadyExists, NotFound
from google.protobuf import field_mask_pb2 as gp_field_mask

import keys

logger = logging.getLogger(__name__)

class GotusManager(object):
    """Represents the state of the server."""

    def __init__(self, project_id: str, cloud_region: str, registry_id: str):
        """
        Initializes the server object. Project id, location and registry.
        Class assumes that the parameters are supplied even if they haven't been created yet.
        The following function will then take care of the creation process.
        """
        self._project_id = project_id
        self._region = cloud_region
        self._registry = registry_id

        self._client = iot_v1.DeviceManagerClient()


    def create_registry(self, pubsub_topics):
        """Creates device registry into IoT Core

        Args:
            pubsub_topics (str): Either single (in str) or multiple (list of str) pre-created pubsub topics

        Returns:
            bool: The result of the registry creation operation.
        """
        parent = f"projects/{self._project_id}/locations/{self._region}"

        if type(pubsub_topics) == str:
            pubsub_topics = [pubsub_topics]

        topics = [
            {"pubsub_topic_name": f"projects/{self._project_id}/topics/{t}"}
             for t in pubsub_topics
             ]

        device_registry = {
            "id": self._registry,
            "event_notification_configs": topics
        }

        try:
            return bool(self._client.create_device_registry(parent=parent, device_registry=device_registry))

        except AlreadyExists:
            logger.error("Registry already exists")
            raise

        except GoogleAPICallError as e:
            logger.error(e)
            raise


    def delete_device_registry(self):
        """Delete the whole device registry
        Deletes the device registry including all the devices within it.
        """
        name = self._client.registry_path(self._project_id, self._region, self._registry)

        try:
            self._client.delete_device_registry(name=name)
        except NotFound:
            logger.warning(f"Deleting registry failed. No registry {self._registry} found.")


    def modify_device_config(self, device_id: str, data: bytes, version: int = 0):
        name = self._client.device_path(
            self._project_id,
            self._region,
            self._registry,
            device_id)

        return self._client.modify_cloud_to_device_config(name, data, version_to_update=version)


    def create_device(
        self, device_id: str, country:str, province:str, locality_name: str, organization: str,
        common_name:str, validity_in_days: int, destination_path: Path) -> bool:
        """Adds new device to the existing registry
        Adding the device will trigger a creation and automatic insertion of

        Args:
            device_id (str): IoT Core device id
            country (str): Certificate country information
            province (str): Certificate province or state information
            locality_name (str): Certificate locality (eg. city) information
            organization (str): Certificate organization information
            common_name (str): Certificate common name
            validity_in_days (int): Certificate validity period.

        Returns:
            [type]: [description]
        """
        key_pair = keys.keys(country, province, locality_name, organization, common_name, validity_in_days)

        cert = key_pair.public_x509_pem

        device_template = {
            'id': device_id,
            'credentials': [{
                'public_key' :{
                    'format': 'ES256_X509_PEM',
                    'key': cert
                },
            }]
        }

        parent = self._client.registry_path(self._project_id, self._region, self._registry)

        try:
            response = self._client.create_device(parent=parent, device=device_template)
        except AlreadyExists:
            # This is not treated an error right now but the command is just ignored
            logger.warning(f"Device {device_id} already exists")
            return False

        except GoogleAPICallError as e:
            logger.error(e)
            raise

        with open(destination_path.joinpath(f"{device_id}_X509.pem"), "wb") as f:
            f.write(key_pair.private_pem)

        return True

    def delete_device(self, device_id:str):
        """Delete device from device registry

        Args:
            device_id (str): device id
        """
        name = self._client.device_path(self._project_id, self._region, self._registry, device_id)
        self._client.delete_device(name=name)

    def get_device(self, device_id:str):
        """gets the device from device registry

        Args:
            device_id (str): device id

        Returns:
            [Device resource information or None]: Device resource information or None if the device is not found.
        """
        client = iot_v1.DeviceManagerClient()
        device_path = client.device_path(self._project_id, self._region, self._registry, device_id)

        field_mask = gp_field_mask.FieldMask(
            paths=[
                "id",
                "name",
                "num_id",
                "credentials",
                "last_heartbeat_time",
                "last_event_time",
                "last_state_time",
                "last_config_ack_time",
                "last_config_send_time",
                "blocked",
                "last_error_time",
                "last_error_status",
                "config",
                "state",
                "log_level",
                "metadata",
                "gateway_config",
            ]
        )

        try:
            device = client.get_device(request={"name": device_path, "field_mask": field_mask})

            logger.info("Id : {}".format(device.id))
            logger.info("Name : {}".format(device.name))
            logger.info("Credentials:")

            if device.credentials is not None:
                for credential in device.credentials:
                    keyinfo = credential.public_key
                    logger.info("\tcertificate: \n{}".format(keyinfo.key))

                    if keyinfo.format == 4:
                        keyformat = "ES256_X509_PEM"
                    elif keyinfo.format == 3:
                        keyformat = "RSA_PEM"
                    elif keyinfo.format == 2:
                        keyformat = "ES256_PEM"
                    elif keyinfo.format == 1:
                        keyformat = "RSA_X509_PEM"
                    else:
                        keyformat = "UNSPECIFIED_PUBLIC_KEY_FORMAT"
                    logger.info("\tformat : {}".format(keyformat))
                    logger.info("\texpiration: {}".format(credential.expiration_time))

            logger.info("Config:")
            logger.info("\tdata: {}".format(device.config.binary_data))
            logger.info("\tversion: {}".format(device.config.version))
            logger.info("\tcloudUpdateTime: {}".format(device.config.cloud_update_time))

            return device
        except NotFound:
            logger.error("Device not found")
            return None


    def enumerate_devices(self) -> list:
        """Enumerate all the devices from the list.

        Returns:
            list: List of devices found in registry
        """
        reg_path = self._client.registry_path(self._project_id, self._region, self._registry)
        return [e.id for e in self._client.list_devices(parent=reg_path)]

    def create_iot_topic(self, topic:str):
        pubsub_client = pubsub.PublisherClient()
        topic_path = pubsub_client.topic_path(self._project_id, topic)

        topic = pubsub_client.create_topic(topic_path)
        policy = pubsub_client.get_iam_policy(topic_path)

        policy.bindings.add(
            role="roles/pubsub.publisher",
            members=["serviceAccount:cloud-iot@system.gserviceaccount.com"],
        )

        pubsub_client.set_iam_policy(topic_path, policy)


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
    parser.add_argument("-d", "--destination", default=Path("."), help="The destination path for client certificates")

    command = parser.add_subparsers(dest="command")
    command.add_parser("create", help="Creates new registry")
    command.add_parser("add", help="Adds new device into registry")
    command.add_parser("remove", help="Remove device from registry")
    command.add_parser("delete", help="Delete registry")
    command.add_parser("enumerate", help="Enumerate all the devices")
    command.add_parser("get", help="Get the device resources")
    args = parser.parse_args()

    # Service account is one (but not the only) way of authenticating with the cloud
    if args.service_account_json:
        os.environ.set("GOOGLE_APPLICATION_CREDENTIALS", args.service_account_json)

    manager = GotusManager(args.project, args.location, args.registry)

    if args.command == "create":
        manager.create_registry(args.topics)
    elif args.command == "add":
        manager.create_device(args.id, "US", "CA", "San Diego", "Team Otus", "otus.com", 2000, args.destination)
    elif args.command == "remove":
        manager.delete_device(args.id)
    elif args.command == "delete":
        manager.delete_device_registry()
    elif args.command == "enumerate":
        manager.enumerate_devices()
    elif args.command == "get":
        manager.get_device(args.id)
    else:
        manager.logger("Invalid command found")


if __name__ == "__main__":
    main()
