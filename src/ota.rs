use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use esp_idf_svc::ota::EspOta;
use log::{error, info};
use std::time::Duration;

pub fn perform_ota_update(download_url: &str) -> anyhow::Result<()> {
    info!("Starting OTA update from: {}", download_url);

    // 1. Initialize OTA
    let mut ota = EspOta::new()?;
    let mut ota_update = ota.initiate_update()?;

    info!("OTA update initiated. Setting up HTTP client...");

    // 2. Setup HTTP Client
    let mut http_config = Configuration::default();
    http_config.timeout = Some(Duration::from_secs(10));
    http_config.buffer_size_tx = Some(4096);
    http_config.buffer_size = Some(4096);
    http_config.crt_bundle_attach = Some(esp_idf_svc::sys::esp_crt_bundle_attach);

    let mut http_connection = EspHttpConnection::new(&http_config)?;

    // 3. Request Firmware
    http_connection.initiate_request(esp_idf_svc::http::Method::Get, download_url, &[])?;
    http_connection.initiate_response()?;

    let status = http_connection.status();
    info!("HTTP Response Status: {}", status);

    if status != 200 {
        error!("Failed to download firmware. HTTP Status: {}", status);
        return Err(anyhow::anyhow!("HTTP request failed with status: {}", status));
    }

    // 4. Read and Write chunks
    let mut buf = [0u8; 4096];
    let mut total_read = 0;

    loop {
        let bytes_read = http_connection.read(&mut buf)?;
        if bytes_read == 0 {
            break; // EOF
        }

        ota_update.write(&buf[..bytes_read])?;
        total_read += bytes_read;
        info!("Downloaded and written {} bytes...", total_read);
    }

    info!("Download complete. Total size: {} bytes", total_read);

    // 5. Complete OTA and set boot partition
    info!("Finalizing OTA...");
    ota_update.complete()?;

    info!("OTA Update successful! Rebooting in 3 seconds...");
    std::thread::sleep(Duration::from_secs(3));
    
    // Reboot the device
    unsafe {
        esp_idf_svc::sys::esp_restart();
    }
}
