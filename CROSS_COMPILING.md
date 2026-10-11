# Cross-compiling DagomApp

This builds the Windows and macOS apps from Linux. Set up a native build
first ([HACKING.md](HACKING.md)): cross builds also build the frontend with
the Node.js installed in `target/`, which runs on the build machine.

## Windows

Windows builds use the MinGW-w64 toolchain (`x86_64-pc-windows-gnu` target),
which needs no Microsoft SDK. Only 64-bit x86 Windows is supported.

1. Add the Rust target:

       rustup target add x86_64-pc-windows-gnu

2. Install MinGW-w64. The linker is set in `.cargo/config.toml` to
   `x86_64-w64-mingw32-gcc`, which these packages provide:

   - Debian/Ubuntu: `sudo apt install gcc-mingw-w64-x86-64`
   - Fedora: `sudo dnf install mingw64-gcc`
   - Arch: `sudo pacman -S mingw-w64-gcc`
   - macOS (Homebrew): `brew install mingw-w64`

3. Build:

       cargo build --release --target x86_64-pc-windows-gnu

The app is `target/x86_64-pc-windows-gnu/release/dagomapp.exe`. Unlike MSVC
builds, MinGW builds load `WebView2Loader.dll` at run time: copy
`target/x86_64-pc-windows-gnu/release/WebView2Loader.dll` next to the `.exe`.

The app also needs the Microsoft Edge WebView2 Runtime, which comes with
Windows 11 and up to date Windows 10 installs.

## macOS

macOS builds use [osxcross](https://github.com/tpoechtrager/osxcross), which
needs the macOS SDK from Xcode. Apple's Xcode license only allows using it on
Apple hardware: check that your use complies before setting this up.

1. Add the Rust targets, for Intel and Apple Silicon Macs:

       rustup target add x86_64-apple-darwin aarch64-apple-darwin

2. Package the SDK on a Mac with Xcode installed, following
   [osxcross' instructions](https://github.com/tpoechtrager/osxcross#packaging-the-sdk),
   e.g. with `./tools/gen_sdk_package.sh` from an osxcross checkout. Copy the
   resulting `MacOSX<version>.sdk.tar.xz` to the Linux machine. Apple Silicon
   builds need the macOS 11 SDK or later.

3. Build osxcross on Linux. It needs clang, LLVM (for `llvm-ar`), cmake,
   patch, libssl, lzma, libxml2 and zlib development files; on Debian/Ubuntu:

       sudo apt install clang llvm lld cmake make git patch python3 libssl-dev liblzma-dev libxml2-dev libbz2-dev zlib1g-dev uuid-dev bzip2 cpio
       git clone https://github.com/tpoechtrager/osxcross
       cp MacOSX<version>.sdk.tar.xz osxcross/tarballs/
       (cd osxcross && UNATTENDED=1 ./build.sh)

4. Put osxcross on the `PATH`, and tell build tools where the SDK is, giving
   the path of your osxcross checkout:

       export PATH="/path/to/osxcross/target/bin:$PATH"
       export SDKROOT="/path/to/osxcross/target/SDK/MacOSX<version>.sdk"

5. Build from the DagomApp checkout, using the osxcross settings from
   `.cargo/osxcross.toml` (they aren't in `.cargo/config.toml` as they would
   break native macOS builds):

       cargo build --release --config .cargo/osxcross.toml --target x86_64-apple-darwin
       cargo build --release --config .cargo/osxcross.toml --target aarch64-apple-darwin

The apps are `target/<target>/release/dagomapp`: plain executables, not `.app`
bundles, which Tauri only makes on macOS. Apple Silicon Macs refuse to run
unsigned executables: if yours isn't ad-hoc signed by the osxcross linker, sign
it with [rcodesign](https://github.com/indygreg/apple-platform-rs/tree/main/apple-codesign):

    cargo install apple-codesign
    rcodesign sign target/aarch64-apple-darwin/release/dagomapp
