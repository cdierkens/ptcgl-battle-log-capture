# Windows 11 ARM64 on UTM (Apple Silicon)

## Prerequisites
- macOS on Apple Silicon (M1/M2/M3/M4)
- UTM installed (`brew install --cask utm` or from [utm.app](https://mac.getutm.app/))
- Windows 11 ARM64 ISO (free from Microsoft)

## Step 1: Get Windows 11 ARM64 ISO
1. Go to https://www.microsoft.com/software-download/windows11
2. Scroll to "Download Windows 11 Disk Image (ISO) for ARM64"
3. Select "Windows 11 (multi-edition ISO for ARM64)" → Download
4. Or use the direct link (changes periodically): search "Windows 11 ARM64 ISO download"

## Step 2: Create VM in UTM
1. Open UTM → **+** → **Virtualize** (not Emulate)
2. **Operating System**: Windows 11
3. **ISO Image**: Select the downloaded ISO
4. **Hardware**: 
   - Memory: 8 GB (minimum 4 GB, 8+ recommended for PTCG Live)
   - CPU Cores: 4 (or half your physical cores)
   - **Graphics**: VirtIO-GPU (Virgl) — enables 3D acceleration
   - **Rosetta**: ✅ Enable (speeds up x64 emulation significantly)
5. **Storage**: 64 GB dynamic (PTCG Live + OS ~25 GB)
6. **Shared Directory** (optional): Add a folder for file exchange

## Step 3: Install Windows
1. Start VM → Follow Windows setup
2. **Skip network** if prompted (OOBE), or use `Shift+F10` → `OOBE\BYPASSNRO`
3. Create local account (no Microsoft account needed)
4. **Install UTM Guest Tools** (from UTM menu → CD/DVD → "Install Guest Tools")
   - Enables clipboard sharing, file drag-drop, dynamic resolution

## Step 4: Post-Install
```powershell
# In Windows terminal (Admin):
# Enable Developer Mode (for sideloading Tauri apps)
Settings → Privacy & Security → For developers → Developer Mode: On

# Install Microsoft Visual C++ Redistributable (Tauri needs it)
winget install Microsoft.VCRedist.2015+.x64

# Install PTCG Live
# Option A: Microsoft Store (ARM64 native)
# Option B: winget install Pokemon.PokemonTCGLive
```

## Step 5: Test the App
```powershell
# Copy the built .exe or .msi to VM (shared folder or scp)
# Run the .exe — should launch Tauri window
# Grant any permissions prompted
```

## Known Issues on ARM64 Windows
| Issue | Workaround |
|-------|------------|
| PTCG Live runs under x64 emulation | Expect 2-3x slower; UI still usable |
| Tauri `webview2` runtime | Install via `winget install Microsoft.EdgeWebView2Runtime` |
| Screen capture (Graphics Capture API) | Works in VM but may need "Allow desktop apps to access graphics" in Settings |
| Overlay transparency | May show black background; test `TauriOverlayPresenter` |

## For Development
- **Build on Mac** → copy `src-tauri/target/release/ptcgl-battle-log-capture.exe` to VM
- **Or**: Install Rust + Node in VM and build natively (slow)
- **Remote debug**: `tauri dev` on Mac won't connect; use `println!`/`tracing` logs

## CI Alternative (No VM Needed)
GitHub Actions `windows-latest` runs x64 Windows Server 2022:
- Compiles + runs `cargo test` (pure logic, no GUI)
- Can build `.msi` for release artifacts
- Add to `.github/workflows/verify.yml` (see extraction spec §6)

---
*Quick start: `brew install --cask utm` → download ISO → 15 min to working VM*
