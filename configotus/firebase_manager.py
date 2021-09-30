import logging

from firebase_admin import initialize_app, credentials, firestore

logger = logging.getLogger(__name__)

def add_device(device_id: str, name: str, characteristics: int, user: str):
    logger.info(f"Add device:{device_id} name:{name}")
    db = firestore.client()

    device_meta = db.collection('devices').document(device_id).get()

    if device_meta.exists:
        logger.warning("Device already exists")

        data = device_meta.to_dict()

        old_users = data["users"]
        old_users.append(user)

        db.collection('devices').document(device_id).update({ "users": old_users})

        # If it does, we are not overriding name or anything but just adding the user
    else:
        logger.info(f"Creating new device {device_id} with name:{name}")
        data = {
            'name': name,
            'users': [user],
            'type': characteristics,
        }
        db.collection('devices').document(device_id).set(data)


def delete_device(device_id: str):
    logger.info(f"delete device:{device_id}")
    db = firestore.client()
    db.collection('devices').document(device_id).delete()


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Provision Otustenverkko devices to the cloud")

    parser.add_argument("-n", "--name", help='Friendly name of IoT device')
    parser.add_argument("-i", "--id", required=True, help='IoT device identification used throughout the system')
    parser.add_argument("-u", "--user", help='Owner Firebase ID')
    parser.add_argument("-c", "--characteristics", help='Device capability bitmask')
    parser.add_argument("-f", "--firebase_json", required=True, help='Firebase Admin JSON file')

    command = parser.add_subparsers(dest="command")
    command.add_parser("add", help="Adds new device to the user")
    command.add_parser("remove", help="Removes the device from the user")

    args = parser.parse_args()

    initialize_app(
        credentials.Certificate(args.firebase_json),
    )

    if args.command == "add":
        if not args.name or not args.user or not args.characteristics:
            logger.error("Name, user-id and type must be supplied")
            return

        add_device(args.id, args.name, args.characteristics, args.user)
    elif args.command == "remove":
        delete_device(args.id)


if __name__ == "__main__":
    main()