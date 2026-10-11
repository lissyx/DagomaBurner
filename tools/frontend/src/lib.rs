//! Runs `scripts/frontend.mjs` (npm install + Vite) with a Node.js downloaded
//! into `target/node/`, so building the app only needs Rust.
//!
//! Set `DAGOMAPP_NODE` to the path of a `node` binary to use it instead, e.g.
//! for offline builds.

use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

const NODE_VERSION: &str = "v24.21.0";

/// Node.js release archives and their SHA-256, from
/// https://nodejs.org/dist/v24.21.0/SHASUMS256.txt.
const NODE_RELEASES: &[(&str, &str, &str)] = &[
    // (os, arch, sha256)
    (
        "linux",
        "x86_64",
        "6e1db87ef58b8819e5d5402eff1536491b18edd8eb7bee5ef7897876e88dc5ff",
    ),
    (
        "linux",
        "aarch64",
        "724282c3b43aec998aa9527380465b45d229e021b58035f5f4f63095eabfe5d5",
    ),
    (
        "macos",
        "x86_64",
        "1462cb3b3046b815cf8ea436d3da450ec1a9f11dac7e5a46b0ada5305d7e8097",
    ),
    (
        "macos",
        "aarch64",
        "bed7eea5325e1108f32ce5228ddd6a5f0f08a499ee42aa7442aea583702f6057",
    ),
    (
        "windows",
        "x86_64",
        "158f7685b44de51f6c0df1d153526cbcd3e1bc739a8dfc607721cef75de9e541",
    ),
    (
        "windows",
        "aarch64",
        "8779b1bde1d39f8d420e3b57aa657b39891af434d3de44a919044cec06785921",
    ),
];

type Result<T> = std::result::Result<T, String>;

/// The release archive name, without extension, for the machine running this.
fn release_name() -> Result<(String, &'static str)> {
    let (os, arch) = (env::consts::OS, env::consts::ARCH);
    let sha256 = NODE_RELEASES
        .iter()
        .find(|(o, a, _)| *o == os && *a == arch)
        .map(|(_, _, sha256)| *sha256)
        .ok_or_else(|| {
            format!("no Node.js download for {os}/{arch}, set DAGOMAPP_NODE to a node binary")
        })?;
    let os = match os {
        "macos" => "darwin",
        "windows" => "win",
        other => other,
    };
    let arch = if arch == "aarch64" { "arm64" } else { "x64" };
    Ok((format!("node-{NODE_VERSION}-{os}-{arch}"), sha256))
}

fn node_binary(install_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        install_dir.join("node.exe")
    } else {
        install_dir.join("bin").join("node")
    }
}

/// Downloads `url` to `dest`, checking its SHA-256.
fn download(url: &str, dest: &Path, sha256: &str) -> Result<()> {
    eprintln!("Downloading {url}");
    let mut body = ureq::get(url)
        .call()
        .map_err(|e| format!("can't download {url}: {e}"))?
        .into_body()
        .into_reader();
    let mut file = File::create(dest).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0; 64 * 1024];
    loop {
        let n = body
            .read(&mut buf)
            .map_err(|e| format!("can't download {url}: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
    }

    let actual: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if actual != sha256 {
        return Err(format!(
            "checksum mismatch for {url}: got {actual}, expected {sha256}"
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn extract(archive: &Path, dest: &Path) -> io::Result<()> {
    let gz = flate2::read::GzDecoder::new(File::open(archive)?);
    tar::Archive::new(gz).unpack(dest)
}

#[cfg(windows)]
fn extract(archive: &Path, dest: &Path) -> io::Result<()> {
    zip::ZipArchive::new(File::open(archive)?)
        .and_then(|mut zip| zip.extract(dest))
        .map_err(io::Error::other)
}

/// Returns the `node` binary to use, installing Node.js into
/// `<root>/target/node/` first if needed.
pub fn node(root: &Path) -> Result<PathBuf> {
    if let Some(node) = env::var_os("DAGOMAPP_NODE") {
        return Ok(node.into());
    }

    let (name, sha256) = release_name()?;
    let node_dir = root.join("target").join("node");
    let install_dir = node_dir.join(&name);
    let node = node_binary(&install_dir);
    if node.exists() {
        return Ok(node);
    }

    // Download and extract next to the final location, then move it in place,
    // so an interrupted install is never mistaken for a complete one.
    let staging = node_dir.join(format!("staging-{}", std::process::id()));
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let result = (|| {
        let extension = if cfg!(windows) { "zip" } else { "tar.gz" };
        let archive = staging.join(format!("{name}.{extension}"));
        download(
            &format!("https://nodejs.org/dist/{NODE_VERSION}/{name}.{extension}"),
            &archive,
            sha256,
        )?;
        extract(&archive, &staging).map_err(|e| format!("can't extract {name}: {e}"))?;
        match fs::rename(staging.join(&name), &install_dir) {
            // Another build installed it meanwhile.
            Err(_) if node.exists() => Ok(()),
            other => other.map_err(|e| format!("can't install {name}: {e}")),
        }
    })();
    let _ = fs::remove_dir_all(&staging);
    result.map(|()| node)
}

/// Runs `scripts/frontend.mjs <command>` from the repository at `root`.
pub fn command(root: &Path, command: &str) -> Result<Command> {
    let node = node(root)?;
    let mut cmd = Command::new(&node);
    cmd.arg(root.join("scripts").join("frontend.mjs"))
        .arg(command)
        .current_dir(root);

    // Make this Node.js the one found by npm and the scripts it runs.
    if let Some(bin_dir) = node.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        let path = env::var_os("PATH").unwrap_or_default();
        let paths = std::iter::once(bin_dir.to_path_buf()).chain(env::split_paths(&path));
        cmd.env("PATH", env::join_paths(paths).map_err(|e| e.to_string())?);
    }
    Ok(cmd)
}
