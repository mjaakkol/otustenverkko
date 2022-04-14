use crate::mqtt::{Client, MqttError, MqttInit, Qos};
use chrono::{self, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use log::{debug, error, info, warn};
use serde;
use serde::{Deserialize, Serialize};
use smol::{
    channel::{Receiver, RecvError},
    future::FutureExt,
};
use std::{fmt::Display, time::Duration};
use thiserror::Error;
//use backoff::{future::FutureOperation as _, ExponentialBackoff};
use backoff::future::retry;
use backoff::ExponentialBackoff;

const DEFAULT_AUTH_TIMEOUT_IN_SEC: i64 = 6400;

/// Google IoT MQTT Error
#[derive(Error, Debug)]
pub enum GcpIoTError {
    /// General error. Typically used for internal operations
    #[error("General error")]
    Error,

    /// Incorrect subscription length
    #[error("Returned incorrect subscription length")]
    ErrorWrongLengthSub,

    /// Incorrect subscription length
    #[error("Returned incorrect subscription length")]
    ErrorSubscriptionFailed,

    /// Error parsing root certificate
    #[error("Error parsing ROOT certificate")]
    ErrorRoot,

    /// Errors derived from jsonwebtoken
    #[error("jsonwebtoken error")]
    JsonWebTokenError {
        /// Cascaded error
        source: jsonwebtoken::errors::Error,
    },

    /// Derived IOError
    #[error("transparent")]
    IOError(#[from] std::io::Error),

    /// Errors derived from sister MQTT module
    #[error("MQTT error: {0}")]
    MqttError(#[from] MqttError),

    /// Errors derived from AsyncChannelRecvError
    #[error("transparent")]
    AsyncChannelRecvError(#[from] RecvError),
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iat: i64,
    exp: i64,
    aud: String,
}

/// Connects the device to Google cloud IoT framework
///
/// Connecting the device to Google cloud authenticating the device. This is internal method and needs to be
/// called prior the expiration period as otherwise the connection will drop.
async fn connect(
    client_id: &str,
    project_id: &str,
    algorithm: &Algorithm,
    expiration: &Duration,
    keep_alive: u16,
    key: &EncodingKey,
    ca_cert: &[u8],
) -> Result<Client, GcpIoTError> {
    let jwt_password = GcpMqtt::create_token(key, project_id, algorithm, expiration)?;

    let client = retry(ExponentialBackoff::default(), || async {
        info!("Creating client");
        //let mut client = Client::with_tls("mqtt.googleapis.com", 8883, ca_cert).await.map_err(GcpMqtt::err_mapper)?;
        let mqtt_init = MqttInit {
            address: "mqtt.googleapis.com".to_string(),
            port: 8883,
            ..Default::default()
        };

        let mut client = mqtt_init
            .with_tls(ca_cert)
            .await
            .map_err(GcpMqtt::err_mapper)?;

        client
            .connect(
                keep_alive,
                &client_id,
                Some("unused"),
                Some(&jwt_password[..]),
                false,
            )
            .await
            .map_err(GcpMqtt::err_mapper)?;
        Ok(client)
    })
    .await?;

    Ok(client)
}

/// Google cloud MQTT client
///
/// Client for engaging with Google IoT hub.
///
pub struct GcpMqtt {
    /// Algorithm for generating token for web token
    algorithm: Algorithm,
    /// Token expiration period. Expiration will cause reauthentiction. This value cannot exceed
    /// 24h - skew (10min).
    expiration: Duration,
    /// Google Core IoT project name
    project_id: String,
    /// ID for IoT device
    device_id: String,
    /// Client ID
    client_id: String,
    /// Client certificate for authentication
    cert: Vec<u8>,
    /// Server certification for authenticating the server
    ca_cert: Vec<u8>,
    config: Receiver<Vec<u8>>,
    command: Receiver<Vec<u8>>,
    client: Client,
    key: EncodingKey,
    keep_alive: u16,
    topic_prefix: String,
}

type RxChannel = Receiver<Vec<u8>>;

/// Defacto subscribed topics for Google IoT Core
pub enum ChannelData {
    /// Configuration data from Google IoT Core
    Config(Vec<u8>),
    /// Command from Google IoT Core
    Command(Vec<u8>),
    /// Timeout. This can be used to signal timeout inside the same enum.
    Timeout,
}

impl GcpMqtt {
    fn err_mapper<E: Display>(err: E) -> GcpIoTError {
        warn!("Initialization error {:?}", err.to_string());
        GcpIoTError::Error
    }

    /// Creates new Google cloud MQTT instance
    ///
    /// Used for setting up MQTT for Google cloud with all necessary parameters.
    ///
    pub async fn new(
        algorithm: Algorithm,
        expiration: Duration,
        project_id: String,
        cloud_region: String,
        registry: String,
        device_id: String,
        keep_alive: u16,
        cert: Vec<u8>,
        ca_cert: &[u8],
    ) -> Result<Self, GcpIoTError> {
        let key = EncodingKey::from_ec_pem(&cert)
            .map_err(|source| GcpIoTError::JsonWebTokenError { source })?;

        // Creating the standard topics to Gcp MQTT. Client topics are added separately
        let topic_prefix = format!("/devices/{}/", device_id);

        let gcp_subscription = vec![
            (topic_prefix.to_owned() + "config", Qos::AtLeastOnce),
            (topic_prefix.to_owned() + "commands/#", Qos::AtLeastOnce),
        ];

        let client_id = format!(
            "projects/{}/locations/{}/registries/{}/devices/{}",
            project_id, cloud_region, registry, device_id
        );

        info!(
            "Project ID:{} Region:{} Registry:{} Device ID:{}, token expiration:{:?} Keep-alive:{}",
            project_id, cloud_region, registry, device_id, expiration, keep_alive
        );

        let mut client = connect(
            &client_id,
            &project_id,
            &algorithm,
            &expiration,
            keep_alive,
            &key,
            ca_cert,
        )
        .await?;

        debug!("MQTTInit done");
        let mut subs = client.subscribe(&gcp_subscription).await?;

        let (config, _) = subs.remove(0)?;
        let (command, _) = subs.remove(0)?;

        info!("GcpMqtt creation succeeded");

        Ok(Self {
            algorithm,
            expiration,
            project_id,
            device_id,
            client_id,
            cert,
            ca_cert: ca_cert.to_vec(),
            config,
            command,
            client,
            key,
            keep_alive,
            topic_prefix,
        })
    }

    fn create_token(
        key: &EncodingKey,
        project_id: &str,
        algorithm: &Algorithm,
        expiration: &Duration,
    ) -> Result<Vec<u8>, GcpIoTError> {
        // Time expiration is so small that failure should equal assert (major programming error) and things should explode
        let base_time = Utc::now();
        let duration = chrono::Duration::from_std(*expiration).unwrap();
        let expiration = base_time.checked_add_signed(duration).unwrap();

        debug!(
            "JWT. Basetime: {} expiration: {} Project id:{:?}",
            base_time, expiration, project_id
        );

        let claims = Claims {
            iat: base_time.timestamp(),
            exp: expiration.timestamp(),
            aud: project_id.to_owned(),
        };

        Ok(encode(&Header::new(*algorithm), &claims, key)
            .map_err(|source| GcpIoTError::JsonWebTokenError { source })?
            .as_bytes()
            .to_vec())
    }

    /// Waiting Google IoT specific subscriptions
    ///
    /// Google cloud has set of default subscriptions for receiving commands and configurations per
    /// the framework.
    ///
    /// # Example
    /// ```
    ///    let packet_result = gcp_mqtt.wait_channels().or(async {
    ///        Timer::after(Duration::from_secs(timeout)).await;
    ///        Err(GcpIoTError::WaitTimeout)
    ///    }).await;
    ///
    ///    match packet_result {
    ///        Ok(ChannelData::Config(packet)) => {
    ///            // This is text of base64 so it is save to cast this to string
    ///            info!("Config packet {:?}", from_utf8(&packet).unwrap());
    ///        },
    ///        Ok(ChannelData::Command(packet)) => {
    ///            info!("Command packet {:?}", packet);
    ///        },
    ///        Err(GcpIoTError::WaitTimeout) => {
    ///            info!("Sending state information");
    ///            gcp_mqtt.set_device_state(&String::from("state")).await.unwrap();
    ///            Timer::after(Duration::from_secs(1)).await;
    ///            gcp_mqtt.connect().await.expect("Re-connect failed!");
    ///            info!("Reconnect succeeded");
    ///        },
    ///        Err(_) => {
    ///            warn!("Test receive loop terminated");
    ///            break;
    ///        }
    ///    }
    /// ```
    pub async fn wait_channels(&self) -> Result<ChannelData, GcpIoTError> {
        let packet: Result<ChannelData, GcpIoTError> = (async {
            let config_data = self.config.recv().await?;
            Ok(ChannelData::Config(config_data))
        })
        .or(async {
            let command_data = self.command.recv().await?;
            Ok(ChannelData::Command(command_data))
        })
        .await;
        packet
    }

    /// Set Google IoT device state
    ///
    /// Google IoT enables the device to set its state to the cloud. The state needs to be in ASCII format.
    /// The state will always be send in QoS 1 (At least once) level.
    ///
    pub async fn set_device_state(&mut self, state: &String) -> Result<&mut Self, GcpIoTError> {
        let topic = self.topic_prefix.to_owned() + "state";
        self.client
            .publish(&topic, Qos::AtLeastOnce, &state.as_bytes().to_vec())
            .await?;
        Ok(self)
    }

    /// Subscribe to a set of topics.
    ///
    /// Enables client to listen given topics with the defined quality of the service. The outer result
    /// indicates how the overall operation performed and each topic inside has a result of its own related to the
    /// status of the subscription.
    ///
    /// The method aggregates the prefix '/devices/*device_id*/ and the caller only need to supply the topic (without '/').
    ///
    /// The returned Receivers channels are used to communicate packets to given topic and they match the order in which the topics were supplied.
    ///
    pub async fn subscribe(
        &mut self,
        mqtt_topics: &Vec<(String, Qos)>,
    ) -> Result<Vec<Result<(Receiver<Vec<u8>>, Qos), MqttError>>, GcpIoTError> {
        // Add the topic management
        let topics: Vec<(String, Qos)> = mqtt_topics
            .iter()
            .map(|(topic, qos)| ((self.topic_prefix.to_owned() + topic), qos.clone()))
            .collect();

        info!("Subscribing into {:?}", topics);

        Ok(self.client.subscribe(&topics).await?)
    }

    /// Unsubscribe client from topics
    ///
    /// By unsubscribing from given topic, the client stops receiving the messages destinated to it.
    ///
    ///
    pub async fn unsubscribe(&mut self, topics: &Vec<String>) -> Result<&Self, GcpIoTError> {
        self.client.unsubscribe(topics).await?;
        Ok(self)
    }

    /// Disconnect from MQTT broker
    ///
    /// Send disconnect message to broker. This leads into breaking the socket so new Client needs to be created
    /// for new connections
    ///
    /// Example
    ///
    /// TODO Create example
    ///
    pub async fn disconnect(&mut self) -> Result<&Self, GcpIoTError> {
        self.client.disconnect().await?;
        Ok(self)
    }

    /// Publish message to given topic
    ///
    /// Publishes message with the topic and QoS-class. Google cloud only support Qos-classes 0 and 1 for now.
    /// The method aggregates the prefix '/devices/*device_id*/ and the caller only need to supply the topic (without '/') and data.
    ///
    /// Example
    ///
    /// TODO Create example
    ///
    pub async fn publish(
        &mut self,
        topic: &str,
        qos: Qos,
        data: &Vec<u8>,
    ) -> Result<&Self, GcpIoTError> {
        //let full_topic = self.topic_prefix.to_owned() + format!("events/{}", topic);
        let full_topic = format!("{}events/{}", self.topic_prefix, topic);
        self.client.publish(&full_topic, qos, data).await?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use env_logger;
    use futures_lite::future::FutureExt;
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
    use log::{info, warn};
    use smol::{block_on, spawn, Timer};
    use std::{env, fs::File, io::Read, path::Path, str::from_utf8, time::Duration};

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_gcp_connection() {
        init();

        let path = env::var("CERT").unwrap();
        let path = Path::new(&path);

        info!("Cert-path {:?}", path);

        let mut f = File::open(path).unwrap();

        let mut user_cert = Vec::new();
        // read the whole file
        f.read_to_end(&mut user_cert).unwrap();

        let root_path = env::var("GOOGLE_ROOT_CERT").unwrap();
        let root_path = Path::new(&root_path);

        let mut f = File::open(root_path).unwrap();

        let mut ca_cert = Vec::new();
        // read the whole file
        f.read_to_end(&mut ca_cert).unwrap();

        let project_id = env::var("PROJECT_ID").unwrap();
        let cloud_region = env::var("REGION").unwrap();
        let registry_id = env::var("REGISTRY_ID").unwrap();
        let device_id = env::var("DEVICE_ID").unwrap();

        block_on(async move {
            let mut gcp_mqtt = GcpMqtt::new(
                Algorithm::ES256,
                Duration::from_secs(1200),
                project_id,
                cloud_region,
                registry_id,
                device_id,
                30,
                user_cert,
                &ca_cert,
            )
            .await
            .unwrap();

            spawn(async move {
                let timeout = 3;
                loop {
                    let packet_result = gcp_mqtt
                        .wait_channels()
                        .or(async {
                            Timer::after(Duration::from_secs(timeout)).await;
                            Ok(ChannelData::Timeout)
                        })
                        .await;

                    match packet_result {
                        Ok(ChannelData::Config(packet)) => {
                            // This is text of base64 so it is save to cast this to string
                            info!("Config packet {:?}", from_utf8(&packet).unwrap());
                        }
                        Ok(ChannelData::Command(packet)) => {
                            info!("Command packet {:?}", packet);
                        }
                        Ok(ChannelData::Timeout) => {
                            info!("Sending state information");
                            gcp_mqtt
                                .set_device_state(&String::from("state"))
                                .await
                                .unwrap();
                            Timer::after(Duration::from_secs(1)).await;
                            gcp_mqtt.connect().await.expect("Re-connect failed!");
                            info!("Reconnect succeeded");
                        }
                        Err(_) => {
                            warn!("Test receive loop terminated");
                            break;
                        }
                    }
                }
            })
            .or(async {
                Timer::after(Duration::from_secs(6)).await;
            })
            .await;

            //gcp_mqtt.disconnect().await.expect("Disconnect failed");

            info!("test_connection to wrap up");
        });
    }

    #[test]
    fn test_jwt() {
        init();

        let path = env::var("CERT").expect("CERT environment variable not set");
        let path = Path::new(&path);

        let mut f = File::open(path).unwrap();

        let mut private_cert = Vec::new();
        // read the whole file
        f.read_to_end(&mut private_cert).unwrap();

        let path = env::var("PUBLIC_CERT").expect("PUBLIC_CERT environment variable not set");
        let path = Path::new(&path);

        let mut f = File::open(path).unwrap();

        let mut public_cert = Vec::new();
        // read the whole file
        f.read_to_end(&mut public_cert).unwrap();

        let key = EncodingKey::from_ec_pem(&private_cert).unwrap();

        let token = GcpMqtt::create_token(
            &key,
            &String::from("test_project"),
            &Algorithm::ES256,
            &Duration::from_secs(1200),
        )
        .unwrap();

        let jwt = from_utf8(&token).unwrap();

        let decode_key = DecodingKey::from_ec_pem(&public_cert).unwrap();

        let decode_jwt = decode::<Claims>(&jwt, &decode_key, &Validation::new(Algorithm::ES256));

        info!("Decoded token: {:?}", decode_jwt);

        //info!("Decoded token: {:?}", decode_jwt);
    }
}
