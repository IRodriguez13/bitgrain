# Bitgrain Go bindings

Build the Bitgrain library from the **repo root**:

```bash
cd /path/to/bitgrain
make install PREFIX=$HOME/.local
```

The bindings use `pkg-config` (`bitgrain.pc`). From this directory:

```bash
cd /path/to/bitgrain/bindings/go
PKG_CONFIG_PATH=$HOME/.local/lib/pkgconfig CGO_ENABLED=1 go build
```

If installed under `/usr/local`:

```bash
PKG_CONFIG_PATH=/usr/local/lib/pkgconfig CGO_ENABLED=1 go build
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
