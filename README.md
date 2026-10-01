# 🏯 Mikage Shrine Raytracer

A beautiful voxel-based raytraced diorama with two distinct worlds to explore: a serene golden-hour shrine and a vibrant nighttime spirit district.

## ✨ What You'll Experience

### Shrine World 🏯
Golden hour lighting over a Japanese shrine with:
- Beautiful sakura trees with falling petals
- Tranquil koi pond with 9 fish
- Traditional architecture with pastel colors
- Stone lanterns lighting pathways
- Guardian statues and water basins
- Serene town atmosphere

### Spirit World 🌙
A red-light district pleasure quarter featuring:
- Brilliant central lantern tower (focal point)
- 14+ teahouses with glowing windows
- Dense nighttime atmosphere with cool purples
- Multiple bridges and canals
- Dynamic point lighting
- Portal gate for world travel

## 🎮 How to Play

### Start the Game
```bash
cargo run --release
```

### Controls
| Key | Action |
|-----|--------|
| **Click** | Capture mouse / aim crosshair |
| **WASD** | Move forward/left/right/backward |
| **Space / Shift** | Move up / down |
| **Scroll wheel** | Zoom in/out |
| **Q / E** | Rotate the scene view |
| **R** | Reset to starting position |
| **F12** | Take high-quality screenshot |
| **Esc** | Release mouse (press again to quit) |

### Portal Travel
When you see a **glowing portal**:
1. Aim your crosshair at it
2. Click to travel between worlds
3. Enjoy a smooth fade transition

## 📊 Performance

- **FPS:** 50-60 (smooth and responsive)
- **Build time:** ~1.6 seconds
- **Rendering:** Real-time raytracing
- **World size:** 96×96 blocks per world

## 🎨 Technical Details

- **Engine:** Custom Rust raytracer with DDA algorithm
- **Graphics:** Full raytracing with:
  - Realistic light reflection & refraction
  - Dynamic point lighting
  - Material-based surface properties
  - Hierarchical block system (1x1 and mini-blocks)

## 🚀 Getting Started

1. Make sure you have Rust installed
2. Clone/download this project
3. Run: `cargo run --release`
4. Click to capture mouse and look around
5. Explore both worlds and find the portal!

## 🌸 Features

- **Two unique worlds** with distinct atmospheres
- **Real-time performance** at 50-60 FPS
- **Beautiful pastel colors** with full raytracing
- **Dynamic animation** (falling petals, swimming koi)
- **Portal travel** between worlds
- **Scalable rendering** (screenshot at higher resolution with F12)

## 💡 Tips

- Start in the shrine world to get oriented
- Use Q/E to rotate the scene for better views
- Look for the cave entrance with glowing lanterns
- Follow the stone path to the portal
- The spirit world portal is in the building at the back

Enjoy your visit to Mikage Shrine! 🏯✨

---

**Built with:** Rust + Custom Raytracer
**License:** Personal Project
