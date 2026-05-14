# JackyNet

Monitor de red ligero para Linux. Muestra qué procesos están usando tu conexión en tiempo real y el consumo histórico por interfaz, con una pequeña ventana flotante opcional (Picture-in-Picture) para tener los datos a la vista mientras trabajas.

Construido sobre [Tauri 2](https://tauri.app/) (Rust + Vue 3) y dos utilidades clásicas de Linux: `nethogs` (tráfico por proceso) y `vnstat` (acumulado por interfaz).

## Origen

Este proyecto nació después de quedarme sin conexión a internet durante 3 días debido a problemas técnicos con mi proveedor. Buscando una forma de seguir trabajando, terminé usando los datos móviles de mi teléfono mediante USB tethering conectado a mi laptop.

El problema fue que estaba consumiendo datos prácticamente a ciegas. En apenas 30 minutos había gastado cerca de 1 GB sin tener claro qué aplicaciones o procesos eran responsables.

Necesitaba más visibilidad: saber exactamente qué programas estaban usando la red y cuánto estaban consumiendo en tiempo real. Herramientas como nethogs y vnstat ya ofrecían parte de esa información, pero quería algo más integrado, minimalista y adaptado a mi flujo de trabajo diario. Así nació JackyNet.

## Características

- **Tabla en vivo** con cada proceso que está enviando o recibiendo paquetes, sus velocidades de subida/bajada, PID y usuario. Click en una fila para ver la ruta completa del binario, su cmdline, asignarle un alias o terminar el proceso.
- **Consumo histórico** por interfaz de red: totales, diario, mensual. Cada interfaz es colapsable y se le puede dar un nombre amigable (`enxa65ac2b88521` → "Mobile Data").
- **Aliases persistentes** para procesos e interfaces, guardados en `~/.config/jackynet/config.json`. Los aliases se resuelven incluso para apps Electron (Slack, VS Code, Chrome) que comparten `/proc/self/exe`, gracias a la resolución vía `readlink /proc/<pid>/exe`.
- **Estadísticas de sesión**: hora de inicio, tiempo online y total acumulado de subida/bajada desde que JackyNet arrancó.
- **Modo PiP**: ventana mini, siempre visible, arrastrable y redimensionable.
- **Tray icon** en la barra superior de Ubuntu. Cerrar la ventana no cierra la app, queda corriendo en segundo plano; menú del tray para mostrar, alternar PiP, refrescar o salir.
- **Hotkey global** configurable para mostrar/ocultar la ventana principal.
- **Autostart** opcional en el arranque de sesión.

## Requisitos

- Ubuntu 22.04+ (o cualquier distro Linux con GTK 3 y WebKit2GTK 4.1).
- `nethogs` y `vnstat` instalados en el sistema.
- `policykit-1` y `libayatana-appindicator3-1` (para pkexec y el icono de tray).
- En GNOME (Ubuntu por defecto): la extensión **AppIndicator and KStatusNotifierItem Support** para que aparezca el tray. Suele venir activada en Ubuntu; si no, `sudo apt install gnome-shell-extension-appindicator` y actívala.

Para **compilar** desde código (no necesario si instalas el `.deb`):

- Rust 1.77+ (instala con [rustup](https://rustup.rs/)).
- Node.js 20+ y npm.
- Dependencias de build de Tauri: `build-essential libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libxdo-dev`.

## Instalación

### Opción A — paquete .deb (recomendado)

Descarga el `.deb` más reciente desde la sección de Releases del repositorio e instálalo:

```bash
sudo apt install ./JackyNet_<version>_amd64.deb
```

apt resolverá automáticamente las dependencias (`nethogs`, `vnstat`, `policykit-1`, `libayatana-appindicator3-1`).

### Opción B — compilar desde código

```bash
git clone https://github.com/jaguilar025/JackyNet
cd JackyNet
npm install
npm run tauri:build
sudo apt install ./src-tauri/target/release/bundle/deb/JackyNet_*.deb
```

### Opción C — AppImage portable

Descarga el `.AppImage` desde Releases, dale permisos de ejecución y úsalo donde quieras:

```bash
chmod +x JackyNet_<version>_amd64.AppImage
./JackyNet_<version>_amd64.AppImage
```

Con esta opción debes instalar las dependencias del sistema a mano:

```bash
sudo apt install nethogs vnstat policykit-1 libayatana-appindicator3-1
```

## Configuración post-instalación: permisos de nethogs

`nethogs` necesita privilegios de root para mapear conexiones de procesos de otros usuarios (por ejemplo daemons del sistema como `nordvpnd`). JackyNet lo lanza mediante `pkexec`, que por defecto pide contraseña en cada arranque.

Para que **no pida contraseña**, ejecuta una sola vez:

```bash
# Si instalaste el .deb:
sudo /usr/share/jackynet/scripts/install-polkit-rule.sh

# Si compilaste desde código:
sudo ./scripts/install-polkit-rule.sh
```

El script crea una regla en `/etc/polkit-1/rules.d/50-jackynet-nethogs.rules` que permite a tu usuario lanzar `nethogs` vía pkexec sin prompt. Sobrevive a reinicios y actualizaciones.

Para revertir:

```bash
sudo rm /etc/polkit-1/rules.d/50-jackynet-nethogs.rules
```

## Uso

Abre JackyNet desde el menú de aplicaciones de Ubuntu o búscalo con la tecla Super.

- **Refresh**: vuelve a leer datos de nethogs y vnstat (no resetea la base de datos de vnstat).
- **PIP**: abre la ventana flotante mini. Para cerrarla, click en la `✕` del PiP o usa el menú del tray.
- **Settings**: configura el hotkey global, autostart y activación del PiP.
- **Renombrar un proceso/interfaz**: click en la fila para expandirla, luego "Rename".
- **Matar un proceso**: click en la fila → "Kill PID". Pedirá confirmación.
- **Tray icon**: click derecho para acceder al menú; cerrar la ventana principal solo la oculta.

La configuración (aliases, hotkey, etc.) se guarda en `~/.config/jackynet/config.json`.

## Desarrollo

```bash
git clone https://github.com/jaguilar025/JackyNet
cd JackyNet
npm install
npm run tauri:dev
```

Esto levanta Vite en `127.0.0.1:1420` y arranca la ventana Tauri en modo desarrollo con hot reload del frontend.

Estructura:

```
JackyNet/
├── src/                   # Frontend Vue 3
│   └── components/        # Dashboard, ProcessTable, HistoricalView, PipView, SessionFooter, SettingsPanel
├── src-tauri/             # Backend Rust + configuración Tauri
│   ├── src/
│   │   ├── lib.rs         # Setup, tray, hotkeys
│   │   ├── commands.rs    # Comandos expuestos al frontend
│   │   ├── nethogs.rs     # Stream y parsing de nethogs
│   │   ├── vnstat.rs      # Parseo de vnstat --json
│   │   └── config.rs      # Persistencia de aliases/preferencias
│   ├── capabilities/      # Permisos Tauri
│   ├── icons/             # Iconos de la app
│   └── tauri.conf.json
├── scripts/
│   └── install-polkit-rule.sh
└── public/                # Assets servidos en runtime
```

## Desinstalar

```bash
sudo apt remove jackynet
sudo rm /etc/polkit-1/rules.d/50-jackynet-nethogs.rules
rm -rf ~/.config/jackynet
```

## Créditos

by jaacker25 — 2026 — v1.0
