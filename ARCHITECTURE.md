# Arquitectura simplificada

## Capas y responsabilidades

- **`main/drivers/`**: adaptadores de hardware (I2C, RTC, OLED, encoder, relé, DHT22). No guardan estado de negocio, sólo exponen primitivas de bajo nivel.
- **`main/app_state*.{c,h}`**: reglas de negocio y persistencia.
  - `app_state_logic`: helpers de horarios y defaults.
  - `app_state_storage`: persistencia en NVS.
  - `app_state`: punto de entrada thread-safe para leer/escribir el estado global.
- **`main/tasks/`**: tareas FreeRTOS que orquestan flujos concretos.
  - `control_task`: decide cuándo encender el relé según horario.
  - `input_task`: normaliza el encoder/botón a eventos.
  - `dht_task`: lee ambiente y notifica.
- **`main/ui/`**: presentación desacoplada en dos partes.
  - `ui_controller`: lógica de navegación (menús, confirmaciones, guardado de hora/modos).
  - `ui_task`: loop de UI que coordina eventos, estado y renderizado.
  - `framebuffer` y `screens`: primitives y vistas.

## Flujo principal

1. `app_main` inicializa NVS/estado y hardware.
2. Se arrancan las tareas (`ui`, `input`, `control`, `dht`).
3. `input_task` entrega eventos normalizados.
4. `ui_task` consume eventos, delega navegación a `ui_controller`, vuelve a renderizar con `screens` y hace flush al OLED.
5. `control_task` consulta `app_state` y enciende/apaga el relé según horario.

## Convenciones prácticas

- Mantener los archivos debajo de ~200 líneas de código y extraer helpers cuando crezcan.
- Cada módulo debe tener una responsabilidad principal; evita mezclar hardware, negocio y UI.
- Prefiere funciones pequeñas y puras; maneja efectos secundarios en capas superiores (tareas o controladores).
- Si necesitas un nuevo flujo, crea una tarea o un controlador pequeño que use `app_state` en vez de duplicar lógica.
