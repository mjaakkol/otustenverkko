import logging
import base64
import os
import time

from datetime import datetime

import firebase_admin
import pandas as pd

from google.cloud import bigquery
from firebase_admin import credentials, firestore
from google.cloud import pubsub_v1

import messages_pb2 as pb

logger = logging.getLogger(__name__)


PROJECT_ID = os.getenv('CLOUD_PROJECT')
DATASET_ID = os.getenv('DATASET_ID')
TABLENAME_TELEMETRY = os.getenv('TABLENAME_TELEMETRY')


if os.getenv('K_SERVICE'):
    logger.warning("Deploying in Google cloud")
    #cred = credentials.ApplicationDefault()
    #firebase_admin.initialize_app(cred, {'projectId': "otustenverkko"})
    firebase_admin.initialize_app()
    logger.info("Deployed in Google cloud")
else:
    logger.warning("Deploying locally")
    firebase_service_json = os.getenv('FB_SERVICE_JSON')
    # This is run locally
    firebase_admin.initialize_app(
        credentials.Certificate(firebase_service_json),
    )


def message_callback(message):
    print(f"message:{message}")
    context = {
        "timestamp": time.time()
    }
    process_message(message.attributes, message.data, context)
    message.ack()

def process_message(attributes, data, context):
    device_id = attributes['deviceId']
    TABLE_ID = f"{PROJECT_ID}.{DATASET_ID}.{TABLENAME_TELEMETRY}"

    logger.info(f"DeviceId:{device_id} Table:{TABLE_ID}")

    msg = pb.EnvironmentDataBlocks()
    msg.ParseFromString(data)

    refined_entries = [
        (
            device_id,
            time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(entry.time)),
            entry.temperature_k,
            entry.humidity_rh,
            entry.voc_index
        )
        for entry in msg.blocks
    ]

    #print(refined_entries)
    client = bigquery.Client()

    # Take existing results first
    QUERY = f"""
        SELECT
            d.humidity as humidity,
            d.temperature as temperature,
            d.voc_index as voc_index,
            d.time as time
        FROM
            `{TABLE_ID}` d
        WHERE
            timestamp(d.time) between timestamp_sub(current_timestamp, INTERVAL 12 HOUR) and current_timestamp()
            and d.device_id = '{device_id}'
        ORDER BY
            d.time
    """

    # We should be able to reuse the previous data to reduce the number of items queries but that
    # optimization is for the future generations :-)
    query_job = client.query(QUERY)

    bq_df = query_job.result().to_dataframe()

    pb_df = pd.DataFrame.from_records(
        refined_entries,
        exclude=['device_id'],
        columns=['device_id','time','temperature','humidity','voc_index']
    )

    # Merge dataframes coming from Big Query and protobufs into linear dataframe
    df = pd.concat((bq_df, pb_df))

    df.loc[:, 'time'] = pd.to_datetime(df['time'], format="%Y-%m-%d %H:%M:%S")

    df.set_index('time', inplace=True)

    df = df.resample('4T').mean().dropna().astype(int)
    history_results_array = df.to_dict('records')

    db = firestore.client()

    doc_ref = db.collection('telemetry_history').document(device_id)

    history_results = {
        'updatetime': context.timestamp,
        'data': history_results_array
    }

    doc_ref.set(history_results)

    # Insert new entries into BiqQuery
    dataset_ref = client.dataset(DATASET_ID, project=PROJECT_ID)
    table_ref = dataset_ref.table(TABLENAME_TELEMETRY)
    table = client.get_table(table_ref)

    errors = client.insert_rows(table, refined_entries)

    if errors:
        logger.error(f"Inserting basic rows failed with {errors}")
        return

    # Insert summary data to Firestore
    QUERY = f"""
        SELECT
            avg(d.temperature) as tKavg,
            avg(d.humidity) as RHavg,
            avg(d.voc_index) as vocIdxAvg,
            min(d.temperature) as tKmin,
            min(d.humidity) as RHmin,
            min(d.voc_index) as vocMin,
            max(d.temperature) as tKmax,
            max(d.humidity) as RHmax,
            max(d.voc_index) as vocMax
        FROM
            `{TABLE_ID}` d
        where
            timestamp(d.time) between timestamp_sub(current_timestamp, INTERVAL 1 DAY) and current_timestamp()
            and d.device_id = '{device_id}'
        """

    query_job = client.query(QUERY)

    try:
        for row in query_job:
            #logger.info(row)
            result_dict = {key: float(value) for key, value in row.items()}
    except Exception as exp:
        logger.error(f"Query failed with {exp}")
        return

    result_dict['tK'] = refined_entries[-1][2]
    result_dict['RH'] = refined_entries[-1][3]
    result_dict['vocIdx'] = refined_entries[-1][4]

    # Update firebase for summary data
    doc_ref = db.collection('telemetry').document(device_id)
    try:
        doc_ref.update(result_dict)
    except:
    # I really want to have update but the first time it will
    # fail if it don't exist
        logger.warning(f"Updating the current telemetry failed so creating it from the scratch")
        doc_ref.set(result_dict)

############### Methods for firebase functions ################

def create_firestore_device(data, context):
    """Creates device into firestore. This is attached to Firestore Write trigger"""
    from google.cloud import iot_v1

    path_parts = context.resource.split('/documents/')[1].split('/')
    #collection_path = path_parts[0]
    device_id = '/'.join(path_parts[1:])

    #logger.info("collection: {} document path: {}".format(collection_path, document_path))
    value = data['value']

    db = firestore.client()

    if value:
        #logger.warning(json.dumps(data["value"]))
        fields      = value['fields']
        #device_id   = fields['device_id']['stringValue']
        project_id  = fields['project_id']['stringValue']
        region      = fields['region']['stringValue']
        registry    = fields['registry']['stringValue']

        client = iot_v1.DeviceManagerClient()

        # TODO: These should probably come as part of the device as it knows best
        # where it comes from and etc.
        name = client.device_path(project_id, region, registry, device_id)

        try:
            client.get_device(name)
        except Exception as e:
            logger.error(f"Registering {device_id} failed due to {e}")
        else:
            # Device exists so we can continue provisioning
            record = {
                'location': fields['location']['stringValue'],
                'owner': fields['owner']['stringValue'],
                'type': int(fields['type']['integerValue']),
                'message_tokens': {
                    'owner': fields['message_token']['stringValue'],
                },
                'sharedWith': [],
            }
            db.collection('devices').document(device_id).set(record)

        db.collection('pending').document(device_id).delete()
    else:
        logger.warning("Empty write")


def cloud_function_pubsub_handler(event, context):
    process_message(event['attributes'], base64.b64decode(event['data']), context)


def subscribe():
    subscriber = pubsub_v1.SubscriberClient()
    topic_name = 'projects/{project_id}/topics/{topic}'.format(
        project_id=os.getenv('GOOGLE_CLOUD_PROJECT'),
        topic='data',  # Set this to something appropriate.
    )

    subscription_name = 'projects/{project_id}/subscriptions/{sub}'.format(
        project_id=os.getenv('GOOGLE_CLOUD_PROJECT'),
        sub='data',  # Set this to something appropriate.
    )

    print(f"Subsription name {subscription_name}")

    streaming_pull_future  = subscriber.subscribe(subscription_name, message_callback)

    with subscriber:
        try:
            # When `timeout` is not set, result() will block indefinitely,
            # unless an exception is encountered first.
            #streaming_pull_future.result(timeout=timeout)
            streaming_pull_future.result()
        except TimeoutError:
            streaming_pull_future.cancel()

if __name__ == "__main__":
    subscribe()