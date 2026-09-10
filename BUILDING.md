# Сборка RustifyTickets

Проект состоит из трёх частей:

| Часть | Путь | Назначение |
|---|---|---|
| Ядро | `src/` | Классификация, чтение xlsx, Excel-отчёт с диаграммами |
| CLI | `src/main.rs` | Консольный запуск без GUI |
| GUI | `src-tauri/` + `ui/` | Tauri-приложение (Rust-backend + TypeScript-фронтенд) |

## Требования

- **Rust** 1.85+ (`rustup`) — в проекте используется edition 2024
- **Node.js** 20+ и npm — для сборки фронтенда
- **Tauri CLI**: `cargo install tauri-cli --version "^2"`
- **Системные зависимости** — только для GUI:
  - **Linux (Fedora)**: `sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel`
  - **Linux (Ubuntu/Debian)**: `sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf build-essential`
  - **Windows**: Visual Studio Build Tools с компонентом «Desktop development with C++» (WebView2 уже входит в Windows 10/11)
  - **macOS**: Xcode Command Line Tools (`xcode-select --install`)

Для CLI (`cargo run`) GUI-зависимости не нужны.

## CLI-версия

```bash
cargo run -- tickets.xlsx classified_tickets.xlsx   # вход и выход можно не указывать
```

## GUI: разработка

```bash
cd ui && npm ci          # один раз
cd ../src-tauri && cargo run
```

При запуске через `cargo run` фронтенд берётся из уже собранного `ui/dist`.

Для разработки интерфейса удобнее Vite с горячей перезагрузкой — запустите его
отдельно, а Tauri подключится к `devUrl` из `tauri.conf.json`:

```bash
cd ui && npm run dev      # терминал 1, порт 1420
cd src-tauri && cargo tauri dev   # терминал 2
```

## GUI: релизная сборка

Фронтенд нужно собрать до запуска `tauri build` — иначе Tauri не найдёт `ui/dist`:

```bash
cd ui && npm ci && npm run build
cd ../src-tauri && cargo tauri build            # текущая ОС, все форматы
cd ../src-tauri && cargo tauri build --bundles deb,rpm
```

Готовые установщики появятся в `src-tauri/target/release/bundle/`.

**Кросс-компиляция невозможна**: Tauri линкуется с системным webview
(WebKitGTK / WebView2 / WKWebView), поэтому сборка под каждую ОС идёт на своей ОС.

## Портативная версия

Установка не требуется — приложение запускается прямо из exe:

- **Windows** — `RustifyTickets.exe` (тот же релизный бинарник, что и в установщике).
  Требуется WebView2: в Windows 10 (с 2020 года) и Windows 11 он уже входит в систему
- **Linux** — готовый `.AppImage` из `bundle/appimage/`, он портативный по своей природе
- **macOS** — `RustifyTickets.app` из `bundle/macos/`, его достаточно перенести в любую папку

Windows-сборка добавляет portable-версию автоматически: в CI выгружается отдельный
артефакт `RustifyTickets-windows-portable` с двумя файлами — `RustifyTickets.exe`
и стандартным `categories.toml`. Распакуйте их в одну папку и запускайте — настройки
будут храниться рядом с exe, ничего в системе не создаётся.

Правила поиска конфига при запуске:

1. `categories.toml` рядом с exe — сюда попадают портативные сборки;
2. `categories.toml` в текущей папке — запуск из репозитория;
3. рядом с exe, если папка доступна для записи — portable остаётся самодостаточным,
   даже если файл рядом удалили;
4. пользовательская папка настроек — для установленных сборок, где каталог программы
   защищён от записи: `%APPDATA%\RustifyTickets` (Windows) или
   `~/.config/rustifytickets` (Linux/macOS).

## Автоматическая сборка на GitHub Actions

Workflow `.github/workflows/build.yml` собирает приложение на всех трёх ОС:

1. **Тесты ядра** — `cargo test` на Ubuntu (быстро, без GUI-зависимостей)
2. **Сборка** — матрица: Linux (deb/rpm/AppImage), Windows (msi/exe), macOS (Apple Silicon и Intel)

Запускается при:

- пуше в `master`/`main` и в pull request — установщики доступны как артефакты запуска
  (вкладка **Actions** → нужный запуск → раздел **Artifacts**)
- пуше тега вида `v1.2.3` — дополнительно создаётся **черновик релиза** со всеми установщиками
- вручную через **Actions → Build → Run workflow**

Артефакты запуска:

| Артефакт | Содержимое |
|---|---|
| `RustifyTickets-linux` | `.deb`, `.rpm`, `.AppImage` |
| `RustifyTickets-windows` | `.msi`, `-setup.exe` |
| `RustifyTickets-windows-portable` | `RustifyTickets.exe` + `categories.toml` |
| `RustifyTickets-macos-arm` | `.dmg` и `.app` для Apple Silicon |
| `RustifyTickets-macos-intel` | `.dmg` и `.app` для Intel |

Чтобы выпустить версию:

```bash
# версия указывается в src-tauri/tauri.conf.json и src-tauri/Cargo.toml
git tag v0.2.0
git push origin v0.2.0
```

Проверить сборку до пуша можно локально только под свою ОС — остальные платформы
собираются в CI.

## Иконки

Набор иконок лежит в `src-tauri/icons/`: `32x32.png`, `128x128.png`,
`128x128@2x.png`, `icon.ico` (Windows), `icon.icns` (macOS).
Перегенерировать из одной картинки: `cargo tauri icon путь/к/logo.png`.

## Конфигурация категорий

- `categories.toml` в корне — **стандартный** набор правил, он вшивается в бинарь
  при компиляции (`include_str!`). Правки в нём попадают в сборку автоматически
- рабочий конфиг ищется в порядке: рядом с исполняемым файлом → в текущей папке →
  в пользовательской папке настроек (`%APPDATA%\RustifyTickets` в Windows,
  `~/.config/rustifytickets` в Linux/macOS)
- кнопка «Сделать стандартными» в GUI сохраняет текущие правила в
  `categories.default.toml` рядом с рабочим конфигом — именно их восстановит
  «Сбросить к стандартным»
- тесты `tests/defaults.rs` проверяют, что встроенный стандарт валиден и совпадает
  с рабочим конфигом
