extern crate env_logger;
#[macro_use]
extern crate serde_json;
extern crate webthing;
extern crate rand;

extern crate bme680;
extern crate embedded_hal;
extern crate linux_embedded_hal as hal;


use rand::Rng;
use std::sync::{Arc, RwLock, Weak};
use std::{thread, time};
use webthing::server::ActionGenerator;
use webthing::{
    Action, BaseProperty, BaseThing, Thing, ThingsType,
    WebThingServer,
};

use bme680::*;
use embedded_hal::blocking::i2c;
use hal::*;
use std::result;
use std::time::Duration;

struct Generator;

impl ActionGenerator for Generator {
    fn generate(
        &self,
        thing: Weak<RwLock<Box<Thing>>>,
        name: String,
        input: Option<&serde_json::Value>,
    ) -> Option<Box<Action>> {
        let input = match input {
            Some(v) => match v.as_object() {
                Some(o) => Some(o.clone()),
                None => None,
            },
            None => None,
        };

        let name: &str = &name;
        match name {
            _ => None,
        }
    }
}

/// BME680 Temperature Sensor
fn make_temp_sensor() -> Arc<RwLock<Box<Thing + 'static>>> {
    let mut thing = BaseThing::new(
        "Tempearure Sensor".to_owned(),
        Some(vec!["MultiLevelSensor".to_owned()]),
        Some("A web connected temperature sensor".to_owned()),
    );

    let level_description = json!({
        "@type": "LevelProperty",
        "title": "Temperature",
        "type": "number",
        "description": "The current temperature in %",
        "minimum": -40,
        "maximum": 85,
        "unit": "Celcius",
        "readOnly": true
    });
    let level_description = level_description.as_object().unwrap().clone();
    thing.add_property(Box::new(BaseProperty::new(
        "level".to_owned(),
        json!(0),
        None,
        Some(level_description),
    )));

    Arc::new(RwLock::new(Box::new(thing)))
}

/// BME680 Humidity Sensor
fn make_hum_sensor() -> Arc<RwLock<Box<Thing + 'static>>> {
    let mut thing = BaseThing::new(
        "My Humidity Sensor".to_owned(),
        Some(vec!["MultiLevelSensor".to_owned()]),
        Some("A web connected humidity sensor".to_owned()),
    );

    let level_description = json!({
        "@type": "LevelProperty",
        "title": "Humidity",
        "type": "number",
        "description": "The current humidity in %",
        "minimum": 0,
        "maximum": 100,
        "unit": "percent",
        "readOnly": true
    });
    let level_description = level_description.as_object().unwrap().clone();
    thing.add_property(Box::new(BaseProperty::new(
        "level".to_owned(),
        json!(0),
        None,
        Some(level_description),
    )));

    Arc::new(RwLock::new(Box::new(thing)))
}

fn main() {
    env_logger::init();

    let mut things: Vec<Arc<RwLock<Box<Thing + 'static>>>> = Vec::new();

    // Create a thing that represents a humidity sensor
    let sensor = make_hum_sensor();
    things.push(sensor.clone());

    let cloned = sensor.clone();
    thread::spawn(move || {
        let mut rng = rand::thread_rng();

        // Mimic an actual sensor updating its reading every couple seconds.
        loop {
            thread::sleep(time::Duration::from_millis(5 * 60 * 1000));

            // Initialize device
            let i2c = I2cdev::new("/dev/i2c-1").unwrap();
            let mut dev = Bme680::init(i2c, Delay {}, I2CAddress::Primary)?;
            let settings = SettingsBuilder::new()
                .with_humidity_oversampling(OversamplingSetting::OS2x)
                .with_pressure_oversampling(OversamplingSetting::OS4x)
                .with_temperature_oversampling(OversamplingSetting::OS8x)
                .with_temperature_filter(IIRFilterSize::Size3)
                .with_gas_measurement(Duration::from_millis(1500), 320, 25)
                .with_run_gas(true)
                .build();
            dev.set_sensor_settings(settings)?;

            // Read sensor data
            dev.set_sensor_mode(PowerMode::ForcedMode)?;
            let (data, _state) = dev.get_sensor_data()?;

            println!("Temperature {}°C", data.temperature_celsius());
            println!("Pressure {}hPa", data.pressure_hpa());
            println!("Humidity {}%", data.humidity_percent());
            println!("Gas Resistence {}Ω", data.gas_resistance_ohm());

            let t = cloned.clone();
            let new_value = 70.0
                * rng.gen_range::<f32, f32, f32>(0.0, 1.0)
                * (-0.5 + rng.gen_range::<f32, f32, f32>(0.0, 1.0));
            let new_value = json!(new_value.abs());

            println!("setting new humidity level: {}", new_value);

            {
                let mut t = t.write().unwrap();
                let prop = t.find_property("level".to_owned()).unwrap();
                let _ = prop.set_cached_value(new_value.clone());
            }

            t.write()
                .unwrap()
                .property_notify("level".to_owned(), new_value);
        }
    });

    // If adding more than one thing, use ThingsType::Multiple() with a name.
    // In the single thing case, the thing's name will be broadcast.
    let mut server = WebThingServer::new(
        ThingsType::Multiple(things, "TemperatureHumidityPressureAndVOC".to_owned()),
        Some(8888),
        None,
        None,
        Box::new(Generator),
        None,
    );
    server.create();
    server.start();
}