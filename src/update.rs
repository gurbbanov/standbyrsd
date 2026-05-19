use crate::CURRENT_VERSION;

pub fn check_for_update() -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("gurbbanov")
        .repo_name("standbyrsd")
        .build()?
        .fetch()?;

    let latest = match releases.first() {
        Some(r) => r,
        None => return Ok(None),
    };

    let latest_ver = latest.version.trim_start_matches('v');

    if self_update::version::bump_is_greater(CURRENT_VERSION, latest_ver)? {
        return Ok(Some(latest_ver.to_string()));
    }

    Ok(None)
}

pub fn apply_update() -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("gurbbanov")
        .repo_name("standbyrsd")
        .build()?
        .fetch()?;

    let latest = match releases.first() {
        Some(r) => r,
        None => return Ok(None),
    };

    let mut builder = self_update::backends::github::Update::configure();
    builder
        .repo_owner("gurbbanov")
        .repo_name("standbyrsd")
        .bin_name("standbyrsd")
        .current_version(CURRENT_VERSION)
        .no_confirm(true)
        .target_version_tag(&latest.name);

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        builder.target("aarch64-apple-darwin");
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        builder.target("x86_64-apple-darwin");
    }
    #[cfg(target_os = "linux")]
    {
        builder.target("linux");
    }
    #[cfg(target_os = "windows")]
    {
        apply_update_windows();
    }

    #[cfg(target_os = "macos")]
    {
        builder.bin_path_in_archive("standbyrsd.app/Contents/MacOS/standbyrsd");
    }
    #[cfg(target_os = "linux")]
    {
        builder.bin_path_in_archive("standbyrsd-linux/standbyrsd");
    }

    let status = builder.build()?.update()?;

    if status.updated() {
        Ok(Some(status.version().to_string()))
    } else {
        Ok(None)
    }
}

#[cfg(target_os = "windows")]
pub fn apply_update_windows() -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("gurbbanov")
        .repo_name("standbyrsd")
        .build()?
        .fetch()?;

    let latest = match releases.first() {
        Some(r) => r,
        None => return Ok(None),
    };

    let asset = latest
        .assets
        .iter()
        .find(|a| a.name.ends_with(".exe"))
        .ok_or("no .exe asset found")?;

    let current_exe = std::env::current_exe()?;
    let new_exe = current_exe.with_file_name("standbyrsd_new.exe");
    let bat_path = current_exe.with_file_name("update.bat");

    let mut new_exe_file = std::fs::File::create(&new_exe)?;

    self_update::Download::from_url(&asset.download_url)
        .set_header(reqwest::header::ACCEPT, "application/octet-stream".parse()?)
        .download_to(&mut new_exe_file)?;

    let bat = format!(
        "@echo off\r\ntimeout /t 1 /nobreak >nul\r\nmove /Y \"{new}\" \"{cur}\"\r\nstart \"\" \"{cur}\"\r\ndel \"%~f0\"",
        new = new_exe.display(),
        cur = current_exe.display(),
    );
    std::fs::write(&bat_path, bat)?;

    std::process::Command::new("cmd")
        .args(["/C", bat_path.to_str().unwrap()])
        .spawn()?;

    Ok(Some(latest.version.trim_start_matches('v').to_string()))
}
