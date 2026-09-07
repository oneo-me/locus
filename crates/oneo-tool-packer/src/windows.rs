use crate::{cli::Target, config::Project};
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn compile_icon(project: &Project) -> Result<tempfile::TempDir> {
    let directory = tempfile::Builder::new()
        .prefix("oneo-packer-icon-")
        .tempdir()?;
    fs::copy(
        project.root.join("icon.ico"),
        directory.path().join("icon.ico"),
    )?;
    fs::write(directory.path().join("icon.rc"), "1 ICON \"icon.ico\"\n")?;
    crate::captured(
        Command::new("rc")
            .current_dir(directory.path())
            .args(["/nologo", "/fo", "icon.res", "icon.rc"]),
    )
    .context(
        "compile icon.ico with Windows SDK rc; use a Visual Studio C++ developer environment",
    )?;
    ensure!(
        directory.path().join("icon.res").is_file(),
        "rc did not produce icon.res"
    );
    Ok(directory)
}

pub fn copy_dependencies(
    project: &Project,
    target: &Target,
    executable: &Path,
    destination: &Path,
) -> Result<()> {
    let mut directories = vec![
        executable
            .parent()
            .context("executable directory")?
            .to_owned(),
    ];
    directories.extend(
        project
            .config
            .windows
            .runtime_directories
            .iter()
            .map(|p| project.root.join(p)),
    );
    if let Some(redist) = std::env::var_os("VCToolsRedistDir") {
        let arch = if target.arch == "aarch64" {
            "arm64"
        } else {
            "x64"
        };
        let root = PathBuf::from(redist).join(arch);
        if root.is_dir() {
            for entry in fs::read_dir(root)? {
                let path = entry?.path();
                if path.is_dir()
                    && path
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().ends_with(".CRT"))
                {
                    directories.push(path);
                }
            }
        }
    }
    let system = PathBuf::from(std::env::var_os("SystemRoot").context("SystemRoot is unset")?)
        .join("System32");
    let extras: Vec<_> = project
        .config
        .windows
        .runtime_files
        .iter()
        .map(|p| project.root.join(p))
        .collect();
    let dependencies = collect(executable, &extras, &directories, &system, target, |path| {
        let bytes = crate::captured(
            Command::new("dumpbin")
                .args(["/nologo", "/dependents"])
                .arg(path),
        )?;
        Ok(import_names(&String::from_utf8_lossy(&bytes)))
    })?;
    for (name, source) in dependencies {
        let output = destination.join(name);
        ensure!(
            !output.try_exists()?,
            "DLL destination already exists: {}",
            output.display()
        );
        fs::copy(&source, &output).with_context(|| format!("copy runtime {}", source.display()))?;
    }
    Ok(())
}

fn collect(
    executable: &Path,
    extras: &[PathBuf],
    directories: &[PathBuf],
    system: &Path,
    target: &Target,
    mut imports: impl FnMut(&Path) -> Result<Vec<String>>,
) -> Result<BTreeMap<String, PathBuf>> {
    let mut queue = VecDeque::from([executable.to_owned()]);
    let mut files = BTreeMap::new();
    for path in extras {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .context("invalid runtime filename")?
            .to_ascii_lowercase();
        crate::config::component(&name)?;
        ensure!(name.ends_with(".dll"), "runtime-files must contain DLLs");
        ensure!(
            files.insert(name, path.clone()).is_none(),
            "duplicate runtime filename"
        );
        queue.push_back(path.clone());
    }
    let mut visited = HashSet::new();
    while let Some(path) = queue.pop_front() {
        if !visited.insert(path.clone()) {
            continue;
        }
        verify_machine(&path, target)?;
        for name in imports(&path)? {
            let name = name.to_ascii_lowercase();
            crate::config::component(&name)?;
            if files.contains_key(&name)
                || name.starts_with("api-ms-win-")
                || name.starts_with("ext-ms-win-")
            {
                continue;
            }
            let runtime = name.starts_with("vcruntime")
                || name.starts_with("msvcp")
                || name.starts_with("concrt")
                || name == "ucrtbased.dll";
            if !runtime && find_file(system, &name)?.is_some() {
                continue;
            }
            let mut found = None;
            for directory in directories {
                if let Some(source) = find_file(directory, &name)? {
                    found = Some(source);
                    break;
                }
            }
            let source = found.with_context(|| format!("missing runtime {name} imported by {}; configure windows.runtime-directories or VCToolsRedistDir", path.display()))?;
            queue.push_back(source.clone());
            files.insert(name, source);
        }
    }
    Ok(files)
}

fn find_file(directory: &Path, name: &str) -> Result<Option<PathBuf>> {
    if !directory.is_dir() {
        return Ok(None);
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(name)
            && entry.file_type()?.is_file()
        {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

fn import_names(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| line.starts_with(char::is_whitespace))
        .map(str::trim)
        .filter(|line| line.to_ascii_lowercase().ends_with(".dll"))
        .map(str::to_owned)
        .collect()
}

fn verify_machine(path: &Path, target: &Target) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("read PE image {}", path.display()))?;
    ensure!(
        bytes.starts_with(b"MZ"),
        "not a PE image: {}",
        path.display()
    );
    let offset_bytes: [u8; 4] = bytes
        .get(0x3c..0x40)
        .context("truncated PE header")?
        .try_into()?;
    let offset = u32::from_le_bytes(offset_bytes) as usize;
    let header = bytes
        .get(offset..offset.saturating_add(6))
        .context("invalid PE header offset")?;
    ensure!(&header[..4] == b"PE\0\0", "invalid PE signature");
    let machine = u16::from_le_bytes([header[4], header[5]]);
    let expected = if target.arch == "aarch64" {
        0xaa64
    } else {
        0x8664
    };
    if machine != expected {
        bail!(
            "{} has PE machine {machine:#x}, expected {expected:#x} for {}",
            path.display(),
            target.triple
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pe(path: &Path, machine: u16) {
        let mut bytes = vec![0; 70];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&64u32.to_le_bytes());
        bytes[64..68].copy_from_slice(b"PE\0\0");
        bytes[68..70].copy_from_slice(&machine.to_le_bytes());
        fs::write(path, bytes).unwrap();
    }
    #[test]
    fn parses_normal_and_delay_load_dependencies() {
        assert_eq!(
            import_names(
                "Dump of file app.dll\n dependencies:\n  KERNEL32.dll\n delay load dependencies:\n  helper library.dll\n Summary\n"
            ),
            ["KERNEL32.dll", "helper library.dll"]
        );
    }
    #[test]
    fn recursively_collects_runtime_dlls_and_excludes_system_libraries() {
        let root = tempfile::tempdir().unwrap();
        let system = root.path().join("system");
        fs::create_dir(&system).unwrap();
        fs::write(system.join("KERNEL32.dll"), "system").unwrap();
        fs::write(
            system.join("vcruntime140.dll"),
            "installed runtime is not a distribution source",
        )
        .unwrap();
        for name in ["app.exe", "Helper.dll", "vcruntime140.dll"] {
            pe(&root.path().join(name), 0x8664);
        }
        let target = Target::resolve(Some("x86_64-pc-windows-msvc"), "").unwrap();
        let result = collect(
            &root.path().join("app.exe"),
            &[],
            &[root.path().to_owned()],
            &system,
            &target,
            |path| {
                Ok(if path.ends_with("app.exe") {
                    vec!["KERNEL32.dll".into(), "helper.dll".into()]
                } else if path.ends_with("Helper.dll") {
                    vec!["vcruntime140.dll".into()]
                } else {
                    vec![
                        "helper.dll".into(),
                        "api-ms-win-crt-runtime-l1-1-0.dll".into(),
                    ]
                })
            },
        )
        .unwrap();
        assert_eq!(
            result.keys().cloned().collect::<Vec<_>>(),
            ["helper.dll", "vcruntime140.dll"]
        );
    }
    #[test]
    fn rejects_wrong_architecture_and_missing_runtime() {
        let root = tempfile::tempdir().unwrap();
        let exe = root.path().join("app.exe");
        pe(&exe, 0xaa64);
        let target = Target::resolve(Some("x86_64-pc-windows-msvc"), "").unwrap();
        assert!(verify_machine(&exe, &target).is_err());
        pe(&exe, 0x8664);
        assert!(
            collect(&exe, &[], &[], root.path(), &target, |_| Ok(vec![
                "missing.dll".into()
            ]))
            .is_err()
        );
    }
}
