<p align="center">
  <a href="README.ja.md">日本語</a> | <a href="README.zh.md">中文</a> | <a href="README.md">English</a> | <a href="README.fr.md">Français</a> | <a href="README.hi.md">हिन्दी</a> | <a href="README.it.md">Italiano</a> | <a href="README.pt-BR.md">Português (BR)</a>
</p>

<p align="center"><img src="https://raw.githubusercontent.com/mcp-tool-shop-org/brand/main/logos/runforge/readme.png" alt="RunForge" width="720"></p>

<p align="center">
  <a href="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml"><img src="https://github.com/mcp-tool-shop-org/runforge/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/mcp-tool-shop-org/runforge"><img src="https://codecov.io/gh/mcp-tool-shop-org/runforge/graph/badge.svg" alt="Coverage"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue" alt="MIT License"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/"><img src="https://img.shields.io/badge/Landing_Page-RunForge-blue" alt="Landing page"></a>
  <a href="https://mcp-tool-shop-org.github.io/runforge/handbook/"><img src="https://img.shields.io/badge/Handbook-RunForge-06b6d4" alt="Handbook"></a>
</p>

# RunForge

RunForge es una herramienta para Windows que permite ajustar con precisión las ejecuciones. Abra una carpeta de ejecuciones y mostrará cada muestra almacenada, generará un informe que comenzará con su conclusión y proporcionará a un modelo local un espacio de trabajo. En el espacio de trabajo, el modelo crea sus propias herramientas de fórmula, propone la función de cada parámetro y recopila datos de diferentes carpetas hasta que se llega a una conclusión.

RunForge lee los registros de entrenamiento. No realiza el entrenamiento. No descarga un modelo, instala PyTorch ni utiliza un modelo en la nube.

## El informe

Abra una carpeta de archivos `run-config*.json`. Pueden estar en la propia carpeta o en un nivel inferior. RunForge muestra cada muestra finita, sin volver a muestrear, y una discontinuidad permanece como tal. `training_summary.final_loss` es un marcador junto a la curva, nunca un punto sobre ella, y el informe nunca clasifica en función de él.

El informe comienza con **En resumen**: si una ejecución tiene éxito y por qué. Para cinco semillas de una receta, el informe podría ser así:

> Ninguna ejecución tiene éxito. La semilla 512 tiene el punto más bajo, 0.0674 en la época 3. La semilla 1024 tiene el tramo más estable alrededor de su valor mínimo: un valor medio de 0.3474, en comparación con 0.4041 alrededor del valor mínimo de la semilla 512. Los valores medios de las ejecuciones se encuentran a una distancia de 0.1508 entre sí: mayor que la mitad del tramo más estable (0.1133), menor que el de la ejecución más ruidosa (0.3238). Las ejecuciones solo se separan parcialmente.

Después de eso, el informe detalla:
- las ejecuciones
- por qué una tiene éxito o ninguna
- qué cambió y qué no
- evaluaciones anteriores de la misma receta
- qué hacer a continuación
- qué no puede indicar el informe
- de dónde proviene cada fórmula

No se imprimirá una sección que no tenga nada que decir. Una configuración que fue la misma en todas las ejecuciones se indicará como no probada, nunca como un parámetro. El panel y el botón "Guardar informe" contienen el mismo texto.

## El espacio de trabajo

Presione **Preguntar** y un modelo local analizará las ejecuciones a través de cinco herramientas: medir una fórmula, comparar un parámetro que cambió, mantener una nueva herramienta, proponer una hipótesis y finalizar. El programa calcula cada resultado y escribe cada oración que contenga un número. El modelo elige qué analizar y lo expresa con palabras. Su nota final se etiqueta como sus palabras, no como una medición.

**Herramientas que crea el modelo.** Una herramienta es una fórmula en un lenguaje pequeño que se evalúa una vez por ejecución, por ejemplo:
- `last / low`: qué tan alto sube la curva después de su valor mínimo
- `slope_between(end_epoch - 1, end_epoch)`: qué tan pronunciada es la última época
- `knob('lora_r')`: un valor de receta

Una fórmula no puede leer un archivo, abrir la red ni ejecutar código. Solo se conserva una nueva herramienta si proporciona un valor en cada ejecución y no es un duplicado. Permanece provisional hasta que se utiliza en una segunda carpeta. Las fórmulas posteriores pueden utilizarla por su nombre, por lo que la biblioteca crece con los datos. Puede probar una fórmula usted mismo en el cuadro de fórmulas del panel.

**Hipótesis sobre los parámetros.** Una hipótesis nombra un parámetro, una fórmula y una dirección, por ejemplo, "cuando el rango de LoRA aumenta, `last / low` disminuye". Su prueba se fija cuando se propone. En cada carpeta, el programa la marca como una de las siguientes:
- no se puede probar: el parámetro no cambió
- confusa: otro parámetro cambió con él
- inconclusa
- o un resultado solo para esas ejecuciones

Cuando las ejecuciones no pueden resolver una hipótesis, RunForge planifica el conjunto más pequeño de ejecuciones que: un parámetro, dos configuraciones, tres semillas o más cada una. Nunca las inicia.

**Evidencia en diferentes carpetas.** Cada carpeta proporciona un valor e: una medida de la evidencia que promedia exactamente 1 cuando el parámetro no tiene efecto, por lo que se puede multiplicar en diferentes carpetas sin perder validez. Se excluyen dos tipos de carpetas:
- cualquier carpeta que contenga una ejecución que RunForge ya haya visto cuando se registró la hipótesis
- una ejecución que ya se ha contado

Las conclusiones, ya sean confirmadas o refutadas, se emiten solo en los puntos de control, uno cada cinco nuevas carpetas. Cada punto de control aplica e-BH con una tasa de falsos descubrimientos del 5% en ambas direcciones de cada hipótesis del espacio de trabajo. Una sola hipótesis necesita aproximadamente tres carpetas limpias de tres ejecuciones por configuración. El método, sus fuentes y una revisión externa se encuentran en [docs/sidecar-workbench.md](docs/sidecar-workbench.md) y [docs/evidence.consult.response.md](docs/evidence.consult.response.md).

El modelo es un Ollama local en `127.0.0.1:11434`. RunForge utiliza solo un modelo que Ollama indica que puede llamar a herramientas y omite los nombres etiquetados como de la nube. Una sesión tiene un límite de seis solicitudes y diez llamadas a herramientas. Sin un modelo local, el informe sigue siendo válido, al igual que el cuadro de fórmulas.

## El espacio de trabajo del historial

Una carpeta con una retropropagación `run_history.json` abre el espacio de trabajo del historial, o la carpeta superior a un directorio `output`. El espacio de trabajo muestra las ejecuciones, dibuja las `loss_history` almacenadas en orden de archivo, compara dos filas y exporta la tabla o la curva.

Cuando `backprop` ya está en PATH, los botones "Entrenar", "Evaluar" y "Exportar modelo" en este espacio de trabajo inician ese comando y siguen su registro. La aplicación construye los argumentos y nada pasa por una shell. Si falta `backprop`, los botones lo indican. RunForge no descarga, instala ni incluye retropropagación.

<p align="center"><img src="docs/bench-dark.png" alt="The history bench in the dark theme, open on a fixture folder" width="720"></p>

## Modelo de amenazas

**Lo que lee.** Una carpeta que usted elige:
- una carpeta de historial: `run_history.json` allí, o `output/run_history.json` un nivel más abajo
- una carpeta de series: `run-config*.json` allí y en sus carpetas inmediatas

No recorre el resto del disco y nunca vuelve a escribir en esos archivos. "Exportar" y "Guardar informe" escriben en una ruta que usted elige.

**Lo que guarda.** Las preferencias (la última carpeta y el tema) y `sidecar-memory.json` se guardan juntas: en el LocalState del paquete cuando la aplicación está empaquetada y junto al archivo ejecutable cuando no lo está. El archivo de memoria no contiene ninguna ruta de carpeta. Contiene:
- las notas del modelo
- cada evaluación medida
- las herramientas de fórmula aprendidas (un máximo de 50)
- las hipótesis con sus resultados de prueba (un máximo de 60)
- los puntos de control

**Qué se transmite al modelo local.** Los nombres de las ejecuciones, los valores de la receta y los resultados de las herramientas propias del programa se envían al puerto de bucle `11434`. La ruta de la carpeta no se envía. El modelo solo puede llamar a las cinco herramientas del entorno de trabajo, y el programa valida cada llamada.

**Qué se inicia.** El inicio del entrenamiento, la evaluación y la exportación del modelo `backprop` solo se produce cuando se presiona el botón en el panel de historial y el programa ya está en la ruta PATH. La función de detención finaliza el árbol de procesos que inició esta ventana. El programa secundario nunca presiona esos botones.

**Qué nunca se modifica.** El manifiesto del paquete no solicita `internetClient`. No hay telemetría ni cuenta, y la lista de referencias del informe está integrada en el programa. La aplicación no tiene un modelo en la nube, ni una shell, ni una copia del entorno, ni un entrenador, ni descarga modelos.

Cómo informar sobre una vulnerabilidad se indica en [SECURITY.md](SECURITY.md).

## Compilación

Rust 1.98.1, edición 2024. El archivo de la cadena de herramientas lo especifica.

```bash
cargo test --locked --workspace
cargo llvm-cov --locked --workspace --all-targets --lcov --output-path lcov.info --remap-path-prefix --fail-under-lines 90
cargo run -p runforge --locked
```

Umbrales de cobertura:
- Las pruebas de CI fallan si la cobertura de líneas es inferior al 90 %.
- Codecov exige que tanto el proyecto como las nuevas líneas de cada solicitud de incorporación de código alcancen el 90 %.

Las pruebas ejecutan el bucle del modelo contra una instancia falsa de Ollama en el puerto de bucle. Una sesión con un modelo local real es opcional:

```bash
RUNFORGE_LIVE_FOLDER=<series folder> cargo test -p runforge --features live live_workbench -- --nocapture
```

## Tienda

El anuncio publicado es el producto `9PHL1HX0CGMF`, el paquete `mcp-tool-shop.RunForge-Desktop`. La versión 2 reemplaza la aplicación de clasificación anterior, y el texto del anuncio debe indicarlo en la misma presentación. La aplicación de la tienda publicada permanece en la versión 1.0.1 hasta que se presente un paquete superior a `1.0.1.0`. El diseño de referencia se encuentra en [docs/CONTRACT.md](docs/CONTRACT.md).

Creado por [MCP Tool Shop](https://mcp-tool-shop.github.io/).
