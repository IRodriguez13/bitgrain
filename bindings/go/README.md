# Bitgrain Go bindings

Build the Bitgrain library from the **repo root**:

```bash
cd /path/to/bitgrain
make bitgrain
```

The bindings use cgo with paths relative to the repo: `includes/` and `rust/target/release/`. From this directory (bindings/go) you can build a Go program that uses the bitgrain package:

```bash
cd /path/to/bitgrain/bindings/go
CGO_ENABLED=1 go build
```

If the library is installed (e.g. `make install` with PREFIX=/usr/local), you can use:

```bash
CGO_CFLAGS="-I/usr/local/include" CGO_LDFLAGS="-L/usr/local/lib -lbitgrain -lpthread -ldl -lm -lwebp" go build
```

## Example

```go
package main

import "bitgrain"  // or your module path to bindings/go

func main() {
    rgb := loadRGB("image.png") // your loader
    bg := bitgrain.EncodeRGB(rgb, width, height, 85)
    if bg == nil { ... }
    res := bitgrain.Decode(bg)
    if res == nil { ... }
    // res.Pixels, res.Width, res.Height, res.Channels
}
```
