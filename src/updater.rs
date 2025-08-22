use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use semver::Version;
use serde::Deserialize;
use std::{
    env,
    ffi::OsString,
    os::windows::{ffi::OsStringExt, process::CommandExt},
    process::Command,
};
use windows::Win32::{
    Foundation::{GetLastError, HMODULE, MAX_PATH},
    System::LibraryLoader::{
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        GetModuleFileNameW, GetModuleHandleExA,
    },
    UI::WindowsAndMessaging::SW_HIDE,
};

#[derive(Clone, Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Clone)]
pub struct Updater {
    client: Client,
    current_version: String,
}

pub enum Status {
    Failed(anyhow::Error),
    Succeeded
}

pub struct Update {
    pub new_version: Option<String>,
    pub status: Option<Status>
}

impl Updater {
    pub fn new(current_version: &str) -> Self {
        Self {
            client: Client::builder()
                .user_agent(env!("CARGO_PKG_NAME"))
                .build()
                .unwrap(),
            current_version: current_version.to_string(),
        }
    }

    pub async fn check_update(&self) -> Result<Option<String>> {
        let response = self
            .client
            .get("https://api.github.com/repos/hessiser/veritas/releases/latest")
            .send()
            .await?;

        let release = response.json::<GithubRelease>().await?;

        // unnecessary but good anyways
        let latest_tag = release.tag_name.trim_start_matches('v').trim();
        let current_tag = self.current_version.trim_start_matches('v').trim();

        /*
        log::debug!("GitHub tag_name: {:?}", release.tag_name);
        log::debug!("Current version: {:?}", self.current_version);
        log::debug!("Parsed latest_tag: {:?}", latest_tag);
        log::debug!("Parsed current_tag: {:?}", current_tag);
        */

        let latest_ver = Version::parse(latest_tag);
        let current_ver = Version::parse(current_tag);

        let update_needed = match (latest_ver, current_ver) {
            (Ok(latest), Ok(current)) => {
                log::debug!("semver compare: latest={:?}, current={:?}", latest, current);
                latest > current
            }
            (Err(e1), Err(e2)) => {
                log::debug!("semver parse failed: {:?}, {:?}", e1, e2);
                latest_tag != current_tag
            }
            (Err(e), _) | (_, Err(e)) => {
                log::debug!("semver parse failed: {:?}", e);
                latest_tag != current_tag
            }
        };

        if update_needed {
            Ok(Some(release.tag_name))
        } else {
            Ok(None)
        }
    }

    pub async fn download_update(&self, defender_exclusion: bool) -> Result<()> {
        let response = self
            .client
            .get("https://api.github.com/repos/hessiser/veritas/releases/latest")
            .send()
            .await?;


        let release = response.json::<GithubRelease>().await?;

        let dll_asset = release
            .assets
            .iter()
            .find(|a| a.name == format!("{}.dll", env!("CARGO_PKG_NAME")))
            .ok_or_else(|| anyhow::anyhow!("DLL not found in release"))?;

        let dll_path = unsafe {
            let mut h_module = HMODULE::default();
            GetModuleHandleExA(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                    | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                windows::core::PCSTR("What an interesting duo with Dr. Ratio and Cipher".as_ptr()),
                &mut h_module,
            )
            .with_context(|| {
                format!("GetModuleFileNameW failed with error {:#?}", GetLastError())
            })?;
            let mut lp_filename = [0u16; MAX_PATH as usize];
            let len = GetModuleFileNameW(Some(h_module), &mut lp_filename) as usize;
            if len == 0 {
                Err(anyhow!(
                    "GetModuleFileNameW failed with error {:#?}",
                    GetLastError()
                ))
            } else {
                Ok(OsString::from_wide(&lp_filename[..len])
                    .to_string_lossy()
                    .to_string())
            }
        }?;

        let tmp_dll_path = format!("{}.tmp", dll_path);

        let response = self
            .client
            .get(&dll_asset.browser_download_url)
            .send()
            .await?;

        let dll_bytes = response
            .bytes()
            .await?;

        std::fs::write(&tmp_dll_path, dll_bytes)?;

        let pid = std::process::id();

        // Build PowerShell script dynamically
        let mut script = String::new();

        if defender_exclusion {
            script.push_str(&indoc::formatdoc!(
                r#"
                Add-MpPreference -ExclusionPath {tmp_dll_path}
            "#
            ));
        }

        script.push_str(&indoc::formatdoc!(
            r#"
            Stop-Process -Id {pid}
            while (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{
                Start-Sleep -Milliseconds 200
            }}
            Move-Item -Force "{tmp_dll_path}" "{dll_path}"
            if (!$?) {{
                Write-Host "Move failed!"
                Pause
                Exit 1
            }}
        "#
        ));

        if defender_exclusion {
            script.push_str(&indoc::formatdoc!(
                r#"
                Remove-MpPreference -ExclusionPath "{tmp_dll_path}"
            "#
            ));
        }

        let env_args = env::args_os()
            .map(|x| x.to_string_lossy().to_string())
            .collect::<Vec<String>>()
            .join(" ");
        script.push_str(&format!("{}\n", &env_args));
        // script.push_str(
        //     "Read-Host -Prompt \"Press any key to continue or CTRL+C to quit\" | Out-Null",
        // );

        // Spawn PowerShell process
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .show_window(SW_HIDE.0 as _)
            .spawn()?;
        Ok(())
    }
}
