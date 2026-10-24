use crate::collector::CollectorError;

pub trait RegistryReader: Send + Sync {
    fn read_string(&self, key: &str, name: &str) -> Result<Option<String>, CollectorError>;

    fn read_u32(&self, key: &str, name: &str) -> Result<Option<u32>, CollectorError>;
}

#[must_use]
pub fn default_registry_reader() -> std::sync::Arc<dyn RegistryReader> {
    std::sync::Arc::new(PlatformRegistryReader)
}

struct PlatformRegistryReader;

#[cfg(windows)]
impl RegistryReader for PlatformRegistryReader {
    fn read_string(&self, key: &str, name: &str) -> Result<Option<String>, CollectorError> {
        match read_raw(key, name, REG_SZ_MASK) {
            Some(bytes) => {
                let units: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                    .take_while(|unit| *unit != 0)
                    .collect();
                Ok(Some(String::from_utf16_lossy(&units)))
            }
            None => Ok(None),
        }
    }

    fn read_u32(&self, key: &str, name: &str) -> Result<Option<u32>, CollectorError> {
        match read_raw(key, name, REG_DWORD_MASK) {
            Some(bytes) if bytes.len() == 4 => Ok(Some(u32::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3],
            ]))),
            _ => Ok(None),
        }
    }
}

#[cfg(windows)]
const REG_SZ_MASK: windows::Win32::System::Registry::REG_ROUTINE_FLAGS =
    windows::Win32::System::Registry::RRF_RT_REG_SZ;
#[cfg(windows)]
const REG_DWORD_MASK: windows::Win32::System::Registry::REG_ROUTINE_FLAGS =
    windows::Win32::System::Registry::RRF_RT_REG_DWORD;

#[cfg(windows)]
fn read_raw(
    key: &str,
    name: &str,
    mask: windows::Win32::System::Registry::REG_ROUTINE_FLAGS,
) -> Option<Vec<u8>> {
    use windows::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, REG_VALUE_TYPE, RRF_SUBKEY_WOW6464KEY, RegGetValueW,
    };
    use windows::core::PCWSTR;

    let subkey: Vec<u16> = key.encode_utf16().chain(std::iter::once(0)).collect();
    let value_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut value_type = REG_VALUE_TYPE::default();
    let mut buffer = [0u8; 2048];
    let mut size = u32::try_from(buffer.len()).expect("buffer fits u32");

    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            mask | RRF_SUBKEY_WOW6464KEY,
            Some(std::ptr::from_mut(&mut value_type)),
            Some(buffer.as_mut_ptr().cast()),
            Some(std::ptr::from_mut(&mut size)),
        )
    };
    if status != windows::Win32::Foundation::WIN32_ERROR(0) {
        return None;
    }
    let size = size as usize;
    Some(buffer[..size.min(buffer.len())].to_vec())
}

#[cfg(not(windows))]
impl RegistryReader for PlatformRegistryReader {
    fn read_string(&self, _key: &str, _name: &str) -> Result<Option<String>, CollectorError> {
        Ok(None)
    }

    fn read_u32(&self, _key: &str, _name: &str) -> Result<Option<u32>, CollectorError> {
        Ok(None)
    }
}
