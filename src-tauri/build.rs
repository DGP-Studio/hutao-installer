fn main() {
    let build_mode = std::env::var("BUILD_MODE").unwrap_or_else(|_| "online".into());
    let embedded_version = std::env::var("EMBEDDED_VERSION").unwrap_or_else(|_| "".into());
    if build_mode == "offline" && embedded_version.is_empty() {
        panic!("offline mode requires EMBEDDED_VERSION to be set");
    }

    println!("cargo:rustc-env=BUILD_MODE={build_mode}");
    println!("cargo:rustc-env=EMBEDDED_VERSION={embedded_version}");

    let windows = tauri_build::WindowsAttributes::new().app_manifest(
        r#"
<assembly manifestVersion="1.0" xmlns="urn:schemas-microsoft-com:asm.v1">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>
"#,
    );

    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to build tauri app");
}
