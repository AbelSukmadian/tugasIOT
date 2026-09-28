# 🌡️ ESP32-S3 IoT Firmware

Firmware berbasis **Rust** untuk **ESP32-S3** yang membaca sensor DHT22 (suhu & kelembapan), mengirimkan data telemetri ke broker MQTT, dan mendukung **OTA (Over-the-Air) update** melalui perintah MQTT.

---

## 📋 Overview

Project ini adalah firmware IoT untuk ESP32-S3 yang dibangun menggunakan bahasa **Rust** dengan framework [esp-idf-svc](https://github.com/esp-rs/esp-idf-svc). Firmware ini dirancang untuk:

- Membaca data suhu dan kelembapan dari sensor **DHT22**
- Menghubungkan perangkat ke jaringan **Wi-Fi**
- Mengirim data **telemetri** secara berkala ke broker **MQTT**
- Menerima dan mengeksekusi perintah **OTA update** melalui MQTT untuk memperbarui firmware secara nirkabel

---

## ✨ Fitur Utama

| Fitur | Deskripsi |
|-------|-----------|
| 📡 **Wi-Fi** | Terhubung ke jaringan Wi-Fi dengan autentikasi WPA2 |
| 🌡️ **Sensor DHT22** | Membaca suhu (°C) dan kelembapan relatif (%) setiap 5 detik |
| 📤 **MQTT Telemetry** | Mempublikasikan data JSON ke topik `esp32/{DEVICE_ID}/telemetry` |
| 🔄 **OTA Update** | Menerima perintah OTA melalui topik `esp32/{DEVICE_ID}/ota/command` dan mengunduh firmware baru via HTTP |
| 🔒 **OTA Version Guard** | Mengabaikan perintah OTA jika versi sama dengan yang sedang berjalan |
| 🛡️ **Partition Table Custom** | Mendukung flash 16MB dengan skema partisi `factory + ota_0 + ota_1` (masing-masing 4MB) |

---

## 🏗️ Arsitektur & Cara Kerja

```
┌─────────────────────────────────────────────────┐
│                  ESP32-S3                        │
│                                                  │
│  ┌──────────┐    ┌────────────────────────────┐  │
│  │  DHT22   │───▶│       Main Loop            │  │
│  │ Sensor   │    │  - Baca suhu & kelembapan  │  │
│  └──────────┘    │  - Publish telemetry MQTT  │  │
│                  │  - Proses pesan dari thread │  │
│                  └──────────┬─────────────────┘  │
│                             │                    │
│                  ┌──────────▼─────────────────┐  │
│                  │   MQTT Thread (terpisah)    │  │
│                  │  - Terima event MQTT        │  │
│                  │  - Parse perintah OTA       │  │
│                  │  - Kirim ke main via mpsc   │  │
│                  └──────────┬─────────────────┘  │
│                             │                    │
│                  ┌──────────▼─────────────────┐  │
│                  │      OTA Module             │  │
│                  │  - Download firmware (HTTP) │  │
│                  │  - Flash ke partisi OTA     │  │
│                  │  - Reboot otomatis          │  │
│                  └─────────────────────────────┘  │
└─────────────────────────────────────────────────┘
          │                          ▲
          │ MQTT Publish             │ MQTT Subscribe
          ▼                          │
┌─────────────────────────────────────────────────┐
│              MQTT Broker                        │
│  Topik Publish : esp32/{ID}/telemetry           │
│  Topik Subscribe: esp32/{ID}/ota/command        │
└─────────────────────────────────────────────────┘
```

### Alur Program

1. **Inisialisasi** — Peripheral (GPIO, Wi-Fi, NVS) diinisialisasi
2. **Koneksi Wi-Fi** — Perangkat terhubung ke Wi-Fi secara blocking
3. **Koneksi MQTT** — Client MQTT dibuat; thread terpisah membaca semua event MQTT
4. **Subscribe OTA** — Setelah menerima event `Connected`, otomatis subscribe ke topik OTA
5. **Loop Utama (setiap 5 detik)**:
   - Proses pesan dari thread MQTT (subscribe atau eksekusi OTA)
   - Baca sensor DHT22
   - Publish payload JSON telemetri ke MQTT broker

---

## 📦 Format Payload MQTT

### Topik Telemetri: `esp32/{DEVICE_ID}/telemetry`

```json
{
  "device_id": "ESP32-S3-01",
  "confidence": 92.5,
  "inference_time_ms": 25,
  "ram_usage_kb": 140,
  "temperature": 30.3,
  "humidity": 71.0
}
```

### Topik Perintah OTA: `esp32/{DEVICE_ID}/ota/command`

```json
{
  "version": "0.4.0",
  "url": "http://your-server.com/firmware_v4.bin"
}
```

> **Catatan:** Jika `version` sama dengan versi yang sedang berjalan, perintah OTA akan diabaikan secara otomatis.

---

## 🗂️ Struktur File

```
tugasIOT/
├── src/
│   ├── main.rs          # Entry point, Wi-Fi, MQTT, loop utama
│   ├── ota.rs           # Modul OTA: download & flash firmware
│   └── api_client.rs    # HTTP client helper (telemetry & OTA check)
├── partitions.csv       # Tabel partisi custom (16MB flash)
├── sdkconfig.defaults   # Konfigurasi ESP-IDF (stack size, flash, dll)
├── Cargo.toml           # Dependensi Rust
├── build.rs             # Build script (embuild)
└── rust-toolchain.toml  # Toolchain Rust esp
```

---

## 🛠️ Persyaratan

### Hardware
- **Board**: ESP32-S3 (flash 16MB)
- **Sensor**: DHT22 terhubung ke **GPIO10** (dengan resistor pull-up 10kΩ)
- **Koneksi**: Kabel USB ke komputer untuk flashing

### Software
- [Rust](https://rustup.rs/) dengan toolchain `esp`
- [espup](https://github.com/esp-rs/espup) — instalasi toolchain Rust untuk ESP
- [espflash](https://github.com/esp-rs/espflash) — tool flashing firmware
- [cargo-espflash](https://github.com/esp-rs/espflash) — integrasi Cargo untuk flashing

---

## ⚙️ Instalasi & Setup

### 1. Install Toolchain Rust ESP

```bash
cargo install espup
espup install
```

Setelah instalasi, jalankan script environment yang dihasilkan:

```powershell
# Windows (PowerShell)
. $HOME\export-esp.ps1
```

### 2. Install espflash

```bash
cargo install espflash
cargo install cargo-espflash
```

### 3. Clone Repository

```bash
git clone https://github.com/AbelSukmadian/tugasIOT.git
cd tugasIOT
```

### 4. Konfigurasi Konstanta

Edit konstanta di `src/main.rs` sesuai environment kamu:

```rust
const WIFI_SSID: &str = "nama_wifi_kamu";
const WIFI_PASS: &str = "password_wifi_kamu";
const MQTT_URL:  &str = "mqtt://your-broker.com:1883";
const MQTT_USER: &str = "username_mqtt";
const MQTT_PASS: &str = "password_mqtt";
const DEVICE_ID: &str = "ESP32-S3-01";   // ID unik perangkat
const FW_VERSION: &str = "0.3.0";         // Versi firmware saat ini
```

---

## 🚀 Build & Flash

### Build + Flash + Monitor (semua sekaligus)

```bash
cargo run
```

### Build saja (tanpa flash)

```bash
cargo build
```

### Flash firmware yang sudah di-build

```bash
espflash flash target/xtensa-esp32s3-espidf/debug/iot
```

### Cek info board sebelum flashing

```bash
espflash board-info
```

---

## 🔄 OTA Update

OTA update dilakukan melalui MQTT. Kirim pesan berikut ke topik `esp32/{DEVICE_ID}/ota/command`:

```json
{
  "version": "0.4.0",
  "url": "http://your-server.com/firmware_v0.4.0.bin"
}
```

### Alur OTA:
1. Perangkat menerima perintah OTA via MQTT
2. Versi dibandingkan — jika sama dengan yang berjalan, perintah diabaikan
3. Firmware baru diunduh dari `url` melalui HTTP dalam chunk 4KB
4. Firmware di-flash ke partisi OTA aktif (`ota_0` atau `ota_1`)
5. Perangkat reboot otomatis setelah 3 detik dan boot dari firmware baru

---

## 💾 Skema Partisi Flash (16MB)

| Nama       | Tipe  | Ukuran | Keterangan                  |
|------------|-------|--------|-----------------------------|
| `nvs`      | data  | 16KB   | Non-volatile storage        |
| `otadata`  | data  | 8KB    | Metadata partisi OTA aktif  |
| `phy_init` | data  | 4KB    | Kalibrasi RF                |
| `factory`  | app   | 4MB    | Firmware bawaan (factory)   |
| `ota_0`    | app   | 4MB    | Slot OTA pertama            |
| `ota_1`    | app   | 4MB    | Slot OTA kedua              |

---

## 📚 Dependensi Utama

| Crate | Kegunaan |
|-------|----------|
| `esp-idf-svc` | Wi-Fi, MQTT, HTTP, OTA, GPIO |
| `dht-sensor` | Membaca sensor DHT22 |
| `serde` + `serde_json` | Serialisasi/deserialisasi JSON |
| `anyhow` | Error handling |
| `log` | Logging ke serial monitor |
| `embuild` | Build system ESP-IDF |

---

## 📝 Lisensi

Project ini dibuat sebagai **Tugas IoT** oleh [Abel Sukmadian](mailto:abelsukmadian15@gmail.com).
