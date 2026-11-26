use std::ffi::c_void;
use std::ptr;

use crate::models::hardware::FanProfile;

const ATKACPI_IOCTL_CONTROL: u32 = 0x0022_240C;
const METHOD_DSTS: u32 = 0x5354_5344; // "DSTS"
const METHOD_DEVS: u32 = 0x5356_4544; // "DEVS"

const GENERIC_READ_WRITE: u32 = 0xC000_0000;
const FILE_SHARE_READ_WRITE: u32 = 0x0000_0003;
const OPEN_EXISTING: u32 = 3;
const INVALID_HANDLE_VALUE: *mut c_void = (-1isize) as *mut c_void;

pub const DEVID_CPU_FAN: u32 = 0x0011_0013;
pub const DEVID_GPU_FAN: u32 = 0x0011_0014;
pub const DEVID_THROTTLE_VIVO: u32 = 0x0011_0019;
pub const DEVID_THROTTLE_ROG: u32 = 0x0012_0075;
pub const DEVID_BATTERY_LIMIT: u32 = 0x0012_0057;
pub const DEVID_KBD_BACKLIGHT: u32 = 0x0005_0021;
pub const DEVID_FN_LOCK: u32 = 0x0010_0023;
pub const DEVID_TOUCHPAD: u32 = 0x0010_0011;
pub const DEVID_PANEL_OD: u32 = 0x0005_0019;
pub const DEVID_DGPU_DISABLE: u32 = 0x0009_0020;

const PRESENCE_BIT: u32 = 0x0001_0000;

#[link(name = "kernel32")]
extern "system" {
    fn CreateFileW(
        lp_file_name: *const u16,
        dw_desired_access: u32,
        dw_share_mode: u32,
        lp_security_attributes: *mut c_void,
        dw_creation_disposition: u32,
        dw_flags_and_attributes: u32,
        h_template_file: *mut c_void,
    ) -> *mut c_void;

    fn DeviceIoControl(
        h_device: *mut c_void,
        dw_io_control_code: u32,
        lp_in_buffer: *const c_void,
        n_in_buffer_size: u32,
        lp_out_buffer: *mut c_void,
        n_out_buffer_size: u32,
        lp_bytes_returned: *mut u32,
        lp_overlapped: *mut c_void,
    ) -> i32;

    fn CloseHandle(h_object: *mut c_void) -> i32;
}

pub struct AtkAcpiClient;

impl AtkAcpiClient {
    pub fn is_available() -> bool {
        Self::open_handle().map(|h| unsafe { CloseHandle(h) }).is_some()
    }

    fn open_handle() -> Option<*mut c_void> {
        let wide: Vec<u16> = "\\\\.\\ATKACPI\0".encode_utf16().collect();
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ_WRITE,
                FILE_SHARE_READ_WRITE,
                ptr::null_mut(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(handle)
        }
    }

    /// Executes an ASUS ACPI `DSTS` (Device Status) read query.
    pub fn dsts(dev_id: u32) -> Option<u32> {
        let handle = Self::open_handle()?;
        let mut in_buf = [0u8; 12];
        in_buf[0..4].copy_from_slice(&METHOD_DSTS.to_le_bytes());
        in_buf[4..8].copy_from_slice(&4u32.to_le_bytes());
        in_buf[8..12].copy_from_slice(&dev_id.to_le_bytes());

        let mut out_buf = [0u8; 16];
        let mut returned = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                handle,
                ATKACPI_IOCTL_CONTROL,
                in_buf.as_ptr() as *const c_void,
                in_buf.len() as u32,
                out_buf.as_mut_ptr() as *mut c_void,
                out_buf.len() as u32,
                &mut returned,
                ptr::null_mut(),
            )
        };
        unsafe {
            CloseHandle(handle);
        }
        if ok != 0 {
            let val = u32::from_le_bytes([out_buf[0], out_buf[1], out_buf[2], out_buf[3]]);
            if val != 0 && val != 0xFFFF_FFFF {
                return Some(val);
            }
        }
        None
    }

    /// Executes an ASUS ACPI `DEVS` (Device Set) write call.
    pub fn devs(dev_id: u32, value: u32) -> Result<u32, String> {
        let handle = Self::open_handle()
            .ok_or_else(|| "Cannot open \\\\.\\ATKACPI (ASUS System Control Interface driver not found)".to_string())?;

        let mut in_buf = [0u8; 16];
        in_buf[0..4].copy_from_slice(&METHOD_DEVS.to_le_bytes());
        in_buf[4..8].copy_from_slice(&8u32.to_le_bytes());
        in_buf[8..12].copy_from_slice(&dev_id.to_le_bytes());
        in_buf[12..16].copy_from_slice(&value.to_le_bytes());

        let mut out_buf = [0u8; 16];
        let mut returned = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                handle,
                ATKACPI_IOCTL_CONTROL,
                in_buf.as_ptr() as *const c_void,
                in_buf.len() as u32,
                out_buf.as_mut_ptr() as *mut c_void,
                out_buf.len() as u32,
                &mut returned,
                ptr::null_mut(),
            )
        };
        unsafe {
            CloseHandle(handle);
        }

        if ok != 0 {
            Ok(u32::from_le_bytes([out_buf[0], out_buf[1], out_buf[2], out_buf[3]]))
        } else {
            Err(format!("ATKACPI DEVS(0x{dev_id:08X}, {value}) rejected by firmware"))
        }
    }

    pub fn is_present(dev_id: u32) -> bool {
        Self::dsts(dev_id)
            .map(|v| (v & PRESENCE_BIT) != 0 || (v & 0x00FF_0000) != 0)
            .unwrap_or(false)
    }

    pub fn read_fan_rpm(dev_id: u32) -> Option<u32> {
        let raw = Self::dsts(dev_id)?;
        let low = raw & 0xFFFF;
        // ASUS EC reports hundreds of RPM (e.g. 59 -> 5900 RPM) or direct RPM if > 255
        Some(if low <= 255 { low * 100 } else { low })
    }

    /// Reads Vivobook (`0x00110019`) or ROG/Zenbook (`0x00120075`) thermal policy.
    pub fn read_fan_profile() -> Option<(FanProfile, &'static str)> {
        if let Some(raw) = Self::dsts(DEVID_THROTTLE_VIVO) {
            let profile = match raw & 0xFF {
                0 => FanProfile::Standard,
                1 => FanProfile::Whisper,
                2 => FanProfile::Performance,
                _ => FanProfile::Standard,
            };
            return Some((profile, "ATKACPI Vivobook (0x00110019)"));
        }
        if let Some(raw) = Self::dsts(DEVID_THROTTLE_ROG) {
            let profile = FanProfile::from_asus_policy_code((raw & 0xFF) as u8)
                .unwrap_or(FanProfile::Standard);
            return Some((profile, "ATKACPI Thermal Policy (0x00120075)"));
        }
        None
    }

    pub fn write_fan_profile(profile: FanProfile) -> Result<String, String> {
        if Self::dsts(DEVID_THROTTLE_VIVO).is_some() {
            let code = match profile {
                FanProfile::Standard => 0,
                FanProfile::Whisper => 1,
                FanProfile::Performance => 2,
            };
            Self::devs(DEVID_THROTTLE_VIVO, code)?;
            return Ok(format!(
                "Set ASUS Vivobook Fan Profile to {} (ATKACPI 0x00110019 = {code})",
                profile.label()
            ));
        }
        if Self::dsts(DEVID_THROTTLE_ROG).is_some() {
            let code = profile.asus_policy_code() as u32;
            Self::devs(DEVID_THROTTLE_ROG, code)?;
            return Ok(format!(
                "Set ASUS Fan Profile to {} (ATKACPI 0x00120075 = {code})",
                profile.label()
            ));
        }
        Err("No supported ASUS thermal throttle ACPI endpoint found".to_string())
    }
}
