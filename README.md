# Bitgrain

Compresor de imágenes en escala de grises, tipo JPEG: toma una imagen pesada y devuelve la misma imagen con menos peso (formato `.bg`). Incluye encoder y decoder en una misma API y CLI, estilo FFmpeg.

## Utilidad real: backend para cámaras y dispositivos

Pensado como **backend de compresión** en dispositivos con cámara (teléfonos, tablets, cámaras embebidas): la captura genera una imagen que pesa mucho; Bitgrain la comprime a `.bg` y así **ahorra espacio** en almacenamiento o en transmisión. Cuando el usuario quiere **ver la foto**, el sistema decodifica el `.bg` y obtiene de nuevo la imagen en memoria: **visible, lista para mostrarla en pantalla** o para guardarla en un formato estándar (PGM, JPEG, PNG) y abrirla en la galería o en cualquier visor.

Flujo típico en un dispositivo:

1. **Captura** → imagen en crudo o de alta resolución (muchos MB).
2. **Compresión** → Bitgrain (encode) genera un `.bg` más liviano; se guarda en disco o se envía.
3. **Visualización** → El usuario abre la foto → Bitgrain (decode) reconstruye la imagen a partir del `.bg` → la app la **muestra en pantalla** (o la exporta a JPEG/PNG para compatibilidad).

La utilidad no es solo “recuperar datos”: es **recuperar la imagen para que sea visible y utilizable** — mostrarla, editarla, compartirla. El decoder devuelve píxeles listos para dibujar en pantalla o para escribir en un archivo que cualquier visor entienda.

---

## Requisitos

- **Rust** (toolchain estable: `rustup default stable`)
- **GCC** (C11) y **make**

---

## Compilación

```bash
make build
```

Genera el binario `bitgrain`. Para limpiar y recompilar:

```bash
make clean
make build
```

---

## Uso de la CLI

### Comprimir (imagen → .bg)

```bash
bitgrain -i foto.jpg -o comprimido.bg
bitgrain foto.png                    # escribe foto.bg
bitgrain -i entrada.bmp -o out.bg -y # -y sobrescribe si existe
```

### Descomprimir (.bg → imagen PGM)

```bash
bitgrain -d -i comprimido.bg -o reconstruida.pgm
bitgrain -d comprimido.bg           # escribe comprimido.pgm
```

### Opciones

| Opción | Descripción |
|--------|-------------|
| `-i <archivo>` | Archivo de entrada (imagen o .bg según modo) |
| `-o <archivo>` | Archivo de salida |
| `-d` | Modo descomprimir (.bg → PGM) |
| `-y` | Sobrescribir salida sin preguntar |
| `-h` | Ayuda |

**Entrada al comprimir:** JPEG, PNG, BMP, PGM, TGA, etc. (vía [stb_image](https://github.com/nothings/stb)). Si la imagen es en color, se convierte automáticamente a escala de grises.

**Salida al comprimir:** flujo `.bg` (con cabecera).  
**Salida al descomprimir:** imagen PGM (escala de grises, 8 bpp).

---

## Cómo funciona

### Pipeline de compresión (encode)

1. **Entrada:** imagen en escala de grises en memoria (`width×height` bytes, 1 byte por píxel).
2. **Bloques 8×8:** la imagen se parte en bloques de 8×8 píxeles (orden fila a fila, de izquierda a derecha). Cada píxel se centra restando 128.
3. **DCT (transformada discreta del coseno):** cada bloque 8×8 se transforma al dominio de frecuencias. Los coeficientes de baja frecuencia concentran la mayor parte de la energía.
4. **Cuantización:** cada coeficiente se divide por el valor correspondiente de una tabla de cuantización fija (tabla luminancia JPEG, calidad ~50). Se pierde precisión y muchos coeficientes pasan a cero.
5. **Entropía (RLE):** por cada bloque se escribe el coeficiente DC (2 bytes) y luego los AC como pares (run de ceros, nivel) hasta un marcador EOB. Eso reduce mucho el tamaño cuando hay muchas rachas de ceros.
6. **Cabecera .bg:** al inicio del flujo se escribe magic `"BG\x01"` y las dimensiones (width, height) para que el decoder sepa cómo reconstruir la imagen.

Resultado: un flujo `.bg` que suele ser bastante más pequeño que la imagen en crudo (sobre todo en imágenes sin comprimir o poco comprimidas).

### Pipeline de descompresión (decode)

1. **Cabecera:** se leen los 11 bytes iniciales (magic + width + height). Si el magic no es `"BG\x01"`, se devuelve error.
2. **Por cada bloque 8×8:** se lee DC (2 bytes), luego pares (run, level) hasta EOB y se reconstruye el bloque de 64 coeficientes.
3. **Dequantización:** cada coeficiente se multiplica por el mismo valor de la tabla de cuantización (operación inversa al encoder).
4. **IDCT:** transformada inversa; se pasa del dominio de frecuencias al espacial (valores centrados en 0).
5. **Píxeles:** se suma 128 a cada valor y se recorta a [0, 255]. Los bloques se reensamblan en orden para formar la imagen.

La imagen reconstruida es **visualmente la misma** que la original, con la pérdida típica de un esquema tipo JPEG (solo cuantización, sin Huffman en este prototipo).

---

## Formato .bg

- **Cabecera (11 bytes):**
  - `0x42 0x47 0x01` — magic "BG" + versión 1
  - 4 bytes: `width` (uint32, little-endian)
  - 4 bytes: `height` (uint32, little-endian)
- **Datos:** secuencia de bloques en orden de escaneo (por filas de bloques). Para cada bloque:
  - DC: 2 bytes (int16 LE).
  - AC: secuencia de tripletes (run: 1 byte, level: 2 bytes int16 LE). Run = número de ceros antes del nivel. **EOB:** run=0xFF, level=0 (2 bytes a cero).

Las dimensiones de la imagen deben ser múltiplos de 8. Los archivos `.bg` generados por versiones antiguas del encoder (sin cabecera) no son compatibles con el decoder actual.

---

## Uso como biblioteca (API C)

Bitgrain se puede usar como backend en pipelines de cámara (teléfonos, tablets, IoT): tras la captura se llama al encoder para guardar en `.bg`; cuando el usuario abre la foto, se llama al decoder y se obtienen píxeles listos para **mostrar en pantalla** o para exportar a JPEG/PNG. Basta con enlazar la estática de Rust y usar la API declarada en `includes/encoder.h`.

### Comprimir

```c
#include "encoder.h"

uint8_t *pixels = ...;  /* width*height, escala de grises */
uint8_t out_buf[OUT_CAPACITY];
int32_t out_len;

if (bitgrain_encode_grayscale(pixels, width, height, out_buf, OUT_CAPACITY, &out_len) == 0) {
    /* out_buf[0 .. out_len-1] es el flujo .bg */
}
```

### Descomprimir

```c
#include "encoder.h"

uint8_t *bg_buf = ...;   /* flujo .bg completo */
int32_t bg_size = ...;
uint8_t pixels[MAX_PIXELS];
uint32_t width, height;

if (bitgrain_decode_grayscale(bg_buf, bg_size, pixels, MAX_PIXELS, &width, &height) == 0) {
    /* pixels[0 .. width*height-1] es la imagen en escala de grises */
}
```

Compilar y enlazar con la estática generada en `rust/target/release/libbitgrain.a` y con `-lpthread -ldl -lm` (y los objetos C del proyecto: quant, bitstream, image_loader, etc.).

---

## Estructura del proyecto

```
bitgrain/
├── main.c              # CLI (encode/decode, -i -o -d -y -h)
├── Makefile            # build: Rust + C, enlace a libbitgrain.a
├── includes/
│   └── encoder.h       # API C (encode/decode)
├── c/
│   ├── quant.c/h       # cuantización por bloque (división)
│   ├── bitstream.c/h   # escritura byte a byte al buffer
│   ├── image_loader.c/h  # carga JPEG/PNG/BMP/etc. → gris (stb_image)
│   └── stb_image.h     # biblioteca de carga de imágenes
└── rust/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── ffi.rs          # bitgrain_encode_grayscale, bitgrain_decode_grayscale (FFI C)
        ├── encoder.rs      # cabecera .bg, pipeline encode, tabla de cuantización
        ├── decoder.rs      # lectura .bg, RLE → dequant → IDCT → imagen
        ├── block.rs        # bloque 8×8 (64 coeficientes i16)
        ├── blockizer.rs    # imagen → lista de bloques 8×8 (centrado -128)
        ├── dct.rs          # DCT e IDCT 8×8
        ├── entropy.rs      # RLE por bloque (DC + pares run/level + EOB)
        └── bitstream.rs    # wrapper a bitstream_write_byte (C)
```

La parte “pesada” (DCT, cuantización lógica, RLE, decoder) está en Rust; la cuantización por muestra y el bitstream básico están en C y se llaman desde Rust vía FFI. El CLI está en C y usa la API de `encoder.h` y el cargador de imágenes en C.

---

## Limitaciones y notas

- **Escala de grises:** solo se codifica un canal (luminancia). Las imágenes en color se convierten a gris al cargar.
- **Dimensiones:** se asume que width y height son múltiplos de 8. Imágenes con otras dimensiones pueden no manejarse correctamente en todos los caminos.
- **Pérdida:** la compresión es lossy (cuantización). La imagen decodificada no es bit a bit igual a la original.
- **Formato .bg:** es propio del proyecto; no es JPEG estándar. La salida al decodificar es PGM; para otros formatos habría que añadir escritura PNG/JPEG aparte.
- **Compatibilidad:** solo los archivos `.bg` que incluyen la cabecera de 11 bytes (magic + width + height) son decodificables con la versión actual.

---

## Licencia

GPL-3.0-or-later (ver cabeceras de los archivos fuente).
