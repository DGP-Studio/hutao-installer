use windows::{
    core::{Error, Result, HSTRING,HRESULT},
    Foundation::{AsyncOperationProgressHandler, Uri},
    Management::Deployment::{
        AddPackageOptions, DeploymentProgress, DeploymentResult, PackageManager,
    },
};

pub async fn try_get_hutao_version() -> Result<Option<String>> {
    let package_manager = PackageManager::new()?;
    let package_family_name = HSTRING::from("60568DGPStudio.SnapHutao_wbnnev551gwxy".to_string());
    let packages = package_manager.FindPackagesByPackageFamilyName(&package_family_name)?;
    let iter = packages.First()?;

    if iter.HasCurrent()? {
        let package = iter.Current()?;
        let id = package.Id()?;
        let version = id.Version()?;
        Ok(Some(format!(
            "{}.{}.{}.{}",
            version.Major, version.Minor, version.Build, version.Revision
        )))
    } else {
        Ok(None)
    }
}

pub async fn add_package(
    package_path: String,
    handler: impl Fn(serde_json::Value) + std::marker::Send + 'static,
) -> Result<bool> {
    let package_manager = PackageManager::new()?;
    let package_path = HSTRING::from(package_path);
    let package_uri = Uri::CreateUri(&package_path)?;
    let options = AddPackageOptions::new()?;
    let _ = options.SetForceAppShutdown(true);
    let _ = options.SetRetainFilesOnFailure(true);
    let op: windows::Foundation::IAsyncOperationWithProgress<
        windows::Management::Deployment::DeploymentResult,
        windows::Management::Deployment::DeploymentProgress,
    > = package_manager.AddPackageByUriAsync(&package_uri, &options)?;
    let progress_sink: AsyncOperationProgressHandler<DeploymentResult, DeploymentProgress> =
        AsyncOperationProgressHandler::new(move |_, progress: &DeploymentProgress| {
            dbg!(progress.percentage);
            let _ = handler(serde_json::json!(progress.percentage));
            Ok(())
        });
    let _ = op.SetProgress(&progress_sink);
    let res = op.get()?;

    if res.IsRegistered()? {
       Ok(true)
    } else {
        Err(Error::from_hresult(res.ExtendedErrorCode()?))
    }
}
