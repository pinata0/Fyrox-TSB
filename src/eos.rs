//! EOS SDK and P2P worker. A Device ID is transport identity, not a DotG account.
use serde::Deserialize;
use std::{
    ffi::{c_char, c_void, CStr, CString},
    fs::File,
    io::{self, Read},
    marker::PhantomData,
    path::Path,
    ptr::NonNull,
    rc::Rc,
    sync::atomic::{AtomicU8, Ordering},
    thread,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EosConfig {
    product_id: String,
    sandbox_id: String,
    deployment_id: String,
    client_id: String,
    client_secret: String,
}
impl std::fmt::Debug for EosConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EosConfig { credentials: [redacted] }")
    }
}
impl EosConfig {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut data = Vec::new();
        File::open(path)?.take(16385).read_to_end(&mut data)?;
        if data.len() > 16384 {
            return Err(invalid("EOS settings file exceeds 16 KiB"));
        }
        let value: Self = serde_json::from_slice(&data).map_err(|_| {
            invalid("Invalid EOS settings JSON; use eos.example.json as a template")
        })?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> io::Result<()> {
        for (name, value, maximum) in [
            ("product_id", &self.product_id, 64),
            ("sandbox_id", &self.sandbox_id, 64),
            ("deployment_id", &self.deployment_id, 64),
            ("client_id", &self.client_id, 256),
            ("client_secret", &self.client_secret, 512),
        ] {
            if value.is_empty()
                || value.len() > maximum
                || !value.bytes().all(|b| b.is_ascii_graphic())
            {
                // Only field names, never user-provided values, appear in errors.
                return Err(invalid(&format!(
                    "Fill in {name} in eos.local.json (no spaces or line breaks)"
                )));
            }
        }
        Ok(())
    }
    fn c_strings(&self) -> [CString; 5] {
        [
            &self.product_id,
            &self.sandbox_id,
            &self.deployment_id,
            &self.client_id,
            &self.client_secret,
        ]
        .map(|v| CString::new(v.as_bytes()).expect("validated EOS settings"))
    }
}

unsafe extern "C" {
    fn tsb_eos_initialize() -> i32;
    fn tsb_eos_shutdown();
    fn tsb_eos_version() -> *const c_char;
    fn tsb_eos_error(code: i32) -> *const c_char;
    fn tsb_eos_create(
        product: *const c_char,
        sandbox: *const c_char,
        deployment: *const c_char,
        client: *const c_char,
        secret: *const c_char,
        force_relay: i32,
        result: *mut i32,
    ) -> *mut c_void;
    fn tsb_eos_destroy(context: *mut c_void);
    fn tsb_eos_tick(context: *mut c_void);
    fn tsb_eos_login(context: *mut c_void);
    fn tsb_eos_login_result(context: *mut c_void) -> i32;
    fn tsb_eos_user(context: *mut c_void, buffer: *mut c_char, capacity: i32) -> i32;
    fn tsb_eos_query_nat(context: *mut c_void);
    fn tsb_eos_nat_result(context: *mut c_void) -> i32;
    fn tsb_eos_nat_type(context: *mut c_void) -> i32;
    fn tsb_eos_open(context: *mut c_void, socket: *const c_char, remote: *const c_char) -> i32;
    fn tsb_eos_close(context: *mut c_void);
    fn tsb_eos_send(context: *mut c_void, peer: *const c_char, data: *const u8, length: u32)
        -> i32;
    fn tsb_eos_receive(
        context: *mut c_void,
        peer: *mut c_char,
        peer_capacity: i32,
        data: *mut u8,
        capacity: u32,
        written: *mut u32,
    ) -> i32;
    fn tsb_eos_network_type(context: *mut c_void) -> i32;
    fn tsb_eos_retryable(result: i32) -> i32;
    fn tsb_eos_random(output: *mut c_char) -> i32;
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn check(result: i32, operation: &str) -> io::Result<()> {
    if result == 0 {
        Ok(())
    } else {
        // EOS supplies a static, NUL-terminated error-name string for result codes.
        let name = unsafe { CStr::from_ptr(tsb_eos_error(result)) }.to_string_lossy();
        Err(io::Error::other(format!("{operation}: {name}")))
    }
}

// EOS forbids further SDK calls after shutdown in the same process.
// 0 = never initialized, 1 = owned, 2 = shut down.
static LIFECYCLE: AtomicU8 = AtomicU8::new(0);

/// Owns process-wide initialization. EOS platform work stays on its creating thread.
pub struct Sdk(PhantomData<Rc<()>>);
impl Sdk {
    pub fn initialize() -> io::Result<Self> {
        LIFECYCLE
            .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|state| {
                io::Error::other(if state == 2 {
                    "EOS SDK was shut down; restart the process before initializing again"
                } else {
                    "EOS SDK is already owned by this process"
                })
            })?;
        // No platform or callbacks exist before this sole initialization call.
        if let Err(error) = check(unsafe { tsb_eos_initialize() }, "EOS initialize") {
            LIFECYCLE.store(0, Ordering::SeqCst);
            return Err(error);
        }
        Ok(Self(PhantomData))
    }
    pub fn version(&self) -> String {
        // SDK version is a static NUL-terminated string while initialized.
        unsafe { CStr::from_ptr(tsb_eos_version()) }
            .to_string_lossy()
            .into_owned()
    }
}
impl Drop for Sdk {
    fn drop(&mut self) {
        // EosClient releases its platform before this owner is dropped.
        unsafe { tsb_eos_shutdown() };
        LIFECYCLE.store(2, Ordering::SeqCst);
    }
}

pub struct EosClient {
    context: NonNull<c_void>,
    _credentials: [CString; 5],
    sdk: Sdk,
    login_started: bool,
    nat_started: bool,
}
impl EosClient {
    pub fn create(config: &EosConfig, force_relay: bool) -> io::Result<Self> {
        config.validate()?;
        let sdk = Sdk::initialize()?;
        let strings = config.c_strings();
        let mut result = 0;
        // The C bridge owns EOS layouts and callback storage. Retain all strings
        // for the platform lifetime even where the SDK currently copies them.
        let raw = unsafe {
            tsb_eos_create(
                strings[0].as_ptr(),
                strings[1].as_ptr(),
                strings[2].as_ptr(),
                strings[3].as_ptr(),
                strings[4].as_ptr(),
                i32::from(force_relay),
                &mut result,
            )
        };
        check(result, "EOS platform/P2P configuration")?;
        let context =
            NonNull::new(raw).ok_or_else(|| io::Error::other("EOS platform creation failed"))?;
        Ok(Self {
            context,
            _credentials: strings,
            sdk,
            login_started: false,
            nat_started: false,
        })
    }
    pub fn version(&self) -> String {
        self.sdk.version()
    }

    /// Diagnostic Device ID identity; never use it as proof of a Supabase account.
    pub fn login_device(&mut self, timeout: Duration) -> io::Result<String> {
        if !self.login_started {
            self.login_started = true;
            // Context is valid, thread-local, and remains alive throughout callbacks.
            unsafe { tsb_eos_login(self.context.as_ptr()) };
        }
        self.wait(timeout, tsb_eos_login_result, "EOS Connect login")?;
        let mut buffer = [0u8; 128];
        // The bridge/SDK bounds writes by the supplied buffer size.
        check(
            unsafe {
                tsb_eos_user(
                    self.context.as_ptr(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as i32,
                )
            },
            "EOS user ID",
        )?;
        let value =
            CStr::from_bytes_until_nul(&buffer).map_err(|_| invalid("Invalid EOS user ID"))?;
        Ok(value.to_string_lossy().into_owned())
    }
    pub fn query_nat(&mut self, timeout: Duration) -> io::Result<&'static str> {
        if !self.nat_started {
            self.nat_started = true;
            unsafe { tsb_eos_query_nat(self.context.as_ptr()) };
        }
        self.wait(timeout, tsb_eos_nat_result, "EOS NAT query")?;
        Ok(match unsafe { tsb_eos_nat_type(self.context.as_ptr()) } {
            1 => "open",
            2 => "moderate",
            3 => "strict",
            _ => "unknown",
        })
    }
    fn wait(
        &mut self,
        timeout: Duration,
        status: unsafe extern "C" fn(*mut c_void) -> i32,
        operation: &str,
    ) -> io::Result<()> {
        let start = Instant::now();
        loop {
            // Only this thread ticks the platform; callbacks and status reads are serialized.
            unsafe { tsb_eos_tick(self.context.as_ptr()) };
            let result = unsafe { status(self.context.as_ptr()) };
            if result != -1 {
                return check(result, operation);
            }
            if start.elapsed() >= timeout {
                return Err(io::Error::new(io::ErrorKind::TimedOut, format!("{operation} timed out; check credentials, client policy and network access")));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for EosClient {
    fn drop(&mut self) {
        // Release/cancel pending callbacks before freeing bridge state or shutting down EOS.
        unsafe { tsb_eos_destroy(self.context.as_ptr()) };
    }
}

use crate::transport::{eos_id, frames, Reassembler, RoomCode, EOS_FRAME, MAX_DATAGRAM};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::AtomicU64,
        mpsc::{self, Receiver, SyncSender, TrySendError},
        Arc, Mutex, OnceLock,
    },
};

#[derive(Debug, Default)]
struct P2pState {
    closed: bool,
    code: Option<String>,
    error: Option<String>,
    network_type: i32,
    incoming: VecDeque<(Instant, [u8; 32], Vec<u8>)>,
}
enum Command {
    Open {
        id: u64,
        code: Option<RoomCode>,
        host: bool,
        state: Arc<Mutex<P2pState>>,
    },
    Send {
        id: u64,
        peer: [u8; 32],
        bytes: Vec<u8>,
        queued: Instant,
    },
    Close(u64),
    Stop,
}
struct Worker {
    tx: SyncSender<Command>,
    thread: thread::JoinHandle<()>,
}
static WORKER: Mutex<Option<Worker>> = Mutex::new(None);
static SETTINGS: OnceLock<(PathBuf, bool)> = OnceLock::new();
static ROOM_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Configure before the first room. Secrets are only read on the EOS thread.
pub fn configure(path: PathBuf, force_relay: bool) -> io::Result<()> {
    SETTINGS
        .set((path, force_relay))
        .map_err(|_| invalid("EOS already configured"))
}
pub struct ShutdownGuard;
impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        if let Some(worker) = WORKER.lock().unwrap().take() {
            let _ = worker.tx.send(Command::Stop);
            let _ = worker.thread.join();
        }
    }
}
fn worker_sender() -> io::Result<SyncSender<Command>> {
    let mut worker = WORKER.lock().unwrap();
    if worker.is_none() {
        let (path, force) = SETTINGS
            .get_or_init(|| (PathBuf::from("eos.local.json"), false))
            .clone();
        let (tx, rx) = mpsc::sync_channel(512);
        let thread = thread::Builder::new()
            .name("tsb-eos".into())
            .spawn(move || p2p_worker(rx, path, force))?;
        *worker = Some(Worker { tx, thread });
    }
    Ok(worker.as_ref().unwrap().tx.clone())
}

pub struct P2pSocket {
    id: u64,
    tx: SyncSender<Command>,
    state: Arc<Mutex<P2pState>>,
    remote: Option<[u8; 32]>,
}
impl std::fmt::Debug for P2pSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("P2pSocket")
            .field("id", &self.id)
            .field("network", &self.network_label())
            .finish_non_exhaustive()
    }
}
fn pending() -> io::Error {
    io::ErrorKind::WouldBlock.into()
}
impl P2pSocket {
    pub fn open(code: Option<&RoomCode>, host: bool) -> io::Result<Self> {
        let tx = worker_sender()?;
        let state = Arc::new(Mutex::new(P2pState::default()));
        let id = ROOM_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        tx.try_send(Command::Open {
            id,
            code: code.cloned(),
            host,
            state: state.clone(),
        })
        .map_err(|_| io::Error::other("EOS worker unavailable; restart the game"))?;
        Ok(Self {
            id,
            tx,
            state,
            remote: if host { None } else { code.map(|c| c.host) },
        })
    }
    pub fn ready(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.code.is_some() && state.error.is_none()
    }
    pub fn room_code(&self) -> Option<String> {
        self.state.lock().unwrap().code.clone()
    }
    pub fn network_label(&self) -> &'static str {
        match self.state.lock().unwrap().network_type {
            1 => "EOS Direct",
            2 => "EOS Relay",
            _ => "EOS P2P",
        }
    }
    pub fn send(&self, bytes: &[u8]) -> io::Result<usize> {
        self.send_to(
            bytes,
            self.remote.ok_or_else(|| invalid("No EOS host selected"))?,
        )
    }
    pub fn send_to(&self, bytes: &[u8], peer: [u8; 32]) -> io::Result<usize> {
        if bytes.len() > MAX_DATAGRAM {
            return Err(invalid("Packet exceeds EOS datagram limit"));
        }
        let state = self.state.lock().unwrap();
        if let Some(error) = &state.error {
            return Err(io::Error::other(error.clone()));
        }
        if state.code.is_none() {
            return Err(pending());
        }
        drop(state);
        match self.tx.try_send(Command::Send {
            id: self.id,
            peer,
            bytes: bytes.to_vec(),
            queued: Instant::now(),
        }) {
            Ok(()) => Ok(bytes.len()),
            Err(TrySendError::Full(_)) => Err(pending()),
            Err(TrySendError::Disconnected(_)) => Err(io::Error::other("EOS worker stopped")),
        }
    }
    pub fn recv_from(&self, bytes: &mut [u8]) -> io::Result<(usize, [u8; 32])> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = &state.error {
            return Err(io::Error::other(error.clone()));
        }
        while let Some((time, peer, data)) = state.incoming.pop_front() {
            if time.elapsed() > Duration::from_millis(250) {
                continue;
            }
            if data.len() > bytes.len() {
                return Err(invalid("Receive buffer too small"));
            }
            bytes[..data.len()].copy_from_slice(&data);
            return Ok((data.len(), peer));
        }
        Err(pending())
    }
}
impl Drop for P2pSocket {
    fn drop(&mut self) {
        self.state.lock().unwrap().closed = true;
        // Leaving a room must not wait for an in-progress EOS login.
        let _ = self.tx.try_send(Command::Close(self.id));
    }
}
fn random_socket() -> io::Result<String> {
    let mut buffer = [0u8; 33];
    if unsafe { tsb_eos_random(buffer.as_mut_ptr().cast()) } != 0 {
        return Err(io::Error::other("Secure room code generation failed"));
    }
    Ok(String::from_utf8(buffer[..32].to_vec()).unwrap())
}
fn p2p_worker(rx: Receiver<Command>, path: PathBuf, force: bool) {
    // All SDK calls, callbacks, platform destruction and shutdown stay here.
    let initialized = (|| -> io::Result<(EosClient, String)> {
        let mut client = EosClient::create(&EosConfig::load(path)?, force)?;
        let user = client.login_device(Duration::from_secs(30))?;
        Ok((client, user))
    })();
    let (client, user) = match initialized {
        Ok(value) => value,
        Err(error) => {
            while let Ok(command) = rx.recv() {
                match command {
                    Command::Open { state, .. } => {
                        state.lock().unwrap().error = Some(format!("EOS login failed: {error}"))
                    }
                    Command::Stop => break,
                    _ => (),
                }
            }
            return;
        }
    };
    let mut active: Option<(u64, Arc<Mutex<P2pState>>)> = None;
    let mut fragments = Reassembler::default();
    let mut sequence = 0u64;
    loop {
        for _ in 0..128 {
            let command = match rx.try_recv() {
                Ok(c) => c,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(_) => return,
            };
            match command {
                Command::Stop => return,
                Command::Open {
                    id,
                    code,
                    host,
                    state,
                } => {
                    if state.lock().unwrap().closed {
                        continue;
                    }
                    fragments = Reassembler::default();
                    unsafe { tsb_eos_close(client.context.as_ptr()) };
                    if let Some((_, old)) = active.take() {
                        old.lock().unwrap().error = Some("EOS room replaced".into());
                    }
                    let opened = (|| -> io::Result<String> {
                        let local = eos_id(&user)?;
                        if host && code.as_ref().is_some_and(|c| c.host != local) {
                            return Err(invalid("This host file belongs to another EOS device. Create the room using this PC's EOS code."));
                        }
                        let socket = match &code {
                            Some(code) => code.socket.clone(),
                            None => random_socket()?,
                        };
                        let socket_c = CString::new(socket.clone()).unwrap();
                        let remote = if host {
                            None
                        } else {
                            Some(
                                CString::new(
                                    code.as_ref()
                                        .ok_or_else(|| invalid("Missing room code"))?
                                        .host,
                                )
                                .unwrap(),
                            )
                        };
                        check(
                            unsafe {
                                tsb_eos_open(
                                    client.context.as_ptr(),
                                    socket_c.as_ptr(),
                                    remote.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                                )
                            },
                            "EOS open room (host and participant must use different devices)",
                        )?;
                        Ok(RoomCode {
                            host: if host { local } else { code.unwrap().host },
                            socket,
                        }
                        .to_string())
                    })();
                    match opened {
                        Ok(code) => {
                            state.lock().unwrap().code = Some(code);
                            active = Some((id, state));
                        }
                        Err(error) => state.lock().unwrap().error = Some(error.to_string()),
                    }
                }
                Command::Close(id) => {
                    if active.as_ref().is_some_and(|(current, _)| *current == id) {
                        unsafe { tsb_eos_close(client.context.as_ptr()) };
                        active = None;
                    }
                }
                Command::Send {
                    id,
                    peer,
                    bytes,
                    queued,
                } => {
                    if let Some((current, state)) = &active {
                        if *current != id || queued.elapsed() > Duration::from_millis(250) {
                            continue;
                        }
                        let peer = CString::new(peer).unwrap();
                        sequence = sequence.wrapping_add(1);
                        for frame in frames(&bytes, sequence).expect("bounded game packet") {
                            let result = unsafe {
                                tsb_eos_send(
                                    client.context.as_ptr(),
                                    peer.as_ptr(),
                                    frame.as_ptr(),
                                    frame.len() as u32,
                                )
                            };
                            if result != 0 && unsafe { tsb_eos_retryable(result) } == 0 {
                                state.lock().unwrap().error =
                                    check(result, "EOS send").err().map(|e| e.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }
        unsafe { tsb_eos_tick(client.context.as_ptr()) };
        if active
            .as_ref()
            .is_some_and(|(_, state)| state.lock().unwrap().closed)
        {
            unsafe { tsb_eos_close(client.context.as_ptr()) };
            active = None;
        }
        if let Some((_, state)) = &active {
            let login = unsafe { tsb_eos_login_result(client.context.as_ptr()) };
            if login != -1 && login != 0 {
                state.lock().unwrap().error = check(login, "EOS authentication refresh")
                    .err()
                    .map(|e| e.to_string());
            }
            for _ in 0..128 {
                let mut peer = [0u8; 128];
                let mut bytes = [0u8; EOS_FRAME];
                let mut length = 0;
                let result = unsafe {
                    tsb_eos_receive(
                        client.context.as_ptr(),
                        peer.as_mut_ptr().cast(),
                        peer.len() as i32,
                        bytes.as_mut_ptr(),
                        bytes.len() as u32,
                        &mut length,
                    )
                };
                if result == -1 {
                    break;
                }
                if let Err(error) = check(result, "EOS receive") {
                    state.lock().unwrap().error = Some(error.to_string());
                    break;
                }
                let parsed = CStr::from_bytes_until_nul(&peer)
                    .ok()
                    .and_then(|s| s.to_str().ok())
                    .and_then(|s| eos_id(s).ok());
                if let Some(peer) = parsed {
                    if let Some(bytes) =
                        fragments.receive(peer, &bytes[..length as usize], Instant::now())
                    {
                        let mut state = state.lock().unwrap();
                        if state.incoming.len() >= 512 {
                            state.incoming.pop_front();
                        }
                        state.incoming.push_back((Instant::now(), peer, bytes));
                    }
                }
            }
            state.lock().unwrap().network_type =
                unsafe { tsb_eos_network_type(client.context.as_ptr()) };
        }
        thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> EosConfig {
        EosConfig {
            product_id: "a".repeat(32),
            sandbox_id: "b".repeat(32),
            deployment_id: "c".repeat(32),
            client_id: "xyz-test-client".into(),
            client_secret: "private-test-value".into(),
        }
    }
    #[test]
    fn invalid_credentials_are_rejected_without_echoing_secrets() {
        let mut config = config();
        assert!(config.validate().is_ok());
        assert!(!format!("{config:?}").contains("private-test-value"));
        config.client_secret.push('\n');
        let error = config.validate().unwrap_err().to_string();
        assert!(error.contains("client_secret"));
        assert!(!error.contains("private-test-value"));
        config.product_id.clear();
        assert!(config
            .validate()
            .unwrap_err()
            .to_string()
            .contains("product_id"));
    }
    #[test]
    fn sdk_has_one_owner_and_releases_cleanly_without_credentials() {
        let sdk = Sdk::initialize().unwrap();
        assert!(!sdk.version().is_empty());
        assert!(Sdk::initialize().is_err());
        drop(sdk);
        assert!(Sdk::initialize().is_err());
    }
}
