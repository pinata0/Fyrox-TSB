#![windows_subsystem = "windows"]

use flate2::read::ZlibDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, Read, Seek, SeekFrom, Write},
    os::windows::process::CommandExt,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

const MAGIC: &[u8; 16] = b"TSB-ONEFILE-v1\0\0";
const FOOTER_SIZE: u64 = 56;

#[derive(Deserialize)]
struct Entry {
    path: String,
    bytes: u64,
    sha256: String,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn digest(reader: &mut impl Read) -> io::Result<String> {
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn safe_path(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(['\\', ':'])
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && Path::new(name)
            .components()
            .all(|p| matches!(p, Component::Normal(_)))
}

fn cache_valid(root: &Path, id: &str, entries: &[Entry], verify: bool) -> bool {
    fs::read_to_string(root.join(".ready")).ok().as_deref() == Some(id)
        && entries.iter().all(|entry| {
            let path = root.join(&entry.path);
            // Fyrox rewrites this generated index on normal shutdown.
            if entry.path == "data/resources.registry" {
                return path.is_file();
            }
            fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() == entry.bytes)
                && (!verify
                    || File::open(path)
                        .and_then(|mut f| digest(&mut f))
                        .ok()
                        .as_deref()
                        == Some(entry.sha256.as_str()))
        })
}

fn unpack(verify: bool) -> io::Result<PathBuf> {
    let mut source = File::open(env::current_exe()?)?;
    let length = source.metadata()?.len();
    if length < FOOTER_SIZE {
        return Err(invalid("The single-file package is incomplete."));
    }
    source.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
    let mut footer = [0u8; FOOTER_SIZE as usize];
    source.read_exact(&mut footer)?;
    if &footer[..16] != MAGIC {
        return Err(invalid("The single-file package footer is missing."));
    }
    let size = u64::from_le_bytes(footer[16..24].try_into().unwrap());
    let offset = length
        .checked_sub(FOOTER_SIZE)
        .and_then(|remaining| remaining.checked_sub(size))
        .ok_or_else(|| invalid("Invalid package length."))?;
    let id: String = footer[24..]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    source.seek(SeekFrom::Start(offset))?;
    if digest(&mut Read::by_ref(&mut source).take(size))? != id {
        return Err(invalid("Package checksum failed. Copy the EXE again."));
    }
    source.seek(SeekFrom::Start(offset))?;
    let mut payload = BufReader::new(ZlibDecoder::new(source.take(size)));
    let mut prefix = [0u8; 4];
    payload.read_exact(&mut prefix)?;
    let manifest_size = u32::from_le_bytes(prefix) as usize;
    if manifest_size > 4 * 1024 * 1024 {
        return Err(invalid("Invalid package manifest length."));
    }
    let mut manifest = vec![0; manifest_size];
    payload.read_exact(&mut manifest)?;
    let entries: Vec<Entry> = serde_json::from_slice(&manifest)?;
    if entries.is_empty()
        || entries.len() > 20000
        || entries
            .iter()
            .any(|e| !safe_path(&e.path) || e.bytes > 4 * 1024 * 1024 * 1024)
    {
        return Err(invalid("Invalid package file list."));
    }
    let base = match env::var_os("TSB_CACHE_DIR") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(
            env::var_os("LOCALAPPDATA").ok_or_else(|| invalid("LOCALAPPDATA is unavailable."))?,
        )
        .join("TSB/SingleFile"),
    };
    fs::create_dir_all(&base)?;
    let base = fs::canonicalize(base)?;
    let destination = base.join(&id);
    if cache_valid(&destination, &id, &entries, verify) {
        return Ok(destination);
    }
    for entry in fs::read_dir(&base)? {
        let entry = entry?;
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(&format!("{id}-"))
            && entry.file_type()?.is_dir()
            && cache_valid(&entry.path(), &id, &entries, verify)
        {
            return Ok(entry.path());
        }
    }
    // Never overwrite a running build or expose a partially extracted build.
    let pending = base.join(format!("{id}-{}", std::process::id()));
    fs::create_dir(&pending)?;
    for entry in &entries {
        let path = pending.join(&entry.path);
        fs::create_dir_all(path.parent().unwrap())?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        let written = io::copy(
            &mut Read::by_ref(&mut payload).take(entry.bytes),
            &mut output,
        )?;
        if written != entry.bytes || digest(&mut File::open(&path)?)? != entry.sha256 {
            return Err(invalid(format!(
                "Extraction checksum failed: {}",
                entry.path
            )));
        }
    }
    if payload.read(&mut [0u8; 1])? != 0 {
        return Err(invalid("Unexpected trailing package data."));
    }
    fs::create_dir_all(pending.join("data/characters"))?;
    fs::write(pending.join(".ready"), &id)?;
    if !destination.exists() && fs::rename(&pending, &destination).is_ok() {
        Ok(destination)
    } else {
        // Simultaneous starts or a damaged old cache can use this complete copy.
        Ok(pending)
    }
}

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    let mut args: Vec<OsString> = env::args_os().skip(1).collect();
    let verify = args.first().is_some_and(|a| a == "--verify-only");
    let root = unpack(verify)?;
    println!("TSB runtime: {}", root.display());
    if verify || args.first().is_some_and(|a| a == "--extract-only") {
        println!("PASS: package extracted and verified.");
        return Ok(0);
    }
    let executable = match args.first().and_then(|a| a.to_str()) {
        Some("--check-eos") => {
            args.remove(0);
            if !args.iter().any(|a| a == "--offline" || a == "--config") {
                args.extend(["--config".into(), root.join("eos.local.json").into()]);
            }
            "tsb-eos-check.exe"
        }
        Some("--check-relay-host") => {
            args.remove(0);
            args.extend(["--host".into(), "--force-relay".into()]);
            "tsb-eos-peer-check.exe"
        }
        Some("--check-relay-join") => {
            args[0] = "--join".into();
            args.push("--force-relay".into());
            "tsb-eos-peer-check.exe"
        }
        _ => "tsb.exe",
    };
    // Preserve caller-relative session/config/output paths after changing cwd.
    let caller = env::current_dir()?;
    for index in 0..args.len().saturating_sub(1) {
        if [
            "--config",
            "--eos-config",
            "--online-session",
            "--code-file",
        ]
        .iter()
        .any(|flag| args[index] == *flag)
        {
            let path = PathBuf::from(&args[index + 1]);
            if path.is_relative() {
                args[index + 1] = caller.join(path).into();
            }
        }
    }
    let logs = root.join("artifacts/logs");
    fs::create_dir_all(&logs)?;
    let log_path = logs.join(format!("launcher-{}.log", std::process::id()));
    let mut log = File::create(&log_path)?;
    writeln!(log, "TSB single-file release\nRuntime: {}", root.display())?;
    println!("TSB log: {}", log_path.display());
    let status = Command::new(root.join(executable))
        .args(args)
        .current_dir(&root)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW; the game still opens its GUI.
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .status()?;
    if !status.success() {
        return Err(format!("TSB exited with {status}.\nLog: {}", log_path.display()).into());
    }
    Ok(0)
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("TSB: {error}");
            // Double-click failures remain readable; CLI diagnostics never block on UI.
            if env::args_os().len() == 1 {
                #[link(name = "user32")]
                unsafe extern "system" {
                    fn MessageBoxW(
                        window: *mut std::ffi::c_void,
                        text: *const u16,
                        title: *const u16,
                        kind: u32,
                    ) -> i32;
                }
                let message: Vec<u16> = format!("게임을 시작하지 못했습니다.\n\n{error}\0")
                    .encode_utf16()
                    .collect();
                let title: Vec<u16> = "Trickcal : Shooting Balddagu\0".encode_utf16().collect();
                unsafe {
                    MessageBoxW(std::ptr::null_mut(), message.as_ptr(), title.as_ptr(), 0x10);
                }
            }
            std::process::exit(1);
        }
    }
}
