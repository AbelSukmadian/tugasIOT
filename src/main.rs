mod ota;

use dht_sensor::dht22;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::{Ets, FreeRtos};
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::mqtt::client::{EspMqttClient, EventPayload, MqttClientConfiguration, QoS};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::thread;

const WIFI_SSID: &str = "pova_zero";
const WIFI_PASS: &str = "12345678";
const MQTT_URL: &str = "mqtt://iotrust.duckdns.org:1883";
const MQTT_USER: &str = "esp32_device";
const MQTT_PASS: &str = "Sukmadian1525";
const DEVICE_ID: &str = "ESP32-S3-01";
const FW_VERSION: &str = "0.3.0";

#[derive(Serialize)]
struct Telemetry<'a> {
    device_id: &'a str,
    confidence: f32,        // dummy sampai TinyML terpasang
    inference_time_ms: i32, // dummy
    ram_usage_kb: i32,      // dummy
    temperature: f32,       // asli dari DHT22 (backend belum menyimpannya)
    humidity: f32,          // asli dari DHT22
}

#[derive(Deserialize)]
struct OtaCommand {
    version: String,
    url: String,
}

enum Msg {
    Connected,
    Ota(OtaCommand),
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("Firmware v{} mulai", FW_VERSION);

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // ---- DHT22 ----
    let mut dht_pin = PinDriver::input_output_od(peripherals.pins.gpio10, Pull::Up)?;
    dht_pin.set_high()?;

    // ---- Wi-Fi ----
    let mut wifi = Box::new(EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?);
    let mut blocking_wifi = BlockingWifi::wrap(wifi.as_mut(), sys_loop.clone())?;
    blocking_wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: WIFI_PASS.try_into().unwrap(),
        channel: None,
        ..Default::default()
    }))?;
    blocking_wifi.start()?;
    blocking_wifi.connect()?;
    blocking_wifi.wait_netif_up()?;
    info!("Wi-Fi terhubung");

    // ---- MQTT ----
    let telemetry_topic = format!("esp32/{}/telemetry", DEVICE_ID);
    let ota_topic = format!("esp32/{}/ota/command", DEVICE_ID);

    let conf = MqttClientConfiguration {
        client_id: Some(DEVICE_ID),
        username: Some(MQTT_USER),
        password: Some(MQTT_PASS),
        ..Default::default()
    };
    let (mut client, mut conn) = EspMqttClient::new(MQTT_URL, &conf)?;

    // Thread khusus membaca event MQTT, hasilnya dikirim ke loop utama lewat channel
    let (tx, rx) = mpsc::channel::<Msg>();
    thread::Builder::new().stack_size(6144).spawn(move || {
        while let Ok(event) = conn.next() {
            match event.payload() {
                EventPayload::Connected(_) => {
                    let _ = tx.send(Msg::Connected);
                }
                EventPayload::Received { data, .. } => {
                    if let Ok(cmd) = serde_json::from_slice::<OtaCommand>(data) {
                        let _ = tx.send(Msg::Ota(cmd));
                    }
                }
                EventPayload::Disconnected => warn!("MQTT terputus"),
                _ => {}
            }
        }
    })?;

    // ---- Loop utama ----
    loop {
        // 1. Proses pesan dari thread MQTT
        while let Ok(msg) = rx.try_recv() {
            match msg {
                Msg::Connected => {
                    info!("MQTT terhubung, subscribe {}", ota_topic);
                    if let Err(e) = client.subscribe(&ota_topic, QoS::AtLeastOnce) {
                        warn!("Subscribe gagal: {:?}", e);
                    }
                }
                Msg::Ota(cmd) => {
                    if cmd.version == FW_VERSION {
                        info!("Command OTA versi {} sama dengan yang berjalan, diabaikan", cmd.version);
                    } else {
                        info!("Command OTA: versi {} dari {}", cmd.version, cmd.url);
                        if let Err(e) = ota::perform_ota_update(&cmd.url) {
                            error!("OTA gagal: {}", e);
                        }
                    }
                }
            }
        }

        // 2. Baca DHT22
        let mut delay = Ets;
        let (temp, hum) = match dht22::blocking::read(&mut delay, &mut dht_pin) {
            Ok(r) => (r.temperature, r.relative_humidity),
            Err(e) => {
                warn!("DHT22 gagal: {:?}", e);
                FreeRtos::delay_ms(2000);
                continue;
            }
        };

        // 3. Publish telemetry
        let t = Telemetry {
            device_id: DEVICE_ID,
            confidence: 92.5,
            inference_time_ms: 25,
            ram_usage_kb: 140,
            temperature: temp,
            humidity: hum,
        };
        let payload = serde_json::to_vec(&t)?;
        match client.publish(&telemetry_topic, QoS::AtLeastOnce, false, &payload) {
            Ok(_) => info!("Terkirim: {:.1} C, {:.1} %", temp, hum),
            Err(e) => warn!("Publish gagal: {:?}", e),
        }

        FreeRtos::delay_ms(5000);
    }
}