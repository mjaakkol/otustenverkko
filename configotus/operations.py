import os
import argparse
import logging
import json
import subprocess

from typing import List, Dict
from pathlib import Path

from google.cloud import bigquery

PROJECT_ID = os.getenv('GOOGLE_CLOUD_PROJECT')
DATASET_ID = os.getenv('DATASET_ID')
TABLENAME_TELEMETRY = os.getenv('TABLENAME_TELEMETRY')

client = bigquery.Client()

logger = logging.getLogger(__name__)

def create_dataset(schema_dict: List[Dict[str,str]]):
    full_dataset = f"{PROJECT_ID}.{DATASET_ID}"

    client = bigquery.Client()

    dataset = bigquery.Dataset(full_dataset)

    dataset.location = "US"

    dataset = client.create_dataset(dataset, timeout=30)

    schema = [bigquery.SchemaField(e['name'], e['type'], mode=e['mode'], description=e['description']) for e in schema_dict]

    """
    schema = [
        bigquery.SchemaField("device_id", "STRING", mode="REQUIRED", description="Unique device identified. Often MAC-address"),
        bigquery.SchemaField("time", "DATETIME", mode="REQUIRED", description="Local time"),
        bigquery.SchemaField("temperature", "FLOAT", mode="REQUIRED", description="Temperature"),
        bigquery.SchemaField("humidity", "FLOAT", mode="REQUIRED", description="Humidity"),
        bigquery.SchemaField("voc_index", "INTEGER", mode="REQUIRED", description="Volatile Organic Compound Index"),
    ]
    """
    table_id = f"{full_dataset}.{TABLENAME_TELEMETRY}"

    table = bigquery.Table(table_id, schema=schema)

    table = client.create_table(table)

    logger.info(f"Table has been created for {table_id}")


def delete_dataset():
    FULL_DATASET=f"{PROJECT_ID}.{DATASET_ID}"
    client = bigquery.Client()

    dataset = bigquery.Dataset(FULL_DATASET)

    client.delete_dataset(dataset, delete_contents=True, timeout=10)
    logger.warning("Ilmaotus datasets deleted")


def create_protos(protofile: Path):
    output = Path("..", "pilviotus")
    subprocess.run(["protoc", f"--proto_path={protofile.parent}", f"--python_out={str(output)}", str(protofile)])


def main():
    parser = argparse.ArgumentParser(description='Ilmaotus cloud configuration')
    # Add roadmap file required after development
    parser.add_argument('-s','--schema', help="Schema file for the BigQuery table")
    parser.add_argument('-p', '--project', help="Google cloud project name")
    parser.add_argument('-d', '--dataset', help="Dataset ID")
    parser.add_argument('-t', '--tablename', help="Used tablename")
    command = parser.add_subparsers(dest="command")
    command.add_parser("create", help="Creates BigQuery database")
    command.add_parser("delete", help="Deletes BigQuery database")
    command.add_parser("protos", help="Converts .proto file into language specific format")

    args = parser.parse_args()

    if args.command == 'protos':
        logger.info("Creating protos files databases")
        create_protos(Path("..", "protos", "messages.proto"))
        return

    # The rest of the commands require PROJECT_ID etc. to work
    if args.project:
        global PROJECT_ID
        PROJECT_ID = args.project
        logger.warning(f"Project overwritten with {PROJECT_ID}")

    if args.dataset:
        global DATASET_ID
        DATASET_ID = args.dataset
        logger.warning(f"Dataset overwritten with {DATASET_ID}")

    if args.tablename:
        global TABLENAME_TELEMETRY
        TABLENAME_TELEMETRY = args.tablename
        logger.warning(f"Telemetry overwritten with {TABLENAME_TELEMETRY}")

    if args.command == 'create':
        logger.info("Creating Ilmaotus databases")
        data = json.load(open(args.schema, 'r'))
        create_dataset(data)
    elif args.command == 'delete':
        logger.info("Creating Ilmaotus databases")
        delete_dataset()
    else:
        logger.error("Unrecognized command")

if __name__ == "__main__":
    main()
