//! Version-pinned release download + extraction (embedded, no shell/external
//! tools). Asset names follow the release pipeline: `<pkg>-<TARGET>.tar.gz`.

use super::target::{
    binary_name, binary_path, cache_dir, expected_marker, marker_path, release_base, version,
    TARGET,
};

/// Version-pinned asset URL. Never `latest`.
pub fn release_url(pkg: &str) -> String {
    format!(
        "{}/v{}/{}-{}.tar.gz",
        release_base(),
        version(),
        pkg,
        TARGET
    )
}

/// Download `pkg` and install the expected binary into the cache, then write
/// its version marker. Only the named binary is extracted (no path traversal).
pub fn download_binary(pkg: &str, base: &str) -> Result<(), String> {
    let url = release_url(pkg);
    let response = ureq::get(&url)
        .call()
        .map_err(|e| format!("下载失败 {url}: {e}"))?;
    let bytes = response
        .into_body()
        .read_to_vec()
        .map_err(|e| format!("读取响应失败 {url}: {e}"))?;
    let name = binary_name(base);
    let dest = binary_path(base);
    std::fs::create_dir_all(cache_dir()).map_err(|e| format!("无法创建缓存目录: {e}"))?;

    let decoder = flate2::read::GzDecoder::new(&bytes[..]);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|e| format!("解压失败 {url}: {e}"))?;
    let mut found = false;
    for entry in entries {
        let mut entry = entry.map_err(|e| format!("解压失败 {url}: {e}"))?;
        let is_target = entry
            .path()
            .map(|p| p.file_name().and_then(|s| s.to_str()) == Some(name.as_str()))
            .unwrap_or(false);
        if is_target {
            entry
                .unpack(&dest)
                .map_err(|e| format!("写入失败 {}: {e}", dest.display()))?;
            found = true;
        }
    }
    if !found {
        return Err(format!("压缩包里没有 {name}（{url}）"));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
    }

    std::fs::write(marker_path(base), format!("{}\n", expected_marker()))
        .map_err(|e| format!("写入版本标记失败: {e}"))?;
    Ok(())
}
