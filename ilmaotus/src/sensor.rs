/// Trait for Sensirion sensor

use std::{
    path::{
        Path,
        PathBuf
    },
    fmt,
    fs,
    convert::{
        From,
        TryFrom
    },
    io::{
        Error,
    },
    time::{
        Duration,
        SystemTime
    },
    thread,
    sync::{
        Arc,
        Mutex
    }
};

use thiserror::Error;
use log::{info, error, warn, debug};

use linux_embedded_hal as hal;

use hal::{Delay, I2cdev};
//use i2cdev::linux::LinuxI2CError;

use shtcx::{
    PowerMode,
    shtc3,
};

use sgp40::Sgp40;

use svm40::{
    Svm40,
    Signals
};

const BASELINE_UPDATE_INTERVAL: u64 = 2*60*60;
const BASELINE_FILENAME: &'static str = "baseline.txt";

/// Sensor type for the measurements. The two different types are
/// needed as one sensor measured VOC index while the other one
/// measures TVOC pppm. Hopefully, someday the sensors can be
/// calibrated to be equal.
pub enum SensorType {
    Sgp40,
    Svm40
}

#[derive(Error, Debug)]
pub enum IlmaotusError {
    #[error("Gas sensor operation failed: {0}")]
    GasSensorError(String),

    #[error("Temperature sensor operation failed: {0}")]
    TemperatureSensorError(String),

    #[error("Creating I2C failed")]
    Ic2Error(#[from] linux_embedded_hal::i2cdev::linux::LinuxI2CError),

    //#[error("Unsupported feature set")]
    //UnsupportedFeatureset,

    #[error("No sensor found")]
    NoSensorFound,

    #[error("File operation failed")]
    FileError(#[from] Error)
}

#[derive(Debug, Clone, Copy)]
pub struct Measurement {
    /// Total Volative Orgamoc Compound value
    pub voc: u16,

    /// Relative humidity in 100x
    pub relative_humidity: u16,

    /// Temperature in 100x
    pub temperature: i32
}

impl From<Signals> for Measurement {
    fn from(signals: Signals) -> Self {
        Measurement {
            voc: signals.voc_index as u16 / 10,
            relative_humidity: signals.relative_humidity as u16,
            temperature: (signals.temperature / 2) as i32
        }
    }
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VOC index: {} RH%: {:.2} Temperature: {:.2}",
            self.voc / 2,
            (self.relative_humidity as f32) / 100.0,
            (self.temperature as f32) / 100.0)
    }
}

pub trait Sensor {
    /// Initializes the sensor for operation. May block for significant period of time
    fn initialize(&mut self) -> Result<(), IlmaotusError>;

    /// Pull sample from sensor
    fn sample(&mut self) ->  Result<Measurement, IlmaotusError>;

    /// Sensor type
    fn sensor_type(&self) -> SensorType;
}



fn gas_err_mapper<T>(message: T) -> IlmaotusError
where T: fmt::Debug
{
    IlmaotusError::GasSensorError(format!("{:?}", message))
}

fn tmp_err_mapper<T>(message: T) -> IlmaotusError
where T: fmt::Debug
{
    IlmaotusError::TemperatureSensorError(format!("{:?}", message))
}


/// Sgpc3 gas sensor with SHT3 humidity and temperature sensor
struct Sgp40Sensor {
    path: PathBuf,
    measurement: Arc<Mutex<Measurement>>,
}

impl Sgp40Sensor
{
    fn new<P: AsRef<Path> + ?Sized>(path: &P) -> Result<Self, IlmaotusError> {
        let mut path_buf = PathBuf::new();
        path_buf.push(path);
        Ok(Sgp40Sensor {
            path: path_buf,
            measurement: Arc::new(
                Mutex::new(
                    Measurement {
                        voc: 0,
                        relative_humidity: 0,
                        temperature: 0
                    }
                )
            )
        })
    }
}

impl Sensor for Sgp40Sensor {
    fn initialize(&mut self) -> Result<(), IlmaotusError> {
        let dev = I2cdev::new(self.path.to_owned())?;
        let dev_voc = I2cdev::new(self.path.to_owned())?;

        let mut rht_sensor = shtc3(dev);

        // This is just to test that the connection to T/RH sensor works
        rht_sensor.device_identifier().map_err(tmp_err_mapper)?;

        let measurement = self.measurement.clone();

        thread::spawn(move || {
            let mut gas_sensor = Sgp40::new(dev_voc, 0x59, Delay);
            let mut prior_time = SystemTime::now();
            loop {
                let combined = rht_sensor.measure(PowerMode::NormalMode, &mut Delay).map_err(tmp_err_mapper).unwrap();
                let (temperature, relative_humidity) =
                    (combined.temperature.as_millidegrees_celsius(), combined.humidity.as_millipercent());

                debug!("Combined: {} °C / {} %RH", temperature, relative_humidity);

                let relative_humidity = u16::try_from(relative_humidity).unwrap(); // This is safe as the value is always between 0-100

                let voc = gas_sensor.measure_voc_index_with_rht(
                    relative_humidity,
                    i16::try_from(temperature).unwrap_or(i16::MAX)
                ).map_err(gas_err_mapper).unwrap();

                let mut measurement_lock = measurement.lock().unwrap();

                *measurement_lock = Measurement {
                    voc,
                    relative_humidity:relative_humidity / 10,
                    temperature : (temperature / 10)
                };
                 // This is safe as time is taken earlier
                let processing_overhead = SystemTime::now().duration_since(prior_time).unwrap();

                // Want to aim as close to one sec sleep as possible. The time varies per VOC algorithm
                // and thus this is calculated each time.
                let one_sec = Duration::from_secs(1) - processing_overhead;

                thread::sleep(one_sec);
                prior_time = SystemTime::now();
            }
        });

        Ok(())
    }

    fn sample(&mut self) -> Result<Measurement, IlmaotusError> {
        let measurement_lock = self.measurement.lock().unwrap();
        Ok(*measurement_lock)
    }

    fn sensor_type(&self) -> SensorType {
        SensorType::Sgp40
    }
}

pub struct Svm40Sensor {
    sensor: Svm40<I2cdev,Delay>,
    baseline_refresh: SystemTime,
}

impl Svm40Sensor {
    fn new<P: AsRef<Path> + ?Sized>(path: &P) -> Result<Self, IlmaotusError> {
        let dev = I2cdev::new(path)?;
        let mut sensor = Svm40::new(dev, 0x6a, Delay);

        // Tests that there is sensor behind this address
        sensor.version().map_err(gas_err_mapper)?;
        Ok(Svm40Sensor {
            sensor,
            baseline_refresh: SystemTime::UNIX_EPOCH
        })
    }
}

impl Sensor for Svm40Sensor {
    fn initialize(&mut self) -> Result<(), IlmaotusError> {
        let time = SystemTime::now();
        // Loading baseline if we have any.

        if let Ok(baseline) = fs::read_to_string(BASELINE_FILENAME) {
            let baseline = u64::from_str_radix(&baseline, 16).expect("Baseline conversion failed");

            info!("Found baseline {}", baseline);
            self.sensor.set_voc_states(baseline).map_err(gas_err_mapper)?;
        }
        else {
            warn!("No baseline found");
        }

        self.baseline_refresh = time + Duration::from_secs(BASELINE_UPDATE_INTERVAL);

        // Starting the sensor microcontroller
        self.sensor.start_measurement().map_err(gas_err_mapper)?;

            // Weird hack needed to overcome ? in async block
        Ok(())
    }

    fn sample(&mut self) -> Result<Measurement, IlmaotusError> {
        let measurement = self.sensor.get_measurements().map_err(gas_err_mapper)?;

        let time = SystemTime::now();

        if time > self.baseline_refresh {
            let baseline = self.sensor.get_voc_states().map_err(gas_err_mapper)?;

            let baseline = format!("{:x}", baseline);

            fs::write(BASELINE_FILENAME, baseline.as_bytes())?;

            self.baseline_refresh = time + Duration::from_secs(BASELINE_UPDATE_INTERVAL);

            info!("Baseline {} saved", baseline);
        }

        Ok(Measurement::from(measurement))
    }

    fn sensor_type(&self) -> SensorType {
        SensorType::Svm40
    }
}

pub fn new<P: AsRef<Path> + ?Sized>(path: &P) -> Result<Box<dyn Sensor>, IlmaotusError> {
    if let Ok(svm40) = Svm40Sensor::new(path) {
        info!("Found SVM40");
        Ok(Box::new(svm40))
    }
    else if let Ok(sgp40) = Sgp40Sensor::new(path) {
        info!("Found SGC40");
        Ok(Box::new(sgp40))
    }
    else {
        error!("No functioning sensor found");
        Err(IlmaotusError::NoSensorFound)
    }
}