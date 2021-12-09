/// This is application reading sensor information and then passing it to the cloud
use std::{
    env,
    path::Path,
    time::{
        Duration,
        SystemTime
    },
    str::from_utf8,
};

use env_logger;
use log::{info, error};
use anyhow::Context;

use jsonwebtoken::Algorithm;

use smol::{
    Timer,
    fs,
    future::FutureExt
};

use protobuf::{
    Clear,
    Message
};

use amqtt::{
    mqtt::Qos,
    gcp_mqtt::{
        GcpMqtt,
        //GcpIoTError,
        ChannelData
    }
};

use messages::{
    EnvironmentDataBlocks,
    EnvironmentData,
    //Configuration
};

mod sensor;
mod messages;


async fn start_cloud() -> GcpMqtt {
    let path        = env::var("CERT").expect("No private cert environment variable");
    let project_id  = env::var("PROJECT_ID").unwrap();
    let cloud_region = env::var("REGION").unwrap();
    let registry_id  = env::var("REGISTRY_ID").unwrap();
    let device_id   = env::var("DEVICE_ID").unwrap();
    let root_path   = env::var("GOOGLE_ROOT_CERT").expect("Google root cert not found");

    let path = Path::new(&path);

    let user_cert = fs::read(path).await.unwrap();

    let root_path = Path::new(&root_path);

    let ca_cert = fs::read(root_path).await.unwrap();

    GcpMqtt::new(
        Algorithm::ES256,
        Duration::from_secs(1200),
        project_id,
        cloud_region,
        registry_id,
        device_id,
        30_u16,
        user_cert,
        &ca_cert
    ).await.with_context(|| "Failed to create GCP MQTT").unwrap()
}

async fn sample(sensor: &mut Box<dyn sensor::Sensor>) -> EnvironmentData {
    let sample = sensor.sample().unwrap();
    info!("Measurement: {}", sample);

    let mut measurement = EnvironmentData::new();

    //measurement.time = u32::try_from(Local::now().naive_local().timestamp()).unwrap();

    // California time. Don't get about the universal time as the temperature has local context
    // Seemed to be major hassle to get timestamp adjusted to the local timezone.
    measurement.time = (SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() - 8*60*60) as u32;
    measurement.temperature_k = (sample.temperature + 27215) as u32;
    measurement.humidity_rh = sample.relative_humidity as u32;
    measurement.voc_index = (sample.voc / 2) as u32;
    measurement
}


fn main() {
    env_logger::init();
    let n_samples_per_packet = env::var("N_SAMPLES").and_then(|x| Ok(x.parse::<usize>().unwrap())).unwrap_or(10);
    let sample_interval = env::var("SAMPLE_INTERVAL").and_then(|x| Ok(x.parse::<usize>().unwrap())).unwrap_or(3);

    info!("Starting IoT node with {} samples per packet and {} sample interval", n_samples_per_packet, sample_interval);

    let mut sensor = sensor::new("/dev/i2c-1").unwrap();

    sensor.initialize().unwrap();

    smol::block_on(async {
        info!("Started async block");
        let mut gcp_mqtt: GcpMqtt = start_cloud().await;

        gcp_mqtt.set_device_state(&"up".to_owned()).await.unwrap();

        let mut measurements = EnvironmentDataBlocks::new();
        info!("About to start loop");
        loop {
            for _ in 1_usize..n_samples_per_packet {
                let packet_result = gcp_mqtt.wait_channels().or(async {
                    Timer::after(Duration::from_secs(sample_interval as u64)).await;
                    Ok(ChannelData::Timeout)
                }).await;

                // We don't care right now about the fact that receiving commands or configurations
                // messes the exact interval. We always wait 30 seconds no matter what as both
                // commands and receiving configurations are going to be rare events
                match packet_result {
                    Ok(ChannelData::Config(packet)) => {
                        // This is text of base64 so it is save to cast this to string
                        info!("Config packet {:?}", from_utf8(&packet).unwrap());
                    },
                    Ok(ChannelData::Command(packet)) => {
                        info!("Command packet {:?}", packet);
                    },
                    Ok(ChannelData::Timeout) => {
                        let sample = sample(&mut sensor).await;
                        measurements.blocks.push(sample);
                    },
                    Err(_) => {
                        error!("Unspecified loop termination");
                        break;
                    }
                }
            }
            let data = measurements.write_to_bytes().unwrap();

            gcp_mqtt.publish("data", Qos::AtLeastOnce, &data).await.unwrap();

            measurements.clear();
        }
    })

}
