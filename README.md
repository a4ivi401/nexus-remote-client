<div align="center">

# 🚀 Nexus Remote Client

**Ultra-low latency, cross-platform open-source remote desktop client built with Rust, GStreamer, and WebRTC**

[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange?style=flat-square)](https://www.rust-lang.org/)
[![Build Releases](https://github.com/a4ivi4/nexus-remote-client/actions/workflows/build-all.yml/badge.svg)](https://github.com/a4ivi4/nexus-remote-client/actions/workflows/build-all.yml)

**P2P WebRTC · H.264 Hardware-Accelerated Video · End-to-End Encryption · Zero-Latency**

</div>

---

## 📋 Обзор

**Nexus Remote Client** — это быстрый, открытый и кроссплатформенный аналог AnyDesk и TeamViewer. Приложение спроектировано для удалённого управления с ультранизкой задержкой (zero-latency) через прямой WebRTC P2P-канал с кодированием H.264.

| Особенность | Описание |
|-------------|----------|
| **Signaling Protocol v2** | Автоматическая генерация 6-значных Session ID, авторизация подключений |
| **GStreamer WebRTC** | Аппаратный захват экрана (`dxgiscreencapsrc`, `avfvideosrc`, `ximagesrc`) и low-latency H.264 кодек |
| **P2P NAT Traversal** | Автоматический STUN / TURN fallback |
| **Кроссплатформенность** | Готовые сборки для Windows (MSI + Portable ZIP), Linux (.deb + Tarball) и macOS (.pkg + Tarball) |
| **E2E Шифрование** | ChaCha20-Poly1305 и SHA-256 |

---

## ⚙️ Настройка адреса сигнального сервера

Клиент поддерживает несколько способов задания адреса сигнального сервера с чётким приоритетом:

1. **CLI аргумент `--server` / `-s`**:
   ```bash
   nexus-remote-client --server ws://46.101.237.80:3000/ws
   nexus-remote-client host -s ws://46.101.237.80:3000/ws
   nexus-remote-client connect 849201 -s ws://46.101.237.80:3000/ws
   ```
2. **Переменная окружения `NEXUS_SERVER`**:
   ```bash
   export NEXUS_SERVER="ws://46.101.237.80:3000/ws"
   nexus-remote-client
   ```
3. **Конфигурационный файл (`config.json`)**:
   Настройки сохраняются автоматически в каталог данных ОС:
   - **Linux**: `~/.config/nexus-remote/config.json`
   - **Windows**: `%APPDATA%\NexusRemote\config.json`
   - **macOS**: `~/Library/Application Support/NexusRemote/config.json`
4. **Интерактивное CLI / TUI Меню**:
   При запуске `nexus-remote-client` без параметров выберите пункт `⚙️ Settings` для смены и сохранения адреса сервера.

---

## 🚀 Использование

### 1. Интерактивный режим (TUI)
```bash
nexus-remote-client
```

### 2. Режим Хоста (Раздача экрана)
```bash
# Обычный запуск хоста (при входящем подключении спросит подтверждение)
nexus-remote-client host

# Автоматический приём всех входящих запросов
nexus-remote-client host --auto-accept
```

### 3. Режим Клиента (Подключение к хосту)
```bash
# Подключение по 6-значному Session ID
nexus-remote-client connect 849201
```

---

## 📦 Сборка и Упаковка

### Локальная сборка
```bash
# Клонирование репозитория
git clone https://github.com/a4ivi4/nexus-remote-client.git
cd nexus-remote-client

# Запуск тестов
cargo test

# Релизная компиляция
cargo build --release
```

### Windows Installer (WiX MSI & Portable ZIP)
Скрипт `scripts/build-msi.ps1` собирает:
1. Автономный **Portable ZIP** (включает бинарник `nexus-remote-client.exe`, все DLL GStreamer и плагины).
2. Полноценный **WiX v4 MSI инсталлятор** с системной регистрацией переменных окружения.

```powershell
pwsh .\scripts\build-msi.ps1
```

---

## 📄 Лицензия

MIT © [Nexus Remote Team](https://github.com/a4ivi4)
