// crates/swg-client-render/src/platform.rs

use serde::{Deserialize, Serialize};

/// Target operating systems supported for deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperatingSystem {
    Windows,
    MacOS,
    Linux,
}

impl OperatingSystem {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::MacOS => "macOS",
            Self::Linux => "Linux",
        }
    }
}

/// Graphics hardware API backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphicsApi {
    Vulkan,
    Metal,
    Dx12,
    Dx11,
    Gl,
}

impl GraphicsApi {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Vulkan => "Vulkan",
            Self::Metal => "Metal",
            Self::Dx12 => "DirectX 12",
            Self::Dx11 => "DirectX 11",
            Self::Gl => "OpenGL / WebGL",
        }
    }
}

/// Configuration describing executable packaging and graphics API selection for a target platform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformPackageConfig {
    pub target_os: OperatingSystem,
    pub primary_api: GraphicsApi,
    pub fallback_api: GraphicsApi,
    pub executable_name: String,
    pub app_id: String,
    pub high_dpi_enabled: bool,
}

pub struct PlatformManager;

impl PlatformManager {
    /// Detects the host operating system at runtime.
    #[must_use]
    pub fn current_os() -> OperatingSystem {
        if cfg!(target_os = "windows") {
            OperatingSystem::Windows
        } else if cfg!(target_os = "macos") {
            OperatingSystem::MacOS
        } else {
            OperatingSystem::Linux
        }
    }

    /// Provides default production packaging configuration for the target OS.
    #[must_use]
    pub fn default_config(os: OperatingSystem) -> PlatformPackageConfig {
        match os {
            OperatingSystem::Windows => PlatformPackageConfig {
                target_os: os,
                primary_api: GraphicsApi::Dx12,
                fallback_api: GraphicsApi::Vulkan,
                executable_name: "SWGClient.exe".into(),
                app_id: "com.baylyai.swg.client".into(),
                high_dpi_enabled: true,
            },
            OperatingSystem::MacOS => PlatformPackageConfig {
                target_os: os,
                primary_api: GraphicsApi::Metal,
                fallback_api: GraphicsApi::Vulkan, // via MoltenVK
                executable_name: "SWGClient".into(),
                app_id: "com.baylyai.swg.client".into(),
                high_dpi_enabled: true,
            },
            OperatingSystem::Linux => PlatformPackageConfig {
                target_os: os,
                primary_api: GraphicsApi::Vulkan,
                fallback_api: GraphicsApi::Gl,
                executable_name: "swg-client".into(),
                app_id: "com.baylyai.swg.client".into(),
                high_dpi_enabled: true,
            },
        }
    }

    /// Validates whether a graphics API is supported natively on the target operating system.
    #[must_use]
    pub fn is_api_supported(os: OperatingSystem, api: GraphicsApi) -> bool {
        matches!(
            (os, api),
            (
                OperatingSystem::Windows,
                GraphicsApi::Dx12 | GraphicsApi::Dx11 | GraphicsApi::Vulkan | GraphicsApi::Gl,
            ) | (
                OperatingSystem::MacOS,
                GraphicsApi::Metal | GraphicsApi::Vulkan | GraphicsApi::Gl,
            ) | (
                OperatingSystem::Linux,
                GraphicsApi::Vulkan | GraphicsApi::Gl
            )
        )
    }

    /// Generates macOS `Info.plist` content for app bundle packaging.
    #[must_use]
    pub fn generate_macos_plist(config: &PlatformPackageConfig) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>{}</string>
    <key>CFBundleIdentifier</key>
    <string>{}</string>
    <key>CFBundleName</key>
    <string>Star Wars Galaxies Client</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
"#,
            config.executable_name, config.app_id
        )
    }

    /// Generates Windows application manifest for DPI awareness and visual styles.
    #[must_use]
    pub fn generate_windows_manifest(config: &PlatformPackageConfig) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
    <assemblyIdentity version="1.0.0.0" name="{}" type="win32"/>
    <application xmlns="urn:schemas-microsoft-com:asm.v3">
        <windowsSettings>
            <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
            <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
        </windowsSettings>
    </application>
</assembly>
"#,
            config.app_id
        )
    }

    /// Generates Linux desktop entry specification.
    #[must_use]
    pub fn generate_linux_desktop_entry(config: &PlatformPackageConfig) -> String {
        format!(
            r#"[Desktop Entry]
Type=Application
Name=Star Wars Galaxies
Exec={}
Icon=swg-icon
Terminal=false
Categories=Game;RolePlaying;
"#,
            config.executable_name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_api_validation() {
        assert!(PlatformManager::is_api_supported(
            OperatingSystem::Windows,
            GraphicsApi::Dx12
        ));
        assert!(PlatformManager::is_api_supported(
            OperatingSystem::Windows,
            GraphicsApi::Vulkan
        ));
        assert!(!PlatformManager::is_api_supported(
            OperatingSystem::Windows,
            GraphicsApi::Metal
        ));

        assert!(PlatformManager::is_api_supported(
            OperatingSystem::MacOS,
            GraphicsApi::Metal
        ));
        assert!(!PlatformManager::is_api_supported(
            OperatingSystem::MacOS,
            GraphicsApi::Dx12
        ));

        assert!(PlatformManager::is_api_supported(
            OperatingSystem::Linux,
            GraphicsApi::Vulkan
        ));
        assert!(!PlatformManager::is_api_supported(
            OperatingSystem::Linux,
            GraphicsApi::Dx12
        ));
    }

    #[test]
    fn test_packaging_manifest_generation() {
        let mac_config = PlatformManager::default_config(OperatingSystem::MacOS);
        let plist = PlatformManager::generate_macos_plist(&mac_config);
        assert!(plist.contains("<key>NSHighResolutionCapable</key>"));
        assert!(plist.contains(&mac_config.executable_name));

        let win_config = PlatformManager::default_config(OperatingSystem::Windows);
        let manifest = PlatformManager::generate_windows_manifest(&win_config);
        assert!(manifest.contains("PerMonitorV2"));

        let lin_config = PlatformManager::default_config(OperatingSystem::Linux);
        let desktop = PlatformManager::generate_linux_desktop_entry(&lin_config);
        assert!(desktop.contains("Categories=Game;RolePlaying;"));
    }
}
