//! WMI access layer for Acer Gaming (NitroSense) hardware.
//!
//! On AN515-57 the EC exposes Acer WMI methods through `root\WMI`.
//! This module wraps the WMI COM connection and provides typed helpers.

use crate::error::{Error, Result};

#[cfg(windows)]
pub fn connect() -> Result<::wmi::WMIConnection> {
    let com_lib = ::wmi::COMLibrary::new().map_err(|e| Error::Wmi(e.to_string()))?;
    let con = ::wmi::WMIConnection::new(com_lib).map_err(|e| Error::Wmi(e.to_string()))?;
    Ok(con)
}

#[cfg(windows)]
/// Connect to `root\WMI` namespace (Acer gaming classes live here).
pub fn connect_acer() -> Result<::wmi::WMIConnection> {
    let com_lib = ::wmi::COMLibrary::new().map_err(|e| Error::Wmi(e.to_string()))?;
    let con = ::wmi::WMIConnection::with_namespace_path("ROOT\\WMI", com_lib)
        .map_err(|e| Error::Wmi(e.to_string()))?;
    Ok(con)
}

#[cfg(not(windows))]
pub fn connect() -> Result<()> {
    Err(Error::Unsupported(
        "WMI is only available on Windows".into(),
    ))
}

#[cfg(not(windows))]
pub fn connect_acer() -> Result<()> {
    Err(Error::Unsupported(
        "WMI is only available on Windows".into(),
    ))
}
