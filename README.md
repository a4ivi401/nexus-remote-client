<div align="center">

# ✨ Nexus Remote Client

**Ultra-low latency, cross-platform remote desktop CLI with GStreamer and WebRTC**

[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange?style=flat-square)](https://www.rust-lang.org/)

**P2P · H.264 · End-to-End Encryption · Zero-latency**

</div>

---

## 📋 Обзор

**Nexus Remote Client** — это CLI-инструмент для удалённого рабочего стола с минимальной задержкой. Создан специально для задач, требующих мгновенного отклика (например, разработка модпаков для игр, где важен низкий пинг). Использует WebRTC (P2P) и аппаратное ускорение GStreamer.

| Особенность | Описание |
|-------------|----------|
| **CLI Интерфейс** | Красивое интерактивное меню в консоли (Inquire) |
| **GStreamer WebRTC** | H.264 с параметром `tune=zerolatency` |
| **P2P NAT Traversal** | Подключение напрямую даже за роутерами (ICE) |
| **E2E Шифрование** | Пароли и метаданные шифруются ChaCha20-Poly1305 |

## 🏗️ Архитектура

Структура клиентского приложения:

```
nexus-remote-client/
├── src/
│   ├── main.rs         # CLI меню, логика Host/Connect
│   ├── crypto.rs       # SHA-256 генерация ключей и ChaCha20 шифрование
│   ├── signaling.rs    # WebSocket клиент для обмена паролями
│   └── webrtc.rs       # GStreamer пайплайны (захват экрана, кодирование, WebRTC)
```

### Стек технологий

| Компонент | Библиотека |
|-----------|-----------|
| CLI Меню | [clap](https://docs.rs/clap/), [inquire](https://docs.rs/inquire/) |
| Видео пайплайн | [gstreamer](https://docs.rs/gstreamer/), [gstreamer-webrtc](https://docs.rs/gstreamer-webrtc/) |
| WebSocket Signaling | [tokio-tungstenite](https://docs.rs/tokio-tungstenite/) |
| Шифрование | [chacha20poly1305](https://docs.rs/chacha20poly1305/), [sha2](https://docs.rs/sha2/) |
| Асинхронность | [tokio](https://tokio.rs/) |

## 🚀 Быстрый старт

### Требования

- **Rust** ≥ 1.70 (edition 2021)
- **GStreamer** (включая плагины base, good, bad, ugly)
  - *macOS*: `brew install pkg-config gstreamer`
  - *Linux*: `sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav`
  - *Windows*: Установить GStreamer MSVC (будет упаковано в инсталлятор)

### Сборка

```bash
git clone https://github.com/a4ivi4/nexus-remote-client.git
cd nexus-remote-client
cargo build --release
```

### Запуск

Вы можете запустить приложение как в интерактивном режиме (без аргументов), так и передав аргументы напрямую.

```bash
# Интерактивное красивое CLI меню
cargo run --release

# Запуск в режиме Host (создаст случайный PIN-код или можно задать свой)
cargo run --release -- host --pin 123456

# Подключение к хосту (Viewer)
cargo run --release -- connect --pin 123456
```

## 🔒 Безопасность

- **E2EE (ChaCha20-Poly1305)** — Сигналинг сервер никогда не видит ваши IP адреса и WebRTC SDP-пакеты.
- **SHA-256** — При вводе PIN-кода на обеих станциях, из него локально генерируется 256-битный ключ.

## 📄 Лицензия

MIT © [a4ivi4](https://github.com/a4ivi4)
