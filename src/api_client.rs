use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::http::Method;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
pub struct Telemetry {
    pub device_id: String,
    pub confidence_score: f32,
    pub inference_time_ms: u32,
    pub ram_usage_kb: u32,
}

#[derive(Deserialize, Debug)]
pub struct OtaCheckResponse {
    pub update_available: bool,
    pub download_url: Option<String>,
    pub version: Option<String>,
}

pub fn post_telemetry(backend_url: &str, telemetry: &Telemetry) -> anyhow::Result<()> {
    let mut http_config = Configuration::default();
    http_config.timeout = Some(Duration::from_secs(10));
    http_config.crt_bundle_attach = Some(esp_idf_svc::sys::esp_crt_bundle_attach);

    let mut http_connection = EspHttpConnection::new(&http_config)?;
    
    let url = format!("{}/api/telemetry", backend_url);
    info!("Mencoba mengirim telemetri ke: {}", url);

    let payload = serde_json::to_string(telemetry)?;
    let content_length = payload.len().to_string();

    let headers = [
        ("Content-Type", "application/json"),
        ("Content-Length", &content_length),
    ];

    http_connection.initiate_request(Method::Post, &url, &headers)?;
    http_connection.write(payload.as_bytes())?;
    http_connection.initiate_response()?;

    let status = http_connection.status();
    if status == 200 || status == 201 {
        info!("Telemetri berhasil dikirim. Status: {}", status);
    } else {
        warn!("Gagal mengirim telemetri. Status: {}", status);
    }

    Ok(())
}

pub fn check_for_updates(backend_url: &str, current_version: &str) -> anyhow::Result<Option<String>> {
    let mut http_config = Configuration::default();
    http_config.timeout = Some(Duration::from_secs(10));
    http_config.crt_bundle_attach = Some(esp_idf_svc::sys::esp_crt_bundle_attach);

    let mut http_connection = EspHttpConnection::new(&http_config)?;
    
    let url = format!("{}/api/ota/check?current_version={}", backend_url, current_version);
    info!("Mengecek update OTA di: {}", url);

    http_connection.initiate_request(Method::Get, &url, &[])?;
    http_connection.initiate_response()?;

    let status = http_connection.status();
    if status == 200 {
        let mut buf = [0u8; 1024];
        let bytes_read = http_connection.read(&mut buf)?;
        
        if let Ok(json_str) = std::str::from_utf8(&buf[..bytes_read]) {
            if let Ok(response) = serde_json::from_str::<OtaCheckResponse>(json_str) {
                if response.update_available {
                    if let Some(download_url) = response.download_url {
                        info!("Update ditemukan! Versi: {:?}", response.version);
                        return Ok(Some(download_url));
                    }
                } else {
                    info!("Firmware sudah dalam versi terbaru.");
                }
            }
        }
    } else {
        warn!("Gagal mengecek OTA. Status: {}", status);
    }

    Ok(None)
}
