fn main() {
    let build_mode = std::env::var("BUILD_MODE").unwrap_or_else(|_| "online".into());
    let embedded_version = std::env::var("EMBEDDED_VERSION").unwrap_or_else(|_| "".into());
    if build_mode == "offline" {
        if embedded_version.is_empty() {
            panic!("offline mode requires EMBEDDED_VERSION to be set");
        }

        compress("Snap.Hutao.msix")
    }

    compress("SegoeIcons.ttf");

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

#[allow(unused_variables)]
fn compress(file_name: &str) {
    #[cfg(not(debug_assertions))]
    {
        use std::io::{Read, Write};
        let start_time = std::time::Instant::now();
        println!("cargo:warning=Compressing {}", file_name);

        let mut input = std::fs::File::open(file_name).unwrap();
        let mut input_bytes = Vec::new();
        input.read_to_end(&mut input_bytes).unwrap();
        drop(input);

        let original_size = input_bytes.len();

        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(&input_bytes).unwrap();
        let compressed = encoder.finish().unwrap();

        let compressed_size = compressed.len();
        let compression_ratio = (1.0 - (compressed_size as f64 / original_size as f64)) * 100.0;

        std::fs::write(file_name, compressed).unwrap();

        let elapsed = start_time.elapsed();
        println!(
            "cargo:warning=Compressed {} ({} bytes -> {} bytes, {:.1}% reduction) in {:.2}ms",
            file_name,
            original_size,
            compressed_size,
            compression_ratio,
            elapsed.as_secs_f64() * 1000.0
        );
    }
}
