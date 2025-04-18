use crate::utils::SentryCapturable;
use std::ffi::CString;
use tokio_util::bytes::Bytes;
use windows::Win32::Foundation::GetLastError;
use windows::{core::s, Win32::Security::Cryptography::*};

pub async fn find_certificate(subject: &str) -> Result<bool, String> {
    unsafe {
        let store_name = s!("Root").as_ptr();
        let h_store = CertOpenStore(
            CERT_STORE_PROV_SYSTEM_A,
            X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
            None,
            CERT_OPEN_STORE_FLAGS(CERT_SYSTEM_STORE_LOCAL_MACHINE),
            Some(store_name as _),
        );

        if h_store.is_err_and_capture("Failed to open store") {
            return Err(format!("Failed to open store: {:?}", h_store.err()));
        }

        let h_store = h_store.unwrap();
        if h_store.is_invalid() {
            sentry::capture_message(
                format!("Failed to open store: {:?}", GetLastError()).as_str(),
                sentry::Level::Error,
            );
            return Err("Failed to open store".to_string());
        }

        let mut cert: *mut CERT_CONTEXT = std::ptr::null_mut();
        cert = CertEnumCertificatesInStore(h_store, Some(cert));

        while !cert.is_null() {
            let subj = cert.read().pCertInfo.read().Subject;
            let mut buffer = vec![0u8; 256];
            let len = CertNameToStrA(
                X509_ASN_ENCODING,
                &subj,
                CERT_SIMPLE_NAME_STR,
                Some(&mut buffer),
            );
            buffer.resize(len as usize, 0);
            let sub_str = CString::from_vec_unchecked(buffer);
            if sub_str.to_string_lossy().contains(subject) {
                break;
            }

            cert = CertEnumCertificatesInStore(h_store, Some(cert));
        }

        let found = !cert.is_null();
        if found {
            let _ = CertFreeCertificateContext(Some(cert));
        }

        let _ = CertCloseStore(Some(h_store), 0);
        Ok(found)
    }
}

pub async fn install_certificate(
    content: Bytes,
    window: tauri::WebviewWindow,
) -> Result<bool, String> {
    unsafe {
        let store_name = s!("ROOT").as_ptr();
        let h_store = CertOpenStore(
            CERT_STORE_PROV_SYSTEM_A,
            X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
            None,
            CERT_OPEN_STORE_FLAGS(CERT_SYSTEM_STORE_LOCAL_MACHINE),
            Some(store_name as _),
        );

        if h_store.is_err_and_capture("Failed to open store") {
            return Err(format!("Failed to open store: {:?}", h_store.err()));
        }

        let h_store = h_store.unwrap();
        if h_store.is_invalid() {
            sentry::capture_message(
                format!("Failed to open store: {:?}", GetLastError()).as_str(),
                sentry::Level::Error,
            );
            return Err("Failed to open store".to_string());
        }

        let cert = CertCreateCertificateContext(X509_ASN_ENCODING, &content);
        if cert.is_null() {
            sentry::capture_message(
                format!("Failed to create certificate context: {:?}", GetLastError()).as_str(),
                sentry::Level::Error,
            );
            return Err("Failed to create certificate context".to_string());
        }

        let title = "安装证书".to_string();
        let message = r#"正在向 本地计算机/受信任的根证书颁发机构 添加证书
如果你无法理解弹窗中的文本，请点击 [是]

Adding certificate to LocalMachine/ThirdParty Root CA store,
please click [yes] on the [Security Waring] dialog

关于更多安全信息，请查看下方的网址
For more security information, please visit the url down below
https://support.globalsign.com/ca-certificates/root-certificates/globalsign-root-certificates"#
            .to_string();

        rfd::MessageDialog::new()
            .set_title(&title)
            .set_description(&message)
            .set_level(rfd::MessageLevel::Info)
            .set_parent(&window)
            .show();

        let add_res = CertAddCertificateContextToStore(
            Some(h_store),
            cert,
            CERT_STORE_ADD_REPLACE_EXISTING,
            None,
        );

        let _ = CertFreeCertificateContext(Some(cert));

        if add_res.is_err_and_capture("Failed to add certificate to store") {
            return Err(format!(
                "Failed to add certificate to store: {:?}",
                add_res.err()
            ));
        }

        let _ = CertCloseStore(Some(h_store), 0);
        Ok(true)
    }
}
