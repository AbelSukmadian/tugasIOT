mod ota;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{Level, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::mqtt::client::{EspMqttClient, EventPayload, MqttClientConfiguration, QoS};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use log::{error, info, warn};
use serde::Deserialize;
use std::sync::mpsc;
use std::thread;

const WIFI_SSID: &str = "pova_zero";
const WIFI_PASS: &str = "12345678";
const MQTT_URL: &str = "mqtt://iotrust.duckdns.org:1883";
const MQTT_USER: &str = "esp32_device";
const MQTT_PASS: &str = "Sukmadian1525";
const DEVICE_ID: &str = "ESP32-S3-01";
const FW_VERSION: &str = "0.10.0"; // naikan versi untuk test OTA

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
    info!("=== Firmware v{} (LED Blink Dummy) mulai ===", FW_VERSION);

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // ---- LED Internal (GPIO48) ----
    let mut led = PinDriver::output(peripherals.pins.gpio48)?;
    info!("LED internal siap di GPIO48");

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
    let ota_topic = format!("esp32/{}/ota/command", DEVICE_ID);
    let status_topic = format!("esp32/{}/status", DEVICE_ID);
    let ota_topic_thread = ota_topic.clone();
    let (tx, rx) = mpsc::channel::<Msg>();

    let conf = MqttClientConfiguration {
        client_id: Some(DEVICE_ID),
        username: Some(MQTT_USER),
        password: Some(MQTT_PASS),
        ..Default::default()
    };
    let (mut client, mut conn) = EspMqttClient::new(MQTT_URL, &conf)?;

    // Thread MQTT event reader
    thread::Builder::new().stack_size(12288).spawn(move || {
        while let Ok(event) = conn.next() {
            match event.payload() {
                EventPayload::Connected(_) => {
                    let _ = tx.send(Msg::Connected);
                }
                EventPayload::Received { topic, data, .. } => {
                    let raw_msg = std::str::from_utf8(data).unwrap_or("invalid utf8");
                    log::info!("Pesan MQTT Masuk! Topic: {:?} | Data: {}", topic, raw_msg);

                    match serde_json::from_slice::<OtaCommand>(data) {
                        Ok(cmd) => {
                            let _ = tx.send(Msg::Ota(cmd));
                        }
                        Err(e) => {
                            log::error!("Gagal parsing JSON OTA: {:?}", e);
                        }
                    }
                }
                EventPayload::Disconnected => warn!("MQTT terputus"),
                _ => {}
            }
        }
    })?;

    // ---- Loop utama: LED Blink + OTA listener ----
    let mut led_state = false;
    let mut tick: u32 = 0;

    loop {
        // 1. Proses pesan MQTT
        while let Ok(msg) = rx.try_recv() {
            match msg {
                Msg::Connected => {
                    info!("MQTT terhubung, subscribe {}", ota_topic);
                    if let Err(e) = client.subscribe(&ota_topic, QoS::AtLeastOnce) {
                        warn!("Subscribe gagal: {:?}", e);
                    }
                    let payload = format!(
                        r#"{{"device_id":"{}","fw":"{}","status":"online"}}"#,
                        DEVICE_ID, FW_VERSION
                    );
                    let _ = client.publish(
                        &status_topic,
                        QoS::AtLeastOnce,
                        false,
                        payload.as_bytes(),
                    );
                }
                Msg::Ota(cmd) => {
                    if cmd.version == FW_VERSION {
                        info!("OTA diabaikan: versi {} sudah berjalan", cmd.version);
                    } else {
                        info!("OTA diterima: v{} dari {}", cmd.version, cmd.url);
                        // Jalankan OTA di dedicated thread dengan stack besar
                        // untuk menghindari stack overflow (HTTP + buffer + OTA state)
                        let url = cmd.url.clone();
                        if let Err(e) = thread::Builder::new()
                            .stack_size(65536) // 64KB: cukup untuk HTTP client + OTA buffers
                            .spawn(move || {
                                if let Err(e) = ota::perform_ota_update(&url) {
                                    error!("OTA gagal: {}", e);
                                }
                            })
                        {
                            error!("Gagal spawn OTA thread: {}", e);
                        }
                    }
                }
            }
        }

        // 2. Toggle LED setiap 500ms
        led_state = !led_state;
        led.set_level(if led_state { Level::High } else { Level::Low })?;
        info!(
            "[tick {}] LED {} | FW v{}",
            tick,
            if led_state { "ON " } else { "OFF" },
            FW_VERSION
        );
        tick += 1;

        FreeRtos::delay_ms(1000);
    }
}
