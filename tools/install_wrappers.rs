//! Install wrapper entry points beside a built or installed Joom executable.
use anyhow::{Context, Result, ensure};
use std::path::Path;

fn install(directory: &Path) -> Result<usize> {
    let executable = directory.join(format!("joom{}", std::env::consts::EXE_SUFFIX));
    ensure!(
        executable.is_file(),
        "{}: build or install joom first",
        executable.display()
    );
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/wrappers.schema.json"))?;
    let names = schema["x-wrappers"]
        .as_object()
        .context("missing wrapper definitions")?;
    for name in names.keys() {
        let target = directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        #[cfg(unix)]
        {
            match std::fs::symlink_metadata(&target) {
                Ok(metadata)
                    if metadata.file_type().is_symlink()
                        && std::fs::read_link(&target)? == Path::new("joom") =>
                {
                    continue;
                }
                Ok(_) => anyhow::bail!(
                    "{}: an unrelated file or symbolic link already exists",
                    target.display()
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            std::os::unix::fs::symlink("joom", &target)?;
        }
        #[cfg(windows)]
        std::fs::copy(&executable, &target)?;
    }
    #[cfg(unix)]
    for name in names
        .keys()
        .map(String::as_str)
        .chain(["jmforms", "jmyt", "jmphotos"])
    {
        if !names.contains_key(name) {
            let obsolete = directory.join(name);
            if std::fs::read_link(&obsolete).is_ok_and(|target| target == Path::new("joom")) {
                std::fs::remove_file(obsolete)?;
            }
        }
        if let Some(suffix) = name.strip_prefix("jm") {
            let previous = directory.join(format!("j{suffix}"));
            match std::fs::read_link(&previous) {
                Ok(target) if target == Path::new("joom") => std::fs::remove_file(previous)?,
                Ok(_) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidInput
                    ) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(names.len())
}

fn main() -> Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    ensure!(
        args.len() == 2 && args[0] == "--directory",
        "usage: install_wrappers --directory PATH"
    );
    let directory = Path::new(&args[1]);
    let count = install(directory)?;
    use std::io::Write;
    writeln!(
        std::io::stdout().lock(),
        "Installed {count} wrappers in {}",
        directory.display()
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/install_wrappers.rs"]
mod tests;
