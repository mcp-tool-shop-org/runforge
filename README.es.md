<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.md">English</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
</p>

<p align="center"><img src="https://raw.githubusercontent.com/mcp-tool-shop-org/brand/main/logos/runforge/readme.png" alt="RunForge" width="720"></p>

<p align="center"><img src="docs/bench-dark.png" alt="The history bench in the dark theme, open on a fixture folder" width="720"></p>

<p align="center">
  <a href="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml"><img src="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT License"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/"><img src="https://img.shields.io/badge/Landing_Page-RunForge-blue" alt="Landing page"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/handbook/"><img src="https://img.shields.io/badge/Handbook-RunForge-06b6d4" alt="Handbook"></a>
</p>

# RunForge

RunForge es una herramienta de Windows para registrar datos de entrenamiento. Abra una carpeta. Una carpeta de series muestra cada muestra almacenada, la receta compartida y un informe en texto sin formato. Una carpeta de retropropagación abre el panel de historial: la lista de ejecuciones, la pérdida almacenada, una comparación de dos filas y la exportación.

La imagen anterior es el panel de historial. Una carpeta de series es una pantalla diferente.

La retropropagación es el entrenador. Esta aplicación no contiene el entrenador, no descarga un modelo y no incluye PyTorch. Cuando `backprop` ya está en PATH, al hacer clic en Entrenar, Evaluar y Exportar modelo en el panel de historial, se inicia ese comando y se sigue su registro. Los argumentos son generados por la aplicación. Nada se pasa a través de una shell. Si `backprop` falta, los botones lo indican. RunForge no descarga la retropropagación, no la instala ni la incluye.

## Una carpeta de series

El archivo es `run-config*.json` en la carpeta que abre, y los mismos nombres están un nivel más abajo en una carpeta secundaria. RunForge no busca más abajo ni realiza búsquedas en el disco. Un archivo incorrecto se omite y se cuenta. Una clave duplicada rechaza solo ese archivo.

Cada muestra permanece como un registro: la época o el paso, la pérdida, la tasa de aprendizaje y cualquier otro campo que se haya registrado. Cada muestra finita se muestra. La vista no se vuelve a muestrear. Una pérdida nula o no finita es un espacio en blanco, no un cero. `training_summary.final_loss`, cuando el archivo lo tiene, es un marcador junto a la curva. No se agrega a la línea y el informe no se clasifica por ello.

El archivo adjunto imprime un informe a partir de esas mediciones. El panel y Guardar informe son las mismas palabras. Guardar utiliza el mismo cuadro de diálogo que la exportación del historial. Preguntar puede agregar una nota corta sobre cómo leer la página. Esa nota puede no contener un dígito, nombrar una configuración o nombrar un resultado. Una nota que exceda los límites se descarta y el panel lo indica. Si no hay un modelo local que responda, el informe sigue siendo válido.

El modelo, cuando lo solicita, es un Ollama local en el puerto `127.0.0.1` `11434`. Un nombre etiquetado como "nube" no se elige. El archivo adjunto no presiona Entrenar. La pregunta no incluye la ruta de la carpeta. Una nota guardada permanece con las preferencias. No se vuelve a escribir en los archivos de series.

## El panel de historial

Abra la carpeta que contiene `run_history.json` o la carpeta que está un nivel por encima de un directorio `output`. El archivo en la carpeta abierta tiene prioridad cuando ambos existen. La ventana muestra las ejecuciones, muestra la pérdida almacenada, compara dos filas y exporta la tabla o la curva.

La curva es la `loss_history` almacenada, en orden de archivo, con un máximo de las muestras que el programa de entrenamiento ha conservado. `final_loss` es una columna. No se añade a la línea. Una muestra nula es un espacio vacío, no un cero.

Entrenar, Evaluar y Exportar modelo permanecen en esta pantalla. No están en la pantalla de series.

## Modelo de amenazas

RunForge lee una carpeta que elige. Una carpeta de historial se abre `run_history.json` allí, o `output/run_history.json` un nivel más abajo. Una carpeta de series se abre `run-config*.json` en esa carpeta y en sus carpetas secundarias inmediatas. La aplicación no recorre el resto del disco y no fusiona los dos tipos de registros. Exportar y Guardar informe se escriben en una ruta que elige. Las preferencias, la última carpeta y el tema se escriben en el paquete LocalState cuando la aplicación está empaquetada, y junto al archivo ejecutable cuando no lo está.

Entrenar, Evaluar y Exportar modelo se inician `backprop` solo cuando presiona el botón en el panel de historial y ese programa ya está en PATH. El registro es la salida de ese programa. Detener finaliza el árbol de procesos que inició esta ventana. El archivo adjunto no presiona esos botones.

El archivo adjunto puede conectarse a un servidor de modelos en la dirección de bucle invertido. El manifiesto del paquete no solicita `internetClient`. No hay telemetría ni cuenta. La lista de referencia del informe está dentro del programa. No se obtiene.

Datos que no toca: el entrenador, una descarga de modelo, una instalación de retropropagación, una shell, una copia del entorno, un modelo en la nube o una reescritura en los archivos de series o `run_history.json`.

Los permisos permanecen en la carpeta que abrió, la ruta de exportación que elige, el archivo de datos que elige, el archivo de preferencias y el puerto de bucle invertido `11434` cuando solicita el modelo local.

Cómo informar sobre una vulnerabilidad se indica en [SECURITY.md](SECURITY.md).

## Compilación

Rust 1.98.1, edición 2024. El archivo de la cadena de herramientas lo especifica.

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

CI ejecuta el comando de cobertura y carga `lcov.info`. Codecov marca la compilación como fallida cuando la cobertura de líneas es inferior al 90 %. El cuadro de diálogo de archivo no se abre durante las pruebas.

## Tienda

El anuncio publicado es el producto `9PHL1HX0CGMF`, el paquete `mcp-tool-shop.RunForge-Desktop`. La versión 2 reemplaza la aplicación de clasificación anterior, y el texto del anuncio debe indicarlo en el mismo envío. Este repositorio aún no contiene ese paquete. La identidad del paquete no cambia cuando se compila el paquete.

El diseño de referencia es [docs/CONTRACT.md](docs/CONTRACT.md). Este repositorio admite la compilación de la fuente 2.0.0. La aplicación de la tienda publicada sigue siendo la versión 1.0.1 hasta que se envíe un paquete con una versión superior a `1.0.1.0`.

Creado por [MCP Tool Shop](https://mcp-tool-shop.github.io/).
