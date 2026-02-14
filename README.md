# Bitgrain

**v1.0.0** — Compresor de imágenes tipo JPEG: toma una imagen y devuelve la misma con menos peso (formato `.bg`). Escala de grises y RGB. Incluye encoder y decoder en una misma API y CLI, estilo FFmpeg.

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

### Descomprimir (.bg → imagen)

```bash
bitgrain -d -i comprimido.bg -o reconstruida.jpg
bitgrain -d comprimido.bg           # escribe comprimido.jpg
```

### Opciones

| Opción | Descripción |
|--------|-------------|
| `-i <archivo>` | Archivo de entrada (imagen o .bg según modo) |
| `-o <archivo>` | Salida (formato por extensión: .jpg .png .pgm) |
| `-d` | Modo descomprimir (.bg → imagen) |
| `-cd` | Round-trip: comprimir + descomprimir en memoria |
| `-q <1-100>` | Calidad de compresión (default 85) |
| `-y` | Sobrescribir salida sin preguntar |
| `-h` | Ayuda |

**Entrada al comprimir:** JPEG, PNG, BMP, PGM, TGA, etc. (vía [stb_image](https://github.com/nothings/stb)). Color se codifica en RGB (3 planos).

**Salida al comprimir:** flujo `.bg` (cabecera 12 bytes + datos).  
**Salida al descomprimir / round-trip:** .jpg, .png o .pgm (por extensión de `-o`).

---

## Cómo funciona

### Pipeline de compresión (encode)

1. **Entrada:** imagen en escala de grises en memoria (`width×height` bytes, 1 byte por píxel).
2. **Bloques 8×8:** la imagen se parte en bloques de 8×8 píxeles (orden fila a fila, de izquierda a derecha). Cada píxel se centra restando 128.
3. **DCT (transformada discreta del coseno):** cada bloque 8×8 se transforma al dominio de frecuencias. Los coeficientes de baja frecuencia concentran la mayor parte de la energía.
4. **Cuantización:** cada coeficiente se divide por la tabla escalada por calidad (`-q`, default 85). Se pierde precisión y muchos coeficientes pasan a cero.
5. **Entropía (RLE):** por cada bloque se escribe DC (2 bytes) y luego los AC en **orden zigzag** (JPEG) como pares (run, level) hasta EOB. Reduce el tamaño cuando hay muchas rachas de ceros.
6. **Cabecera .bg:** magic `"BG"` + versión (1=gris, 2=RGB) + width + height (LE) + quality (12 bytes).

Resultado: un flujo `.bg` que suele ser bastante más pequeño que la imagen en crudo (sobre todo en imágenes sin comprimir o poco comprimidas).

### Pipeline de descompresión (decode)

1. **Cabecera:** se leen 12 bytes (magic + versión + width + height + quality). Si el magic no es `"BG"`, error.
2. **Por cada bloque 8×8:** se lee DC (2 bytes), luego pares (run, level) en orden zigzag hasta EOB y se reconstruye el bloque de 64 coeficientes.
3. **Dequantización:** cada coeficiente se multiplica por el mismo valor de la tabla de cuantización (operación inversa al encoder).
4. **IDCT:** transformada inversa; se pasa del dominio de frecuencias al espacial (valores centrados en 0).
5. **Píxeles:** se suma 128 a cada valor y se recorta a [0, 255]. Los bloques se reensamblan en orden para formar la imagen.

La imagen reconstruida es **visualmente la misma** que la original, con la pérdida típica de un esquema tipo JPEG (solo cuantización, sin Huffman en este prototipo).

---

## Formato .bg (v1.0)

- **Cabecera (12 bytes):**
  - `0x42 0x47` — magic "BG"
  - 1 byte: versión (1 = escala de grises, 2 = RGB)
  - 4 bytes: `width` (uint32, little-endian)
  - 4 bytes: `height` (uint32, little-endian)
  - 1 byte: `quality` (1–100; 0 = 50)
- **Datos:** secuencia de bloques en orden de escaneo. Para cada bloque:
  - DC: 2 bytes (int16 LE).
  - AC en orden zigzag: tripletes (run: 1 byte, level: 2 bytes int16 LE). **EOB:** run=0xFF, level=0.

Las dimensiones pueden ser cualesquiera (no es necesario múltiplo de 8); los bloques se truncan en los bordes.

---

## Uso como biblioteca (API C)

Bitgrain se puede usar como backend en pipelines de cámara (teléfonos, tablets, IoT): tras la captura se llama al encoder para guardar en `.bg`; cuando el usuario abre la foto, se llama al decoder y se obtienen píxeles listos para **mostrar en pantalla** o para exportar a JPEG/PNG. Basta con enlazar la estática de Rust y usar la API declarada en `includes/encoder.h`.

### Comprimir

```c
#include "encoder.h"

uint8_t *pixels = ...;  /* width*height, escala de grises */
uint8_t out_buf[OUT_CAPACITY];
int32_t out_len;

if (bitgrain_encode_grayscale(pixels, width, height, out_buf, OUT_CAPACITY, &out_len, 85) == 0) {
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

Compilar y enlazar con `rust/target/release/libbitgrain.a` y los objetos C (quant, image_loader, image_writer, main.c) y con `-lpthread -ldl -lm`.

---

## Estructura del proyecto

```
bitgrain/
├── main.c              # CLI (encode/decode, -i -o -d -y -h)
├── Makefile            # build: Rust + C, enlace a libbitgrain.a
├── includes/
│   └── encoder.h       # API C (encode/decode)
├── c/
│   ├── quant.c/h         # cuantización por bloque (FFI desde Rust)
│   ├── image_loader.c/h  # carga JPEG/PNG/BMP/etc. (stb_image)
│   ├── image_writer.c/h  # escritura JPG/PNG (stb_image_write)
│   └── stb_image.h       # (stb_image_write en image_writer.c)
└── rust/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── ffi.rs          # FFI C: encode/decode con quality
        ├── encoder.rs      # cabecera .bg, pipeline encode, calidad
        ├── decoder.rs      # .bg → RLE → dequant → IDCT (paralelo)
        ├── block.rs, blockizer.rs, dct.rs, entropy.rs, bitstream.rs, zigzag.rs
```

La parte “pesada” (DCT, RLE, decode, paralelismo) está en Rust; la cuantización por muestra se hace en C vía FFI. El CLI está en C y usa la API de `encoder.h` y el cargador/escritor de imágenes.

---

## Limitaciones y notas

- **Pérdida:** la compresión es lossy (cuantización). La imagen decodificada no es bit a bit igual a la original.
- **Formato .bg:** propio del proyecto (no es JPEG estándar). Salida al decodificar: .jpg, .png o .pgm según extensión de `-o`.
- **Tests:** los tests de Rust (`cargo test`) usan solo datos sintéticos en memoria y **no se incluyen en el binario de release**; no afectan a lo que el usuario comprime ni decodifica.

---

## Licencia

GPL-3.0-or-later (ver cabeceras de los archivos fuente).
