use std::{
    fmt,
    io,
    time::{Duration, SystemTime},
    net::ToSocketAddrs,
    marker,
    collections::HashMap,
    sync::Arc,
};

//TODO: If smol will ever include stream.split, the reference for futures crate can be removed.
use futures::io::AsyncReadExt;

use smol::{
    channel::{
        unbounded,
        Sender,
        RecvError,
        Receiver
    },
    io::AsyncWriteExt,
    net,
    future::FutureExt,
    Timer
};

use bytes::BytesMut;
use mqttrs::{
    self,
    Packet,
    Connect,
    Pid,
    Publish,
    Subscribe,
};
use thiserror::Error;

use futures_rustls::{
    TlsConnector,
    rustls::{
        ServerName,
    }
};

use rustls::{
    internal::msgs::handshake::DigitallySignedStruct, Certificate, client::HandshakeSignatureValid,
    client::ServerCertVerified, Error, ClientConfig
};

use x509_parser::prelude::*;
use log::{debug, info, error, warn};
use crate::packets;

const MQTT_DEFAULT: u16 = 1883;

/// Packet future result to implement asynchronous constructs
pub(crate) enum PacketFutureResult {
    /// Packet send successfully
    Ok,
    /// Subscription received either with valid values or with error
    Subscriptions(Vec<Result<(Receiver<Vec<u8>>, Qos), MqttError>>)
}

/// MQTT Subsription
pub(crate) type Subscriptions = Result<PacketFutureResult,MqttError>;

/// Mqtt module errors
#[derive(Error, Debug)]
pub enum MqttError {
    /// Error parsing root certificate
    #[error("Error parsing ROOT certificate")]
    ErrorRoot,

    /// Passing std::io::Error
    #[error("transparent")]
    IOError(#[from] std::io::Error),

    /// Passing mqttrs::Error
    #[error("transparent")]
    MqttPacketError(#[from] mqttrs::Error),

    /// Passing AsyncChannelRecvError
    #[error("transparent")]
    AsyncChannelRecvError(#[from] RecvError),

    /// AsyncChannelSendError
    #[error("AsyncChannelSendError")]
    AsyncChannelSendError,

    /// Error in connecting to hub
    #[error("Connect error")]
    ConnectError,

    /// Error in subscribing to topic
    #[error("Subscribe error")]
    SubscribeError,

    /// Socket timeout
    #[error("Socket connect timeout")]
    SocketConnectTimeout,

    /// Connection timeout
    #[error("Connection timeout")]
    ConnectionTimeout,

    /// Server disconnected client
    #[error("Disconnected")]
    Disconnected,

    /// Error receiving keep-alive
    #[error("Ping response timeout")]
    PingRespTimeout,

    /// Received packet incomplete
    #[error("Received packet incomplete")]
    ReceptionIncomplete,

    /// Parsing PEM failed
    #[error("Parsing PEM failed")]
    ParsingPEM,

    /// Parsing Certificate failed
    #[error("Parsing PEM failed")]
    ParsingCertificateFailed,

    /// Unsupported Qos
    #[error("Unsupported MQTT Qos value {0}")]
    UnsupportedQoS(usize),
}

/// MQTT quality category
#[derive(PartialEq, Clone, Debug)]
pub enum Qos {
    /// MQTT QoS 0 - shoot and forget mode
    AtMostOnce,
    /// MQTT QoS 1 - Requires confirmation but can lead into sending duplicates
    AtLeastOnce,
    /// MQTT QoS 2 - Ensures exact one deliver but most complex and not currently supported
    ExactlyOnce,
}

enum ProcessLoop {
    DataReceived(usize),
    ChannelPacket(ChannelPacket),
    Timeout,
    Error(MqttError)
}


/// Asynchronous MQTT client
///
/// After getting the client, the application needs to connect to the broker with connect and after that the system
/// is ready for other operations. If the connection drops, the application needs to acquire new client through the
/// initiation process.
///
pub struct Client {
    pid: Pid,
    tx_stream: Sender<ChannelPacket>,
}

#[derive(Clone)]
enum ChannelPacket {
    Generic(MqttMsg),
    Connect(MqttMsg,u16),
    RemovePacket(u16)
}

#[derive(Clone)]
struct MqttMsg {
    pid : i32,
    buffer : Arc<BytesMut>,
    req_future: packets::PacketFuture,
}

impl MqttMsg {
    fn new(pid: i32, buffer : BytesMut/*, meta: Meta*/) -> Self {
        Self {
            pid,
            buffer: Arc::new(buffer),
            req_future: packets::PacketFuture::new(),
            //meta
        }
    }

    fn get_packet_future(&self) -> packets::PacketFuture {
        self.req_future.clone()
    }
}

impl fmt::Display for MqttMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mqtt timeout {}", self.pid)
    }
}

impl Client {
    /// Creates new insecure MQTT client
    ///
    /// Creates standard non-TLS MQTT connection. MQTT TCP/IP port is 1883 but some brokers/application can choose to use something
    /// different. Connect needs to be called to finalize the setup and register the client-id etc.
    ///
    fn init_common<T: AsyncReadExt+AsyncWriteExt+Send+Unpin+'static>(stream: T) -> Self {
        let (rx_stream, tx_stream) = stream.split();
        //let rx_stream = stream;
        //let tx_stream = rx_stream.clone();

        let (s, r) = unbounded(); // channels for IPC between receiver and client

        let mut mqtt = Mqtt::new(tx_stream);

        info!("ready_spawn_receiver");

        smol::spawn(async move {
            info!("Entering receive loop");
            mqtt.process_packets(rx_stream, r).await.map_err(|x| {
                warn!("Exited receive loop {:?}",x);
            }).unwrap_err();
        }).detach();

        Self {
            pid: mqttrs::Pid::new(),
            tx_stream: s,
        }
    }

    async fn send_packet<F>(&mut self, f: F) -> Result<PacketFutureResult,MqttError>
        where F: FnOnce(mqttrs::Pid) -> Result<(ChannelPacket,packets::PacketFuture,u16), MqttError> {
        debug!("send_packet");

        let (msg, fut, pid) = f(self.pid)?;
        // Same send method is used for all request/response messages and if pid was not used
        // we are not incrementing it. Zero is special implementation value as MQTT never has zero pid
        // so it is safe to use)
        if pid > 0 {
            self.pid = self.pid + 1;
        }

        // Add timeout but no retry as we assume TCP to take care of that
        self.tx_stream.send(msg).await.map_err( |_| {
            MqttError::AsyncChannelSendError
        })?;

        Ok(fut.or(async {
            Timer::after(Duration::from_secs(4)).await;

            if pid > 0 {
                self.tx_stream.send(ChannelPacket::RemovePacket(pid)).await.map_err( |_| {
                    MqttError::AsyncChannelSendError
                })?;
            }

            error!("send_msg timeout");
            Err(MqttError::ConnectionTimeout)
        }).await?)
    }

    /// Connect MQTT client to the broker
    ///
    /// Used for sending connect request to the broker after Client has been initialized. This method can be called multiple times
    /// during the session if needed (eg. refreshing authentication parameters).
    ///
    pub async fn connect(&mut self, keep_alive: u16, client_id: &str, username: Option<&str>, password: Option<&[u8]>, clean_session: bool) -> Result<&Self, MqttError> {
        info!("--Connect-- client {} username: {:?} password: {:?}", client_id, username, password);
        // Calculate the exact size 10 bytes +2 for password length for the basic header + client_id, username & password

        //let mut raw_buf = Vec::with_capacity(16 + client_id.len() + username.map_or(0, |s| s.len()+1) + password.map_or(0, |s| s.len()+1));
        let mut raw_buf = [0 as u8; 1024];

        self.send_packet(|_| {
            // Encode an MQTT Connect packet.
            let pkt = Connect {
                    protocol: mqttrs::Protocol::MQTT311,
                    keep_alive,
                    client_id,
                    clean_session,
                    last_will: None,
                    username,
                    password,
            }.into();

            // Keeping encoding as hard error as there is nothing on runtime that can be done to fix this. This should crash and hard
            let length = mqttrs::encode_slice(&pkt, &mut raw_buf).expect("Encoding connect packet failed");
            let buf = BytesMut::from(&raw_buf[..length]);

            // Magic number for Connect request as there are no Pids for them.
            let msg = MqttMsg::new((0 as u16).into(), buf);
            let fut = msg.get_packet_future();
            let channel_packet = ChannelPacket::Connect(msg, keep_alive);

            Ok((channel_packet, fut, 0))
        }).await?;

        info!("Connect done");
        Ok(self)
    }

    /// Publish message to given topic
    ///
    /// Publishes message with the topic and QoS-class. Only Qos-classes 0 and 1 are implemented now
    ///
    /// # Example
    /// ```
    /// let data = vec![1,2,3,4,5,6,7,8,9,10];
    ///
    /// let result = amqtt.publish("test/topic", Qos::AtLeastOnce, &data).await;
    /// ```
    pub async fn publish(&mut self, topic_name: &str, qos: Qos, payload: &[u8]) -> Result<&Self, MqttError> {
        info!("--Publish--");
        // Fixed header 2 bytes + variable header fixed part + 4 bytes
        //let mut raw_buf = Vec::with_capacity(topic_name.len()+payload.len()+1);
        let mut raw_buf = [0 as u8; 4096];

        match qos {
            Qos::AtMostOnce => {
                debug!("starting Qos zero publishing");
                // Encode an MQTT Connect packet.
                let pkt = Publish {
                        dup: false,
                        qospid : mqttrs::QosPid::AtMostOnce,
                        retain: false,
                        topic_name,
                        payload
                }.into();

                let length = mqttrs::encode_slice(&pkt, &mut raw_buf)?;
                let buf = BytesMut::from(&raw_buf[..length]);

                let msg = ChannelPacket::Generic(MqttMsg::new(-1, buf));

                self.tx_stream.send(msg).await.map_err( |_| {
                    MqttError::AsyncChannelSendError
                })?;
            },
            Qos::AtLeastOnce | Qos::ExactlyOnce => {
                assert!(Qos::AtLeastOnce == qos);
                debug!("starting Qos non-zero publishing");
                self.send_packet(|pid| {
                    // Encode an MQTT Connect packet.

                    let pkt = mqttrs::Publish {
                                dup: false,
                                qospid : mqttrs::QosPid::AtLeastOnce(pid),
                                retain: false,
                                topic_name,
                                payload
                    }.into();
                    let length = mqttrs::encode_slice(&pkt, &mut raw_buf)?;
                    let buf = BytesMut::from(&raw_buf[..length]);

                    let pid = pid.get();

                    let msg = MqttMsg::new((pid as u16).into(), buf);
                    let fut = msg.get_packet_future();
                    let channel_packet = ChannelPacket::Generic(msg);

                    Ok((channel_packet, fut, pid))
                }).await?;
            }
        }
        Ok(self)
    }

    /// Disconnect from MQTT broker
    ///
    /// Send disconnect message to broker. This leads into breaking the socket so new Client needs to be created
    /// for new connections
    ///
    pub async fn disconnect(&mut self) -> Result<&Self, MqttError> {
        // Allocate write buffer.
        info!("--Disconnect--");
        let mut raw_buf = [0 as u8; 8];

        let pkt = mqttrs::Packet::Disconnect;

        let length = mqttrs::encode_slice(&pkt, &mut raw_buf)?;
        let buf = BytesMut::from(&raw_buf[..length]);

        let msg = ChannelPacket::Generic(MqttMsg::new(-1, buf));
        self.tx_stream.send(msg).await.map_err( |_| {
            MqttError::AsyncChannelSendError
        })?;

        info!("Disconnect complete");
        Ok(self)
    }

    /// Unsubscribe client from topics
    ///
    /// By unsubscribing from given topic, the client stops receiving the messages destinated to it.
    ///
    pub async fn unsubscribe(&mut self, topics: &Vec<String>) -> Result<&Self, MqttError> {
        info!("--Unsubscribe--");
        let mut raw_buf: [u8; 1024] = [0; 1024];

        self.send_packet(|pid| {

            // Encode an MQTT Connect packet.
            let pkt = mqttrs::Unsubscribe {
                pid,
                topics: topics.to_vec(),
            }.into();

            let length = mqttrs::encode_slice(&pkt, &mut raw_buf)?;
            let buf = BytesMut::from(&raw_buf[..length]);

            let pid = pid.get();

            let msg = MqttMsg::new((pid as u16).into(), buf);
            let fut = msg.get_packet_future();
            let channel_packet = ChannelPacket::Generic(msg);

            Ok((channel_packet, fut, pid))
        }).await?;
        Ok(self)
    }

    /// Subscribe to a set of topics.
    ///
    /// Subscribes the client to set of topics with the defined quality of the service. The outer result
    /// indicates how the overall operation performed and each topic inside has a result of its own related to the
    /// status of the subscription.
    ///
    /// The returned Receivers channels are used to communicate packets to given topic and they match the order in which the topics were supplied.
    ///
    /// # Example
    /// ```
    /// let test_subscription = vec![(String::from("test/topic"), Qos::AtLeastOnce),(String::from("test/topic1"), Qos::AtMostOnce)];
    ///
    /// let subs_result = amqtt.subscribe(&test_subscription).await;
    ///
    /// if let Ok(subs) = &subs_result {
    ///     for (i, individual_sub_result) in subs.iter().enumerate() {
    ///         match individual_sub_result {
    ///             Ok((_, qos)) => info!("Subscription for {} succeeded with Qos {:?}", test_subscription[i].0, qos),
    ///             Err(err) => panic!("MQTT error {:?}", err)
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn subscribe(&mut self, mqtt_topics: &Vec<(String, Qos)>) -> Result<Vec<Result<(Receiver<Vec<u8>>, Qos), MqttError>>, MqttError> {
        info!("--Subscribe--");
        // Fixed header 2 bytes + variable header fixed part 3 per topic
        let mut raw_buf: [u8; 1024] = [0; 1024];

        let topics: Vec<mqttrs::SubscribeTopic> = mqtt_topics.into_iter()
                                .map(|(topic_path, qos)| {
                                    mqttrs::SubscribeTopic {
                                        topic_path: topic_path.to_owned(),
                                        qos : match qos {
                                            Qos::AtMostOnce => mqttrs::QoS::AtMostOnce,
                                            Qos::AtLeastOnce => mqttrs::QoS::AtLeastOnce,
                                            Qos::ExactlyOnce => mqttrs::QoS::ExactlyOnce,
                                        },
                                    }
                                })
                                .collect();

        let status = self.send_packet(|pid| {

            // Encode an MQTT Connect packet.
            let pkt = Subscribe {
                    pid,
                    topics,
            }.into();

            let length = mqttrs::encode_slice(&pkt, &mut raw_buf)?;
            let buf = BytesMut::from(&raw_buf[..length]);

            let pid = pid.get();

            let msg = MqttMsg::new((pid as u16).into(), buf);
            let fut = msg.get_packet_future();
            let channel_packet = ChannelPacket::Generic(msg);

            Ok((channel_packet, fut, pid))
        }).await?;

        // We need to use the inner as the outer is async
        info!("End of subscribe");

        if let PacketFutureResult::Subscriptions(subs) = status {
            Ok(subs)
        }
        else {
            panic!("We should never be here");
        }
    }
}


struct Mqtt<T> {
    packets: HashMap<u16, MqttMsg>,
    subscriptions: HashMap<String, Sender<Vec<u8>>>,
    tx_stream: T,
    keep_alive: u16,
    ongoing_ping_reg: bool,
}

impl<T: AsyncWriteExt + marker::Unpin> Mqtt<T> {
    fn new(stream: T) -> Self {
        Self {
            packets : HashMap::with_capacity(8),
            subscriptions : HashMap::with_capacity(8),
            tx_stream : stream,
            keep_alive: u16::MAX,
            ongoing_ping_reg: false
        }
    }

    fn complete_packet(&mut self, pid: &u16) {
        debug!("Enter complete_packet");
        let msg_opt = self.packets.remove(pid);
        if let Some(msg) = msg_opt {
            //TODO: Cascade potential error value. Now everything always work.
            msg.req_future.complete_ok();
        }
        else {
            error!("complete_packet error: Didn't find expected Pid");
        }
        debug!("Exit complete_packet");
    }

    fn complete_subscription(&mut self, suback: &mqttrs::Suback) {
        debug!("Enter complete_subscription");
        let pid = suback.pid.get();

        let msg_opt = self.packets.remove(&pid);

        if let Some(msg) = msg_opt {
            // The data that has been encoded by this module is being decoded now so unwrap
            // is as safe as it can be.
            let result = mqttrs::decode_slice(&msg.buffer).unwrap().unwrap();

            if let Packet::Subscribe(subs_packet) = result {
                let meta = subs_packet.topics;
                assert!(meta.len() == suback.return_codes.len());

                let mut subscription_results = Vec::<Result<(Receiver<Vec<u8>>, Qos), MqttError>>::with_capacity(meta.len());

                for (subs, result) in meta.iter().zip(suback.return_codes.iter()) {
                    subscription_results.push(
                        if let mqttrs::SubscribeReturnCodes::Success(result_qos) = result {
                            // TODO: Check it - not sure if this matters

                            if subs.qos != *result_qos {
                                warn!("Different QoS allocated");
                            }

                            let result_qos = match result_qos {
                                mqttrs::QoS::AtMostOnce => Qos::AtMostOnce,
                                mqttrs::QoS::AtLeastOnce => Qos::AtLeastOnce,
                                mqttrs::QoS::ExactlyOnce => Qos::ExactlyOnce,
                            };

                            let (s, r) = unbounded();

                            self.subscriptions.insert(subs.topic_path.to_owned(), s);
                            Ok((r, result_qos))
                        }
                        else {
                            Err(MqttError::SubscribeError)
                        }
                    )
                }
                msg.req_future.complete(Ok(PacketFutureResult::Subscriptions(subscription_results)));
            }
            else {
                // This should never be possible to happen but implemented to keep Rust happy
                // there is probably a more clever way to do this.
                panic!("Non-subscribe meta-data found. This is not possible");
            }
        }
        else {
            error!("complete_subscription error: Didn't find expected Pid");
        }
    }

    async fn send_ping(&mut self) -> Result<(), MqttError> {
        let mut buf: [u8; 8] = [0; 8];

        let pkt = mqttrs::Packet::Pingreq;
        let length = mqttrs::encode_slice(&pkt, &mut buf)?;

        self.tx_stream.write_all(&buf[..length]).await?;
        Ok(())
    }

    async fn write_packet(&mut self, packet: ChannelPacket) -> Result<(), MqttError> {
        let msg = match packet {
            ChannelPacket::Generic(msg) => msg,
            ChannelPacket::Connect(msg, keep_alive) => {
                debug!("Keep-alive set to {}s", keep_alive);
                self.keep_alive = keep_alive;
                msg
            },
            ChannelPacket::RemovePacket(pid) => {
                self.packets.remove(&pid);
                return Ok(());
            }
        };

        let msg_buffer = msg.buffer.clone();

        // The packet needs to be in the HashMap prior sending packet out as
        // otherwise ack can happen and be processed prior it gets inserted.
        if msg.pid >= 0 {
            self.packets.insert(msg.pid as u16, msg);
        }
        self.tx_stream.write_all(&msg_buffer).await?;
        Ok(())
    }

    async fn process_recv(&mut self, packet: &mqttrs::Packet<'_>) -> Result<(), MqttError> {
        debug!("Process recv");
        // This helps to keep error handling outside the select macro
        self.ongoing_ping_reg = false;
        match packet {
            Packet::Connack(_c) => {
                warn!("Connect ack received");
                self.complete_packet(&0);
            },
            Packet::Pingresp => {
                info!("Ping received");
                self.ongoing_ping_reg = false;
            },
            Packet::Suback(suback) => {
                self.complete_subscription(suback);
            },
            Packet::Puback(pid) | mqttrs::Packet::Unsuback(pid)=> self.complete_packet(&pid.get()),
            Packet::Publish(publish) => {
                info!("Received packet");
                let qos_pid = publish.qospid;

                if qos_pid != mqttrs::QosPid::AtMostOnce {
                    if let mqttrs::QosPid::AtLeastOnce(pid) = qos_pid {
                        // Only support AtLeastOnce for now
                        let mut buf: [u8; 8] = [0; 8];

                        let pkt = Packet::Puback(pid);
                        let length = mqttrs::encode_slice(&pkt, &mut buf)?;
                        self.tx_stream.write_all(&buf[..length]).await?;
                    }
                    else {
                        panic!("Not supporting ExactlyOnce yet");
                    }
                }

                let topic = String::from(publish.topic_name);
                let topic_channel = self.subscriptions.get(&topic);

                if let Some(channel) = topic_channel {
                    if let Err(err) = channel.send(publish.payload.to_vec()).await {
                        warn!("Error message {:?}. Client has closed the handle for topic: {:?}", err, publish.topic_name);
                        self.subscriptions.remove(&topic);
                    }
                } else {
                    warn!("Didn't find topic {:?} from hashtable", publish.topic_name);
                }
            },
            _ => warn!("Other packet received"),
        }
        Ok(())
    }

    async fn manage_timeout(&mut self) -> Result<(), MqttError> {
        if self.ongoing_ping_reg == false {
            self.ongoing_ping_reg = true;
            self.send_ping().await?;
            return Ok(());
        }
        else {
            error!("Ping failed");
            return Err(MqttError::PingRespTimeout);
        }
    }

    fn packet_length(data: &[u8]) -> Result<usize, MqttError> {
        let mut length = 0;
        let mut adjusted = 2; // In minimum two bytes are not part of the length
        let mut byte = 0x80; // If this doesn't get changed, then we know we are in trouble

        let end = std::cmp::min(4, data.len()-1);

        for b in &data[1..end] {
            byte = *b;
            length |= (byte & 0x7f) as usize;
            if byte & 0x80 > 0 {
                // We are making room for upcoming byte
                length <<= 7;
                adjusted += 1;
            }
            else {
                break;
            }
        }

        // If the last available byte is marked to-be-continued, it is clear
        // sign we need more data.
        if byte & 0x80 > 0 {
            Err(MqttError::ReceptionIncomplete)
        }
        else {
            // We can only calculate fields after
            // header bytes and length field(s)
            Ok(length + adjusted)
        }
    }

    async fn process_packets<R: AsyncReadExt + Unpin>(&mut self, stream: R, msg_receiver: Receiver<ChannelPacket>) -> Result<(), MqttError> {
        debug!("fn process_packet");
        let mut stream = stream;

        // It is possible that the timeout is happening even when when packet sending has been performed
        // with QoS 0 -> no Acks. This is by design as it still makes sense to check if the gateway is alive.
        let mut buf: [u8; 4096] = [0; 4096];
        let mut start = 0;
        let mut stop = 0;

        loop {
            // Waiting for the event and acting on it must be separated due to borrow-checker as the operations
            // are mutable while the listing of the invent isn't.
            let status = (
                async {
                    let stream_result = stream.read(&mut buf[stop..]).await.map_err(|op| {
                        error!("Data receive error {:?}", op);
                        MqttError::from(op)
                    });
                    debug!("Data received");
                    match stream_result {
                        Ok(n) => {
                            if n > 0 {
                                ProcessLoop::DataReceived(n)
                            }
                            else {
                                ProcessLoop::Error(MqttError::Disconnected)
                            }
                        },
                        Err(err) => ProcessLoop::Error(err)
                    }
                }
            ).or(
                async {
                    let stream_result = msg_receiver.recv().await.map_err(|op| {
                        error!("Channel error {:?}", op);
                        MqttError::from(op)
                    });
                    debug!("Channel packet received");
                    match stream_result {
                        Ok(msg) => ProcessLoop::ChannelPacket(msg),
                        Err(err) => ProcessLoop::Error(err)
                    }
                }
            ).or(
                async {
                    Timer::after(Duration::from_secs(self.keep_alive as u64)).await;
                    debug!("Keep-alive timeout");
                    ProcessLoop::Timeout
                }
            ).await;

            match status {
                ProcessLoop::DataReceived(n) => {
                    stop += n;
                    loop {
                        if let Ok(single_packet_length) = Mqtt::<T>::packet_length(&buf[start..stop]) {
                            // Only for debugging
                            if single_packet_length != n {
                                warn!("Lengths differ n:{} packet_length {}", n, single_packet_length);
                            }

                            let result = mqttrs::decode_slice(&buf[start..stop])?;

                            if let Some(packet) = result {
                                start += single_packet_length;

                                //println!("Start: {} and end: {}", start, stop);

                                self.process_recv(&packet).await?;

                                if start == stop {
                                    start = 0;
                                    stop = 0;
                                }
                                else {
                                    continue;
                                }
                            }
                            else {
                                warn!("Received {:?} bytes data but didn't manage to decode full package", n);
                            }
                        }
                        else {
                            warn!("Received {:?} bytes that is too little data to decode the package", n);
                        }
                        break; // All error cases end up here
                    }
                },
                ProcessLoop::ChannelPacket(msg) =>  self.write_packet(msg).await?,
                ProcessLoop::Timeout => {
                    warn!("Keep-alive timeout");
                    if self.manage_timeout().await.is_err() == true {
                        return Err(MqttError::PingRespTimeout);
                    }
                },
                ProcessLoop::Error(error) => return Err(error),
            }
        }
    }
}

struct SelfSignedCertVerifier {
    certs: Vec<u8>,
}

impl SelfSignedCertVerifier {
    fn new(root_cert: &[u8]) -> Self {
        Self {
            certs: Vec::from(root_cert)
        }
    }

    fn check_signature(&self, cert: &Certificate) -> Result<HandshakeSignatureValid, Error> {
        let res = X509Certificate::from_der(&cert.0);
        match res {
            Ok((rem, xcert)) => {
                assert!(rem.is_empty());

                info!("X.509 Subject: {}", xcert.subject());
                info!("X.509 Issuer: {}", xcert.issuer());
                info!("X.509 serial: {}", xcert.tbs_certificate.raw_serial_as_string());
                //
                //assert_eq!(xcert.version(), X509Version::V3);

                for pem in Pem::iter_from_buffer(&self.certs) {
                    let pem = pem.expect("this certificate is fucked");
                    let x509 = pem.parse_x509().expect("X.509: decoding DER failed");
                    assert_eq!(x509.tbs_certificate.version, X509Version::V3);

                    let issuer_public_key = x509.public_key();
                    if xcert.verify_signature(Some(issuer_public_key)).is_ok() {
                        warn!("Signature validation worked");
                        return Ok(HandshakeSignatureValid::assertion())
                    }
                }
            },
            _ => {
                error!("Certification parsin failed");
                return Err(Error::InvalidCertificateEncoding)
            },
        }
        error!("verify_tls13_signature: error checking signature");
        Err(Error::InvalidCertificateSignature)
    }
}


impl rustls::client::ServerCertVerifier for SelfSignedCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &Certificate,
        _intermediates: &[Certificate],
        server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        now: SystemTime
    ) -> Result<ServerCertVerified, Error> {

        info!("Server-name {:?}", server_name);

        // TODO: Check that certificate matches DNS-name
        // TODO: Flag UnknownIssuer
        let res = X509Certificate::from_der(end_entity.0.as_slice());
        match res {
            Ok((rem, x509)) => {
                assert!(rem.is_empty());
                //
                info!("-----------Entity certificate---------------");
                info!("X.509 Subject: {}", x509.subject());
                info!("X.509 Version: {}", x509.version());
                info!("X.509 Issuer: {}", x509.issuer());
                info!("X.509 serial: {}", x509.tbs_certificate.raw_serial_as_string());

                let subject = x509.subject();

                let matches_found: Vec::<_> = subject
                    .iter_common_name()
                    .filter(|name| {
                        if let ServerName::DnsName(server) = server_name {
                            if let Ok(cert_server_str) = name.as_str() {
                                return cert_server_str == server.as_ref()
                            }
                        }
                        false
                    })
                    .collect();

                if matches_found.is_empty() {
                    error!("No DN match found");
                    return Err(Error::InvalidCertificateData(String::from("No DN match found")));
                }

                // TODO: SAN validation should be added

                // Starting with timing topic
                let asn_time = ASN1Time::from_timestamp(
                    now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().try_into().unwrap()
                );

                if !x509.validity().is_valid_at(asn_time) {
                    error!("Outside of validity period");
                    return Err(Error::InvalidCertificateData(String::from("Invalid time information")));
                }

                if x509.version() != X509Version::V3 {
                    warn!("Certificate version is not V3 but {}", x509.version());
                }

                for pem in Pem::iter_from_buffer(&self.certs) {
                    let pem = pem.expect("this certificate is broken");
                    let xcert = pem.parse_x509().expect("X.509: decoding DER failed");

                    info!("---------Stored-------------");
                    info!("X.509 Subject: {}", xcert.subject());
                    info!("X.509 Version: {}", xcert.version());
                    info!("X.509 Issuer: {}", xcert.issuer());
                    info!("X.509 serial: {}", xcert.tbs_certificate.raw_serial_as_string());

                    if xcert.issuer() == x509.issuer() {
                        return Ok(rustls::client::ServerCertVerified::assertion());
                    }
                }
                return Err(Error::InvalidCertificateData(String::from("No valid DN found for issuer")));
            },
            _ => error!("x509 parsing failed: {:?}", res),
        }

        return Err(Error::InvalidCertificateEncoding);
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        cert: &Certificate,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        debug!("verify_tls12_signature");
        return self.check_signature(cert);
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        cert: &Certificate,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        debug!("verify_tls13_signature");
        return self.check_signature(cert);
    }
}

/// Builder MQTT Client
///
/// Builder for setting various MQTT connection parameters creating the actual client for the connection.
/// If the client connection to the broker is broken, the application needs to create new client through this mechanism.
///
/// # Example
/// ```
/// let mut mqtt_init = MqttInit {
///     address: address.to_string(),
///     ..Default::default()
/// };
///
/// let mut amqtt = if let Some(c) = certificate {
///     mqtt_init.port = 8883;
///     mqtt_init.with_tls(c).await?
/// }
/// else {
///     mqtt_init.init().await?
/// };
///
/// Mqtt.connect(30, client_id, None, None, true).await?;
/// ```
#[non_exhaustive]
pub struct MqttInit {
    /// MQTT broker address
    pub address: String,
    /// MQTT port address
    pub port: u16,
    //#[doc(hidden)]
    //pub __non_exhaustive: ()
}

impl Default for MqttInit {
    fn default() -> Self {
        Self {
            address: String::new(),
            port: MQTT_DEFAULT,
        }
    }
}

impl MqttInit {
    /// Initializes MQTT for connection
    pub async fn init(self) -> Result<Client, MqttError> {
        debug!("init");
        let stream = net::TcpStream::connect(format!("{}:{}", self.address, self.port)).await?;

        Ok(Client::init_common(stream))
    }


    /// Sets MQTT to use TLS
    pub async fn with_tls(self, root_cert: &[u8]) -> Result<Client, MqttError> {
        info!("init_tls");

        let verifier = SelfSignedCertVerifier::new(root_cert);

        let client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth();

        warn!("Cert inserted");

        let addr = (self.address.clone(), self.port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;

        let domain = ServerName::try_from(self.address.as_str()).unwrap();

        //let stream = net::TcpStream::connect(format!("{}:{}", addr, self.port)).await?;
        let stream = net::TcpStream::connect(&addr).await?;

        let stream = TlsConnector::from(Arc::new(client_config)).connect(domain, stream).await?;

        debug!("TLS stream crated");
        Ok(Client::init_common(stream))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::Read,
        path::Path,
        env
    };
    use env_logger;
    use smol::block_on;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn test_connection(address: &str, client_id: &str, cert: Option<&[u8]>) {
        block_on(
            async {
                let mut mqtt_init = MqttInit {
                    address: address.to_string(),
                    ..Default::default()
                };

                let mut amqtt = if let Some(c) = cert {
                    mqtt_init.port = 8883;
                    //mqtt_init.set_certificate(c).expect("Setting certificate failed");
                    mqtt_init.with_tls(c).await.unwrap()
                }
                else {
                    mqtt_init.init().await.unwrap()
                };

                debug!("Init done");
                amqtt.connect(30, client_id, None, None, true).await.expect("Connect failed");

                debug!("Connect done");

                let test_subscription = vec![("test/topic".to_string(), Qos::AtLeastOnce),("test/topic1".to_string(), Qos::AtMostOnce)];

                let subs_result = amqtt.subscribe(&test_subscription).await;

                if let Ok(subs) = &subs_result {
                    for (i, individual_sub_result) in subs.iter().enumerate() {
                        match individual_sub_result {
                            Ok((_, qos)) => info!("Subscription for {} succeeded with Qos {:?}", test_subscription[i].0, qos),
                            Err(err) => panic!("MQTT error {:?}", err)
                        }
                    }
                }

                let subs = subs_result.unwrap();

                smol::spawn(async move {
                    info!("Entering receive loop");
                    let (channel, _) = subs[0].as_ref().unwrap();
                    loop {
                        let rmessage = channel.recv().await;
                        match rmessage {
                            Ok(message) => info!("Message received:{:?}", String::from_utf8_lossy(&*message) ),
                            Err(err) => {
                                warn!("test: Receive loop ended with code {:?}", err);
                                break;
                            }
                        }
                    }
                }).detach();

                let data = vec![1,2,3,4,5,6,7,8,9,10];

                amqtt.publish("test/topic", Qos::AtMostOnce, &data).await.expect("Qos zero publish failed");
                debug!("First publish done");

                info!("Test:Loop up");

                amqtt.publish("test/topic", Qos::AtLeastOnce, &data).await.expect("Qos one publish failed");

                amqtt.disconnect().await.expect("Disconnect failed");
            }
        );
    }

    #[test]
    fn test_plain_connection() {
        init();

        test_connection("test.mosquitto.org", "amqtt_test1", None);
        //test_connection("127.0.0.1", "amqtt_test1", None);
    }

    #[test]
    fn test_secure_connection() {
        init();

        let root_path = env::var("MOSQUITTO_CERT").expect("Couldn't find the environment variable MOSQUITTO_CERT");
        let root_path = Path::new(&root_path);

        let mut f = File::open(root_path).expect("Couldn't find the certificate from defined path");

        let mut ca_cert = Vec::new();
        // read the whole file
        f.read_to_end(&mut ca_cert).unwrap();

        /*
        let s = match std::str::from_utf8(&ca_cert) {
            Ok(v) => warn!("{}", v),
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
          };*/

        test_connection("test.mosquitto.org", "amqtt_test2", Some(&ca_cert[..]));
    }

}