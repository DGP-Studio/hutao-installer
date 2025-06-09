use crate::{capture_and_return_default, capture_and_return_err, utils::process};
use windows::Management::Deployment::RemovalOptions;
use windows::{
    core::{Error, HRESULT, HSTRING},
    Foundation::Uri,
    Management::Deployment::{AddPackageOptions, DeploymentProgress, PackageManager},
};
use windows_future::AsyncStatus;

pub fn need_migration() -> bool {
    let package_manager = PackageManager::new();
    if package_manager.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!(
                "Failed to create package manager: {:?}",
                package_manager.err()
            ),
            false
        );
    }

    let package_manager = package_manager.unwrap();

    let package_family_name = HSTRING::from("60568DGPStudio.SnapHutao_ebfp3nyc27j86".to_string());
    let packages = package_manager.FindPackagesByPackageFamilyName(&package_family_name);
    if packages.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!(
                "Failed to find packages by package family name: {:?}",
                packages.err()
            ),
            false
        );
    }

    let packages = packages.unwrap();
    let iter = packages.First();
    if iter.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get first package iterator: {:?}", iter.err()),
            false
        );
    }
    let iter = iter.unwrap();

    let has_current = iter.HasCurrent();
    if has_current.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to check if has current: {:?}", has_current.err()),
            false
        );
    }
    has_current.unwrap()
}

pub fn try_get_hutao_version() -> Option<String> {
    let package_manager = PackageManager::new();
    if package_manager.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!(
                "Failed to create package manager: {:?}",
                package_manager.err()
            ),
            None
        );
    }

    let package_manager = package_manager.unwrap();

    let package_family_name = HSTRING::from("60568DGPStudio.SnapHutao_wbnnev551gwxy".to_string());
    let packages = package_manager.FindPackagesByPackageFamilyName(&package_family_name);
    if packages.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!(
                "Failed to find packages by package family name: {:?}",
                packages.err()
            ),
            None
        );
    }

    let packages = packages.unwrap();
    let iter = packages.First();
    if iter.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get first package iterator: {:?}", iter.err()),
            None
        );
    }
    let iter = iter.unwrap();

    let has_current = iter.HasCurrent();
    if has_current.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to check if has current: {:?}", has_current.err()),
            None
        );
    }

    if !has_current.unwrap() {
        return None;
    }

    let package = iter.Current();
    if package.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get current package: {:?}", package.err()),
            None
        );
    }
    let package = package.unwrap();
    let id = package.Id();
    if id.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get package ID: {:?}", id.err()),
            None
        );
    }
    let id = id.unwrap();
    let version = id.Version();
    if version.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get package version: {:?}", version.err()),
            None
        );
    }
    let version = version.unwrap();
    Some(format!(
        "{}.{}.{}.{}",
        version.Major, version.Minor, version.Build, version.Revision
    ))
}

pub fn add_package(
    raw_package_path: String,
    handler: impl Fn(serde_json::Value) + Send + 'static,
) -> Result<bool, anyhow::Error> {
    let package_manager = PackageManager::new();
    if package_manager.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to create package manager: {:?}",
            package_manager.err()
        ));
    }
    let package_manager = package_manager?;
    let package_path = HSTRING::from(raw_package_path.clone());
    let package_uri = Uri::CreateUri(&package_path);
    if package_uri.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to create URI: {:?}",
            package_uri.err()
        ));
    }
    let package_uri = package_uri?;
    let options = AddPackageOptions::new();
    if options.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to create AddPackageOptions: {:?}",
            options.err()
        ));
    }
    let options = options?;
    let _ = options.SetForceAppShutdown(true);
    let _ = options.SetRetainFilesOnFailure(true);
    let op = package_manager.AddPackageByUriAsync(&package_uri, &options);
    if op.is_err() {
        capture_and_return_err!(anyhow::anyhow!("Failed to add package: {:?}", op.err()));
    }
    let op = op?;
    let progress_sink = windows_future::AsyncOperationProgressHandler::new(
        move |_, progress: windows::core::Ref<DeploymentProgress>| {
            handler(serde_json::json!(progress.percentage));
            Ok(())
        },
    );
    let _ = op.SetProgress(&progress_sink);
    let res = op.get();
    if res.is_err() {
        capture_and_return_err!(anyhow::anyhow!("Failed to get result: {:?}", res.err()));
    }
    let res = res?;

    let is_registered = res.IsRegistered();
    if is_registered.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to check if registered: {:?}",
            is_registered.err()
        ));
    }
    let is_registered = is_registered?;

    let ex_code = res.ExtendedErrorCode();
    if ex_code.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to get extended error code: {:?}",
            ex_code.err()
        ));
    }
    let ex_code = ex_code?;

    if is_registered && ex_code.is_ok() {
        Ok(true)
    } else {
        let err_text = res.ErrorText();
        if err_text.is_err() {
            capture_and_return_err!(anyhow::anyhow!(
                "Failed to get error text: {:?}",
                err_text.err()
            ));
        }
        let err_text = err_text?;

        if ex_code == HRESULT(0x80070570u32 as i32) || ex_code == HRESULT(0x80070057u32 as i32) {
            let _ = std::fs::remove_file(raw_package_path);
        } else if ex_code == HRESULT(0x80073CFFu32 as i32) {
            rfd::MessageDialog::new()
                .set_title("错误")
                .set_description("部署包失败，请启用开发者模式\n\n点击确定将跳转到开发者设置页面")
                .set_level(rfd::MessageLevel::Error)
                .show();

            process::run(false, "ms-settings:developers", None::<&str>);
            return Ok(false);
        } else if ex_code == HRESULT(0) {
            capture_and_return_err!(anyhow::anyhow!(
                "Failed to add package: {:?}, HResult Last Error: {:?}",
                err_text,
                Error::from_win32()
            ));
        }

        capture_and_return_err!(anyhow::anyhow!(
            "Failed to add package: {:?}, HResult Error: {:?}",
            err_text,
            Error::from_hresult(ex_code)
        ));
    }
}

pub fn remove_package(package_family_name: String) -> Result<(), anyhow::Error> {
    let package_manager = PackageManager::new();
    if package_manager.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to create package manager: {:?}",
            package_manager.err()
        ));
    }
    let package_manager = package_manager?;

    let family_name = HSTRING::from(package_family_name);
    let packages = package_manager.FindPackagesByPackageFamilyName(&family_name);
    if packages.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to find packages by package family name: {:?}",
            packages.err()
        ));
    }
    let packages = packages?;

    let iter = packages.First();
    if iter.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to get first package iterator: {:?}",
            iter.err()
        ));
    }
    let iter = iter?;

    let has_current = iter.HasCurrent();
    if has_current.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to check if has current: {:?}",
            has_current.err()
        ));
    }
    if !has_current? {
        return Ok(());
    }

    let package = iter.Current();
    if package.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to get current package: {:?}",
            package.err()
        ));
    }
    let package = package?;

    let package_id = package.Id();
    if package_id.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to get package ID: {:?}",
            package_id.err()
        ));
    }
    let package_id = package_id?;

    let package_full_name = package_id.FullName();
    if package_full_name.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to get package full name: {:?}",
            package_full_name.err()
        ));
    }
    let package_full_name = package_full_name?;

    let op = package_manager
        .RemovePackageWithOptionsAsync(&package_full_name, RemovalOptions::RemoveForAllUsers);
    if op.is_err() {
        capture_and_return_err!(anyhow::anyhow!("Failed to remove package: {:?}", op.err()));
    }
    let op = op?;

    let res = op.get();
    if res.is_err() {
        capture_and_return_err!(anyhow::anyhow!("Failed to get result: {:?}", res.err()));
    }
    let res = res?;

    let status = op.Status();
    if status.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to get operation status: {:?}",
            status.err()
        ));
    }
    let status = status?;

    if status != AsyncStatus::Completed {
        let error_text = res.ErrorText();
        if error_text.is_err() {
            capture_and_return_err!(anyhow::anyhow!(
                "Failed to get error text: {:?}",
                error_text.err()
            ));
        }
        let error_text = error_text?;

        let extended_error_code = res.ExtendedErrorCode();
        if extended_error_code.is_err() {
            capture_and_return_err!(anyhow::anyhow!(
                "Failed to get extended error code: {:?}",
                extended_error_code.err()
            ));
        }
        let extended_error_code = extended_error_code?;

        if extended_error_code == HRESULT(0) {
            capture_and_return_err!(anyhow::anyhow!(
                "Failed to remove package: {:?}, HResult Last Error: {:?}",
                error_text,
                Error::from_win32()
            ));
        }

        capture_and_return_err!(anyhow::anyhow!(
            "Failed to remove package: {:?}, HResult Error: {:?}",
            error_text,
            Error::from_hresult(extended_error_code)
        ));
    }

    Ok(())
}
