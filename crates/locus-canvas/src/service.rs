use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Child, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

const FILES: &[(&str, &str)] = &[
    ("package.json", include_str!("../web/package.json")),
    (
        "package-lock.json",
        include_str!("../web/package-lock.json"),
    ),
    ("launch.mjs", include_str!("../web/launch.mjs")),
    ("server.mjs", include_str!("../web/server.mjs")),
    ("plugin.mjs", include_str!("../web/plugin.mjs")),
    ("library.mjs", include_str!("../web/library.mjs")),
    ("packages.mjs", include_str!("../web/packages.mjs")),
    ("host.tsx", include_str!("../web/host.tsx")),
    ("host.css", include_str!("../web/host.css")),
    ("sdk.tsx", include_str!("../web/sdk.tsx")),
];

pub struct CanvasRuntime {
    root: PathBuf,
    process: Mutex<Option<ServerProcess>>,
}

struct ServerProcess {
    child: Option<Child>,
    origin: String,
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        child.stdin.take();
        std::thread::spawn(move || {
            for _ in 0..50 {
                if child.try_wait().ok().flatten().is_some() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            let _ = child.kill();
            let _ = child.wait();
        });
    }
}

impl CanvasRuntime {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            process: Mutex::new(None),
        }
    }

    pub(crate) fn has_exited(&self) -> bool {
        self.process.try_lock().ok().is_some_and(|mut process| {
            process.as_mut().is_some_and(|server| {
                server
                    .child
                    .as_mut()
                    .unwrap()
                    .try_wait()
                    .ok()
                    .flatten()
                    .is_some()
            })
        })
    }

    pub fn page_url(&self, directory: &Path) -> io::Result<String> {
        let root = self.root.canonicalize()?;
        let directory = directory.canonicalize()?;
        let relative = directory.strip_prefix(&root).map_err(io::Error::other)?;
        let path = relative
            .components()
            .map(|part| {
                percent_encoding::utf8_percent_encode(
                    &part.as_os_str().to_string_lossy(),
                    percent_encoding::NON_ALPHANUMERIC,
                )
                .to_string()
            })
            .collect::<Vec<_>>()
            .join("/");
        if path.is_empty() {
            return Err(io::Error::other("Canvas must belong to a note"));
        }
        let mut process = self
            .process
            .lock()
            .map_err(|_| io::Error::other("Canvas service unavailable"))?;
        if let Some(server) = process.as_mut()
            && server.child.as_mut().unwrap().try_wait()?.is_some()
        {
            *process = None;
        }
        if process.is_none() {
            *process = Some(start(&root)?);
        }
        Ok(format!(
            "{}/notes/{path}/",
            process.as_ref().unwrap().origin
        ))
    }
}

fn runtime_directory() -> io::Result<PathBuf> {
    let mut digest = Sha256::new();
    for (name, source) in FILES {
        digest.update(name);
        digest.update(source);
    }
    let fingerprint = format!("{:x}", digest.finalize());
    let home = dirs::home_dir().ok_or_else(|| io::Error::other("Cannot locate home directory"))?;
    let directory = home.join(".oneo/locus/canvas").join(fingerprint);
    fs::create_dir_all(&directory)?;
    for (name, source) in FILES {
        let file = directory.join(name);
        if fs::read(&file).ok().as_deref() != Some(source.as_bytes()) {
            fs::write(file, source)?;
        }
    }
    Ok(directory)
}

fn start(root: &Path) -> io::Result<ServerProcess> {
    let directory = runtime_directory()?;
    let mut child = locus_library::background_command("node")
        .arg(directory.join("launch.mjs"))
        .arg(root)
        .current_dir(&directory)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            io::Error::other(format!(
                "Could not start Node.js. Install Node.js and npm, then reload. {error}"
            ))
        })?;
    let stderr = child.stderr.take().unwrap();
    let errors = Arc::new(Mutex::new(String::new()));
    let output = errors.clone();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut bytes = [0; 4096];
        while let Ok(count) = reader.read(&mut bytes) {
            if count == 0 {
                break;
            }
            if let Ok(mut output) = output.lock() {
                output.push_str(&String::from_utf8_lossy(&bytes[..count]));
                if output.len() > 32000 {
                    let mut boundary = output.len() - 16000;
                    while !output.is_char_boundary(boundary) {
                        boundary += 1;
                    }
                    output.drain(..boundary);
                }
            }
        }
    });
    let stdout = child.stdout.take().unwrap();
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(origin) = line.strip_prefix("LOCUS_READY ") {
                let _ = send.send(origin.to_owned());
            }
        }
    });
    match receive.recv_timeout(Duration::from_secs(180)) {
        Ok(origin)
            if origin
                .strip_prefix("http://127.0.0.1:")
                .is_some_and(|port| port.parse::<u16>().is_ok_and(|port| port != 0)) =>
        {
            Ok(ServerProcess {
                child: Some(child),
                origin,
            })
        }
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            Err(io::Error::other(format!(
                "Canvas service did not start. Check Node.js, npm and dependency installation, then reload.\n{}",
                errors.lock().unwrap()
            )))
        }
    }
}
