# 🌡️ ESP32-S3 IoT Firmware

Firmware berbasis **Rust** untuk **ESP32-S3** yang membaca sensor DHT22 (suhu & kelembapan), mengirimkan data telemetri ke broker MQTT, mendukung **OTA (Over-the-Air) update** melalui perintah MQTT, dan menjalankan **inferensi model ML** via komponen C/C++.

---

## 📋 Overview

Project ini adalah firmware IoT untuk ESP32-S3 yang dibangun menggunakan bahasa **Rust** dengan framework [esp-idf-svc](https://github.com/esp-rs/esp-idf-svc). Firmware ini dirancang untuk:

- Membaca data suhu dan kelembapan dari sensor **DHT22**
- Menghubungkan perangkat ke jaringan **Wi-Fi**
- Mengirim data **telemetri** (suhu, kelembapan, confidence ML, dll.) secara berkala ke broker **MQTT**
- Menerima dan mengeksekusi perintah **OTA update** melalui MQTT untuk memperbarui firmware secara nirkabel
- Menjalankan **inferensi model klasifikasi** menggunakan komponen C++ (`model_api`) yang terintegrasi via FFI
- Mem-publish status perangkat (`device_id`, `fw`, `status`) saat pertama terhubung ke broker

> **Versi firmware saat ini:** `0.10.0`

---

## ✨ Fitur Utama

| Fitur | Deskripsi |
|-------|-----------|
| 📡 **Wi-Fi** | Terhubung ke jaringan Wi-Fi dengan autentikasi WPA2 |
| 🌡️ **Sensor DHT22** | Membaca suhu (°C) dan kelembapan relatif (%) |
| 📤 **MQTT Telemetry** | Mempublikasikan data JSON ke topik `esp32/{DEVICE_ID}/telemetry` |
| 📊 **Status Publish** | Publikasi status `online` + versi firmware saat pertama konek MQTT |
| 🧠 **Inferensi ML** | Klasifikasi fitur sensor via komponen C++ (`model_api`) menggunakan FFI |
| 🔄 **OTA Update** | Menerima perintah OTA via MQTT, mengunduh & mem-flash firmware baru via HTTP |
| 🔒 **OTA Version Guard** | Mengabaikan perintah OTA jika versi sama dengan yang sedang berjalan |
| 💡 **LED Blink** | LED internal (GPIO48) berkedip setiap 1 detik sebagai heartbeat visual |
| 🛡️ **Partisi Custom** | Flash 16MB dengan skema `factory + ota_0 + ota_1` (masing-masing 4MB) |

---

## 🏗️ Arsitektur & Cara Kerja

```
┌──────────────────────────────────────────────────────┐
│                     ESP32-S3                          │
│                                                       │
│  ┌──────────┐    ┌───────────────────────────────┐   │
│  │  DHT22   │───▶│         Main Loop             │   │
│  │ Sensor   │    │  - Baca suhu & kelembapan     │   │
│  └──────────┘    │  - Inferensi model (FFI C++)  │   │
│                  │  - Toggle LED heartbeat        │   │
│                  │  - Proses pesan dari thread    │   │
│                  └───────────────┬───────────────┘   │
│                                  │                   │
│                  ┌───────────────▼───────────────┐   │
│                  │    MQTT Thread (terpisah)      │   │
│                  │  - Terima event MQTT           │   │
│                  │  - Parse perintah OTA          │   │
│                  │  - Kirim ke main via mpsc      │   │
│                  └───────────────┬───────────────┘   │
│                                  │                   │
│                  ┌───────────────▼───────────────┐   │
│                  │        OTA Module              │   │
│                  │  - Download firmware (HTTP)    │   │
│                  │  - Flash ke partisi OTA        │   │
│                  │  - Reboot otomatis (3 detik)   │   │
│                  └───────────────────────────────┘   │
└──────────────────────────────────────────────────────┘
          │                           ▲
          │ MQTT Publish              │ MQTT Subscribe
          ▼                           │
┌──────────────────────────────────────────────────────┐
│                   MQTT Broker                         │
│  Topik Publish  : esp32/{ID}/telemetry               │
│                   esp32/{ID}/status                  │
│  Topik Subscribe: esp32/{ID}/ota/command             │
└──────────────────────────────────────────────────────┘
```

### Alur Program

1. **Inisialisasi** — Peripheral (GPIO, Wi-Fi, NVS) diinisialisasi; LED GPIO48 siap
2. **Koneksi Wi-Fi** — Perangkat terhubung ke Wi-Fi secara blocking
3. **Koneksi MQTT** — Client MQTT dibuat; thread terpisah (stack 12KB) membaca semua event MQTT
4. **Subscribe OTA + Publish Status** — Setelah event `Connected`, subscribe ke topik OTA dan publish status `online`
5. **Loop Utama (setiap 1 detik)**:
   - Proses pesan dari thread MQTT (subscribe atau eksekusi OTA)
   - Toggle LED heartbeat
   - Log tick + versi firmware ke serial monitor

---

## 📦 Format Payload MQTT

### Topik Status: `esp32/{DEVICE_ID}/status`

Dipublikasikan **otomatis** setiap kali perangkat terhubung ke broker MQTT.

```json
{
  "device_id": "ESP32-S3-01",
  "fw": "0.10.0",
  "status": "online"
}
```

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
  "version": "0.11.0",
  "url": "http://your-server.com/firmware_v11.bin"
}
```

> **Catatan:** Jika `version` sama dengan versi yang sedang berjalan, perintah OTA akan **diabaikan** secara otomatis.

---

## 🗂️ Struktur File

```
tugasIOT/
├── src/
│   ├── main.rs          # Entry point: Wi-Fi, MQTT, LED blink, loop utama
│   ├── ota.rs           # Modul OTA: download & flash firmware via HTTP
│   ├── model.rs         # FFI binding ke komponen C++ model_api
│   └── api_client.rs    # HTTP client helper
├── components/
│   └── model/
│       ├── CMakeLists.txt      # Build config komponen C++
│       ├── include/
│       │   └── model_api.h     # Header FFI model classify
│       └── model_api.cpp       # Implementasi inferensi model
├── firmware.bin         # Binary firmware terbaru (untuk OTA)
├── firmware_v3.bin      # Binary firmware v0.3.0
├── firmware_v4.bin      # Binary firmware v0.4.0
│   ... (v5 – v10)
├── partitions.csv       # Tabel partisi custom (16MB flash)
├── sdkconfig.defaults   # Konfigurasi ESP-IDF
├── Cargo.toml           # Dependensi Rust
├── build.rs             # Build script (embuild)
└── rust-toolchain.toml  # Toolchain Rust esp
```

---

## 🛠️ Persyaratan

### Hardware
- **Board**: ESP32-S3 (flash 16MB)
- **Sensor**: DHT22 terhubung ke **GPIO10** (dengan resistor pull-up 10kΩ)
- **LED**: Internal LED di **GPIO48** (built-in pada banyak board ESP32-S3)
- **Koneksi**: Kabel USB ke komputer untuk flashing awal

### Software
- [Rust](https://rustup.rs/) dengan toolchain `esp`
- [espup](https://github.com/esp-rs/espup) — instalasi toolchain Rust untuk ESP
- [espflash](https://github.com/esp-rs/espflash) — tool flashing firmware
- [cargo-espflash](https://github.com/esp-rs/espflash) — integrasi Cargo untuk flashing
- MQTT client (contoh: [MQTTX](https://mqttx.app/), `mosquitto_pub`, atau dashboard IoT)

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
const WIFI_SSID:  &str = "nama_wifi_kamu";
const WIFI_PASS:  &str = "password_wifi_kamu";
const MQTT_URL:   &str = "mqtt://your-broker.com:1883";
const MQTT_USER:  &str = "username_mqtt";
const MQTT_PASS:  &str = "password_mqtt";
const DEVICE_ID:  &str = "ESP32-S3-01";   // ID unik perangkat
const FW_VERSION: &str = "0.10.0";        // Versi firmware saat ini
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

### Monitor serial output

```bash
espflash monitor
```

### Cek info board sebelum flashing

```bash
espflash board-info
```

---

## 🔄 OTA Update — Panduan Lengkap

OTA update memungkinkan kamu memperbarui firmware ESP32-S3 **tanpa kabel** langsung dari broker MQTT.

### Prasyarat OTA

1. Firmware awal sudah di-flash ke perangkat (via USB)
2. Perangkat terhubung ke Wi-Fi dan MQTT broker
3. File binary firmware baru (`.bin`) tersedia dan dapat diakses via **URL HTTP publik**

---

### Langkah 1 — Build Firmware Baru

Naikkan versi di `src/main.rs`:

```rust
const FW_VERSION: &str = "0.11.0"; // versi baru
```

Kemudian build:

```bash
cargo build --release
```

File binary ada di:
```
target/xtensa-esp32s3-espidf/release/iot
```

Konversi ke `.bin` dengan espflash:

```bash
espflash save-image --chip esp32s3 target/xtensa-esp32s3-espidf/release/iot firmware_v11.bin
```

---

### Langkah 2 — Host Firmware di Server HTTP

Firmware harus dapat diakses oleh ESP32 via URL HTTP. Beberapa pilihan hosting:

#### Opsi A: GitHub Releases (Direkomendasikan)

Upload `firmware_v11.bin` sebagai asset di GitHub Release, lalu gunakan URL raw:
```
https://github.com/AbelSukmadian/tugasIOT/releases/download/v0.11.0/firmware_v11.bin
```

#### Opsi B: Server Lokal (untuk testing)

Jalankan HTTP server sederhana di komputer yang **berada di jaringan yang sama** dengan ESP32:

```bash
# Python 3
python -m http.server 8080

# Atau dengan npx
npx serve . -p 8080
```

URL firmware:
```
http://192.168.x.x:8080/firmware_v11.bin
```

> ⚠️ ESP32 harus bisa menjangkau URL ini. Pastikan IP komputer & ESP32 berada di subnet yang sama, atau gunakan URL publik.

---

### Langkah 3 — Kirim Perintah OTA via MQTT

Kirim pesan JSON ke topik:
```
esp32/ESP32-S3-01/ota/command
```

Isi payload:
```json
{
  "version": "0.11.0",
  "url": "http://192.168.1.100:8080/firmware_v11.bin"
}
```

#### Menggunakan MQTTX (GUI)
1. Buka MQTTX → Hubungkan ke broker
2. Buat koneksi baru ke `mqtt://iotrust.duckdns.org:1883`
3. Isi **Username** dan **Password** sesuai konfigurasi
4. Pilih tab **Publish**
5. Set topik: `esp32/ESP32-S3-01/ota/command`
6. Paste payload JSON di atas → klik **Send**

#### Menggunakan `mosquitto_pub` (CLI)

```bash
mosquitto_pub \
  -h iotrust.duckdns.org \
  -p 1883 \
  -u "esp32_device" \
  -P "password_mqtt" \
  -t "esp32/ESP32-S3-01/ota/command" \
  -m '{"version":"0.11.0","url":"http://192.168.1.100:8080/firmware_v11.bin"}'
```

---

### Langkah 4 — Pantau Proses OTA via Serial Monitor

Jalankan monitor serial untuk melihat log OTA:

```bash
espflash monitor
```

Log yang akan muncul:

```
[INFO] Pesan MQTT Masuk! Topic: Some("esp32/ESP32-S3-01/ota/command") | Data: {"version":"0.11.0","url":"..."}
[INFO] OTA diterima: v0.11.0 dari http://...
[INFO] Starting OTA update from: http://...
[INFO] OTA update initiated. Setting up HTTP client...
[INFO] HTTP Response Status: 200
[INFO] Downloaded and written 4096 bytes...
[INFO] Downloaded and written 8192 bytes...
...
[INFO] Download complete. Total size: XXXXXX bytes
[INFO] Finalizing OTA...
[INFO] OTA Update successful! Rebooting in 3 seconds...
=== Firmware v0.11.0 (LED Blink Dummy) mulai ===
```

---

### Alur OTA (Detail Teknis)

```
MQTT Broker ──→ ESP32 MQTT Thread
                    │ mpsc::channel
                    ▼
              Main Loop (try_recv)
                    │ version check
                    ├─ [sama]  → abaikan
                    └─ [beda]  → spawn OTA Thread (stack 64KB)
                                      │
                                      ▼
                              EspOta::initiate_update()
                                      │
                                      ▼
                              HTTP GET firmware URL
                              (chunk 4KB, heap-allocated)
                                      │
                                      ▼
                              ota_update.write(chunk)  ←── loop
                                      │
                                      ▼
                              ota_update.complete()
                              (set boot partition)
                                      │
                                      ▼
                              sleep(3s) → esp_restart()
```

| Langkah | Detail |
|---------|--------|
| **Version Guard** | Jika `version == FW_VERSION`, OTA diabaikan tanpa error |
| **OTA Thread** | Dijalankan di thread terpisah dengan stack **64KB** untuk menghindari stack overflow |
| **Buffer Download** | Buffer 4KB di-alokasikan di **heap** (bukan stack) menggunakan `Box<[u8]>` |
| **Partisi Boot** | `ota_update.complete()` secara otomatis menandai partisi OTA aktif untuk boot berikutnya |
| **Rollback** | Jika firmware baru crash sebelum memanggil `ota_mark_valid`, ESP-IDF dapat rollback ke partisi sebelumnya |

---

### Troubleshooting OTA

| Masalah | Kemungkinan Penyebab | Solusi |
|---------|----------------------|--------|
| `HTTP request failed with status: 404` | URL firmware salah atau file tidak ada | Periksa URL dan pastikan file `.bin` tersedia |
| `OTA diabaikan` | Versi sama dengan yang sedang berjalan | Naikkan `version` di payload JSON |
| Download berhenti di tengah | Koneksi Wi-Fi tidak stabil | Pastikan sinyal Wi-Fi kuat, coba lagi |
| Perangkat tidak reboot | OTA thread error sebelum selesai | Lihat log serial untuk error detail |
| `Gagal parsing JSON OTA` | Format JSON salah | Pastikan payload memiliki field `version` dan `url` |

---

## 💾 Skema Partisi Flash (16MB)

| Nama | Tipe | Ukuran | Keterangan |
|------|------|--------|------------|
| `nvs` | data | 16KB | Non-volatile storage |
| `otadata` | data | 8KB | Metadata partisi OTA aktif |
| `phy_init` | data | 4KB | Kalibrasi RF |
| `factory` | app | 4MB | Firmware bawaan (factory) |
| `ota_0` | app | 4MB | Slot OTA pertama |
| `ota_1` | app | 4MB | Slot OTA kedua |

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
