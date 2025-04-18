use crate::utils::SentryCapturable;
use windows::{
    core::{Error, Result, HSTRING},
    Foundation::Uri,
    Management::Deployment::{AddPackageOptions, DeploymentProgress, PackageManager},
};

pub fn try_get_hutao_version() -> Result<Option<String>> {
    let package_manager = PackageManager::new();
    if package_manager.is_err_and_capture("Failed to create package manager") {
        return Ok(None);
    }
    let package_manager = package_manager?;

    let package_family_name = HSTRING::from("60568DGPStudio.SnapHutao_wbnnev551gwxy".to_string());
    let packages = package_manager.FindPackagesByPackageFamilyName(&package_family_name);
    if packages.is_err_and_capture("Failed to find packages by package family name") {
        return Ok(None);
    }
    let packages = packages?;
    let iter = packages.First();
    if iter.is_err_and_capture("Failed to get first package") {
        return Ok(None);
    }
    let iter = iter?;

    let has_current = iter.HasCurrent();
    if has_current.is_err_and_capture("Failed to check if has current") {
        return Ok(None);
    }
    let has_current = has_current?;

    if has_current {
        let package = iter.Current();
        if package.is_err_and_capture("Failed to get current package") {
            return Ok(None);
        }
        let package = package?;
        let id = package.Id();
        if id.is_err_and_capture("Failed to get package ID") {
            return Ok(None);
        }
        let id = id?;
        let version = id.Version();
        if version.is_err_and_capture("Failed to get package version") {
            return Ok(None);
        }
        let version = version?;
        Ok(Some(format!(
            "{}.{}.{}.{}",
            version.Major, version.Minor, version.Build, version.Revision
        )))
    } else {
        Ok(None)
    }
}

pub fn add_package(
    package_path: String,
    handler: impl Fn(serde_json::Value) + Send + 'static,
) -> Result<bool> {
    let package_manager = PackageManager::new();
    if package_manager.is_err_and_capture("Failed to create package manager") {
        return Err(package_manager.unwrap_err());
    }
    let package_manager = package_manager?;
    let package_path = HSTRING::from(package_path);
    let package_uri = Uri::CreateUri(&package_path);
    if package_uri.is_err_and_capture("Failed to create URI") {
        return Err(package_uri.unwrap_err());
    }
    let package_uri = package_uri?;
    let options = AddPackageOptions::new();
    if options.is_err_and_capture("Failed to create AddPackageOptions") {
        return Err(options.unwrap_err());
    }
    let options = options?;
    let _ = options.SetForceAppShutdown(true);
    let _ = options.SetRetainFilesOnFailure(true);
    let op = package_manager.AddPackageByUriAsync(&package_uri, &options);
    if op.is_err_and_capture("Failed to add package") {
        return Err(op.unwrap_err());
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
    if res.is_err_and_capture("Failed to get result") {
        return Err(res.unwrap_err());
    }
    let res = res?;

    let is_registered = res.IsRegistered();
    if is_registered.is_err_and_capture("Failed to check if registered") {
        return Err(is_registered.unwrap_err());
    }
    let is_registered = is_registered?;

    if is_registered {
        Ok(true)
    } else {
        let ex_code = res.ExtendedErrorCode();
        if ex_code.is_err_and_capture("Failed to get extended error code") {
            return Err(ex_code.unwrap_err());
        }
        let ex_code = ex_code?;

        Err(Error::from_hresult(ex_code))
    }
}
