<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.md">English</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
</p>

<p align="center"><img src="https://raw.githubusercontent.com/mcp-tool-shop-org/brand/main/logos/runforge/readme.png" alt="RunForge" width="720"></p>

<p align="center"><img src="docs/bench-dark.png" alt="The RunForge window in the dark theme, open on a fixture folder" width="720"></p>

<p align="center">
  <a href="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml"><img src="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT License"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/"><img src="https://img.shields.io/badge/Landing_Page-RunForge-blue" alt="Landing page"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/handbook/"><img src="https://img.shields.io/badge/Handbook-RunForge-06b6d4" alt="Handbook"></a>
</p>

# RunForge

RunForge es la herramienta de Windows para analizar la carpeta de resultados de [backpropagate](https://github.com/mcp-tool-shop-org/backpropagate). Abra la carpeta que contiene `run_history.json` o la carpeta que se encuentra un nivel superior a un directorio `output`. La ventana muestra las ejecuciones, dibuja la pérdida almacenada, compara dos filas y exporta la tabla o la curva.

Backpropagate es el programa de entrenamiento. Esta aplicación no contiene el programa de entrenamiento, no descarga un modelo y no incluye PyTorch. Cuando `backprop` ya está en la variable PATH, los botones "Entrenar", "Evaluar" y "Exportar modelo" inician ese comando y siguen su registro. Los argumentos son generados por la aplicación. No se pasa nada a través de una terminal. Si `backprop` no está presente, los botones lo indican. RunForge no descarga, instala ni incluye backpropagate.

La curva es la `loss_history` almacenada, en orden de archivo, con un máximo de las muestras que el programa de entrenamiento ha conservado. `final_loss` es una columna. No se añade a la línea. Una muestra nula es un espacio vacío, no un cero.

## Modelo de amenazas

RunForge lee una carpeta que usted selecciona. Abre `run_history.json` en esa carpeta o `output/run_history.json` un nivel más abajo, y puede exportar la tabla, la curva o una entrada. Las preferencias, la última carpeta y el tema se guardan en la carpeta LocalState del paquete cuando la aplicación está empaquetada, y junto al archivo ejecutable cuando no lo está. Los botones "Entrenar", "Evaluar" y "Exportar modelo" inician `backprop` solo cuando se presiona el botón y ese programa ya está en la variable PATH. El registro es la salida de ese programa. "Detener" finaliza el árbol de procesos que inició esta ventana.

Datos que no se modifican: el programa de entrenamiento, la descarga de un modelo, la instalación de backpropagate, una terminal, una copia del entorno o la telemetría. No hay ninguna cuenta.

Los permisos se mantienen en la carpeta que se abrió, la ruta de exportación que se seleccionó, el archivo de datos que se seleccionó y el archivo de preferencias. La aplicación no solicita ninguna capacidad de red.

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
