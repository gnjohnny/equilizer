use std::ffi::c_void;

/// Windows COM GUID representation.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl GUID {
    pub const fn from_values(d1: u32, d2: u16, d3: u16, d4: [u8; 8]) -> Self {
        Self {
            data1: d1,
            data2: d2,
            data3: d3,
            data4: d4,
        }
    }
}

// CLSID_MMDeviceEnumerator: {BCDE0395-E52F-467C-8E3D-C4579291692E}
pub const CLSID_MM_DEVICE_ENUMERATOR: GUID = GUID::from_values(
    0xBCDE0395, 0xE52F, 0x467C, [0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E],
);

// IID_IMMDeviceEnumerator: {A95664D2-9614-4F35-A746-DE8DB63617E6}
pub const IID_IMM_DEVICE_ENUMERATOR: GUID = GUID::from_values(
    0xA95664D2, 0x9614, 0x4F35, [0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6],
);

// CLSID_CPolicyConfigClient: {870AF99C-171D-4F9E-AF0D-E63DF40C2BC9}
pub const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_values(
    0x870AF99C, 0x171D, 0x4F9E, [0xAF, 0x0D, 0xE6, 0x3D, 0xF4, 0x0C, 0x2B, 0xC9],
);

// IID_IPolicyConfig: {F8679F50-850A-41CF-9C72-430F290290C8}
pub const IID_POLICY_CONFIG: GUID = GUID::from_values(
    0xF8679F50, 0x850A, 0x41CF, [0x9C, 0x72, 0x43, 0x0F, 0x29, 0x02, 0x90, 0xC8],
);

// PKEY_Device_FriendlyName: {A45C254E-DF1C-4EFD-8020-67D146A850E0}, pid: 14
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PropertyKey {
    pub fmtid: GUID,
    pub pid: u32,
}

pub const PKEY_DEVICE_FRIENDLY_NAME: PropertyKey = PropertyKey {
    fmtid: GUID::from_values(
        0xA45C254E, 0xDF1C, 0x4EFD, [0x80, 0x20, 0x67, 0xD1, 0x46, 0xA8, 0x50, 0xE0],
    ),
    pid: 14,
};

#[repr(C)]
struct PropVariant {
    vt: u16,
    w_reserved1: u16,
    w_reserved2: u16,
    w_reserved3: u16,
    pwsz_val: *mut u16,
    _padding: [u8; 8],
}

#[repr(C)]
#[allow(non_snake_case)]
struct IUnknownVtbl {
    QueryInterface: unsafe extern "system" fn(this: *mut c_void, riid: *const GUID, ppv: *mut *mut c_void) -> i32,
    AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct IMMDeviceEnumeratorVtbl {
    parent: IUnknownVtbl,
    EnumAudioEndpoints: unsafe extern "system" fn(this: *mut c_void, data_flow: i32, state_mask: u32, devices: *mut *mut c_void) -> i32,
    GetDefaultAudioEndpoint: unsafe extern "system" fn(this: *mut c_void, data_flow: i32, role: i32, endpoint: *mut *mut c_void) -> i32,
    GetDevice: unsafe extern "system" fn(this: *mut c_void, id: *const u16, endpoint: *mut *mut c_void) -> i32,
    RegisterEndpointNotificationCallback: unsafe extern "system" fn(this: *mut c_void, client: *mut c_void) -> i32,
    UnregisterEndpointNotificationCallback: unsafe extern "system" fn(this: *mut c_void, client: *mut c_void) -> i32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct IMMDeviceCollectionVtbl {
    parent: IUnknownVtbl,
    GetCount: unsafe extern "system" fn(this: *mut c_void, count: *mut u32) -> i32,
    Item: unsafe extern "system" fn(this: *mut c_void, index: u32, device: *mut *mut c_void) -> i32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct IMMDeviceVtbl {
    parent: IUnknownVtbl,
    Activate: unsafe extern "system" fn(this: *mut c_void, id: *const GUID, cls_ctx: u32, activation_params: *mut c_void, interface_pointer: *mut *mut c_void) -> i32,
    OpenPropertyStore: unsafe extern "system" fn(this: *mut c_void, stgm_access: u32, properties: *mut *mut c_void) -> i32,
    GetId: unsafe extern "system" fn(this: *mut c_void, id: *mut *mut u16) -> i32,
    GetState: unsafe extern "system" fn(this: *mut c_void, state: *mut u32) -> i32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct IPropertyStoreVtbl {
    parent: IUnknownVtbl,
    GetCount: unsafe extern "system" fn(this: *mut c_void, count: *mut u32) -> i32,
    GetAt: unsafe extern "system" fn(this: *mut c_void, i_prop: u32, pkey: *mut c_void) -> i32,
    GetValue: unsafe extern "system" fn(this: *mut c_void, key: *const PropertyKey, pv: *mut PropVariant) -> i32,
    SetValue: unsafe extern "system" fn(this: *mut c_void, key: *const PropertyKey, pv: *const PropVariant) -> i32,
    Commit: unsafe extern "system" fn(this: *mut c_void) -> i32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct IPolicyConfigVtbl {
    parent: IUnknownVtbl,
    GetMixFormat: unsafe extern "system" fn(this: *mut c_void, psz_device_name: *const u16, pp_format: *mut c_void) -> i32,
    GetDeviceFormat: unsafe extern "system" fn(this: *mut c_void, psz_device_name: *const u16, b_default: i32, pp_format: *mut c_void) -> i32,
    ResetDeviceFormat: unsafe extern "system" fn(this: *mut c_void, psz_device_name: *const u16) -> i32,
    SetDeviceFormat: unsafe extern "system" fn(this: *mut c_void, psz_device_name: *const u16, p_endpoint_format: *mut c_void, p_mix_format: *mut c_void) -> i32,
    GetProcessingPeriod: unsafe extern "system" fn(this: *mut c_void, psz_device_name: *const u16, b_default: i32, pmft_default_period: *mut i64, pmft_minimum_period: *mut i64) -> i32,
    SetProcessingPeriod: unsafe extern "system" fn(this: *mut c_void, psz_device_name: *const u16, pmft_period: *mut i64) -> i32,
    GetShareMode: unsafe extern "system" fn(this: *mut c_void, psz_device_name: *const u16, p_mode: *mut c_void) -> i32,
    SetShareMode: unsafe extern "system" fn(this: *mut c_void, psz_device_name: *const u16, mode: *mut c_void) -> i32,
    GetPropertyValue: unsafe extern "system" fn(this: *mut c_void, psz_device_name: *const u16, key: *const PropertyKey, pv: *mut PropVariant) -> i32,
    SetPropertyValue: unsafe extern "system" fn(this: *mut c_void, psz_device_name: *const u16, key: *const PropertyKey, pv: *const PropVariant) -> i32,
    SetDefaultEndpoint: unsafe extern "system" fn(this: *mut c_void, wsz_device_id: *const u16, e_role: u32) -> i32,
    SetEndpointVisibility: unsafe extern "system" fn(this: *mut c_void, wsz_device_id: *const u16, is_visible: i32) -> i32,
}

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(pv_reserved: *mut c_void, dw_co_init: u32) -> i32;
    #[allow(dead_code)]
    fn CoUninitialize();
    fn CoCreateInstance(
        rclsid: *const GUID,
        p_unk_outer: *mut c_void,
        dw_cls_context: u32,
        riid: *const GUID,
        ppv: *mut *mut c_void,
    ) -> i32;
    fn CoTaskMemFree(pv: *mut c_void);
}

#[link(name = "oleaut32")]
extern "system" {
    fn PropVariantClear(pvar: *mut PropVariant) -> i32;
}

const CLSCTX_ALL: u32 = 23;
const COINIT_MULTITHREADED: u32 = 0;
const VT_LPWSTR: u16 = 31;

/// Converts a null-terminated UTF-16 pointer into a Rust String.
unsafe fn u16_ptr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    String::from_utf16_lossy(slice)
}

/// Helper struct for COM interface pointer wrapping and safe releasing.
struct ComPtr<T> {
    ptr: *mut c_void,
    _marker: std::marker::PhantomData<T>,
}

impl<T> ComPtr<T> {
    unsafe fn new(ptr: *mut c_void) -> Self {
        Self {
            ptr,
            _marker: std::marker::PhantomData,
        }
    }

    fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

    unsafe fn vtbl(&self) -> &T {
        let vtbl_ptr = *(self.ptr as *mut *mut T);
        &*vtbl_ptr
    }
}

impl<T> Drop for ComPtr<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                let unk = *(self.ptr as *mut *mut IUnknownVtbl);
                ((*unk).Release)(self.ptr);
            }
        }
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn SetConsoleCtrlHandler(
        handler_routine: Option<unsafe extern "system" fn(u32) -> i32>,
        add: i32,
    ) -> i32;
}

unsafe extern "system" fn console_ctrl_handler(_ctrl_type: u32) -> i32 {
    restore_emergency_endpoint();
    0 // Return FALSE so Windows default termination can still proceed
}

/// Registers a native Windows console control handler to restore physical audio endpoints
/// whenever the terminal process receives Ctrl+C (SIGINT), Ctrl+Break, or console closure.
pub fn init_console_ctrl_handler() {
    unsafe {
        SetConsoleCtrlHandler(Some(console_ctrl_handler), 1);
    }
}

static ORIGINAL_ENDPOINT_ID: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Caches the known original physical hardware endpoint ID for emergency recovery.
pub fn record_original_endpoint(endpoint_id: &str) {
    if let Ok(mut lock) = ORIGINAL_ENDPOINT_ID.lock() {
        if lock.is_none() {
            *lock = Some(endpoint_id.to_string());
        }
    }
}

/// Attempts an emergency restoration of the default playback device to the original
/// physical audio endpoint (or any non-virtual speaker) during panic or abnormal shutdown.
pub fn restore_emergency_endpoint() {
    let target = {
        ORIGINAL_ENDPOINT_ID
            .lock()
            .ok()
            .and_then(|lock| lock.clone())
            .unwrap_or_else(|| "Speakers".to_string())
    };
    let _ = switch_default_endpoint(&target);
}

/// Internal COM implementation for querying the default audio playback endpoint.
unsafe fn get_default_endpoint_inner() -> Result<(String, String), String> {
    let mut enumerator_ptr: *mut c_void = std::ptr::null_mut();
    let hr = CoCreateInstance(
        &CLSID_MM_DEVICE_ENUMERATOR,
        std::ptr::null_mut(),
        CLSCTX_ALL,
        &IID_IMM_DEVICE_ENUMERATOR,
        &mut enumerator_ptr,
    );
    if hr < 0 || enumerator_ptr.is_null() {
        return Err(format!("Failed to create MMDeviceEnumerator: 0x{:08X}", hr));
    }
    let enumerator = ComPtr::<IMMDeviceEnumeratorVtbl>::new(enumerator_ptr);

    let mut default_dev_ptr: *mut c_void = std::ptr::null_mut();
    // eRender = 0, eConsole = 0
    let hr = (enumerator.vtbl().GetDefaultAudioEndpoint)(enumerator.as_ptr(), 0, 0, &mut default_dev_ptr);
    if hr < 0 || default_dev_ptr.is_null() {
        return Err(format!("GetDefaultAudioEndpoint failed: 0x{:08X}", hr));
    }
    let default_dev = ComPtr::<IMMDeviceVtbl>::new(default_dev_ptr);

    let mut id_raw: *mut u16 = std::ptr::null_mut();
    (default_dev.vtbl().GetId)(default_dev.as_ptr(), &mut id_raw);
    let device_id = u16_ptr_to_string(id_raw);
    if !id_raw.is_null() {
        CoTaskMemFree(id_raw as *mut c_void);
    }

    let mut props_ptr: *mut c_void = std::ptr::null_mut();
    (default_dev.vtbl().OpenPropertyStore)(default_dev.as_ptr(), 0, &mut props_ptr);
    let props = ComPtr::<IPropertyStoreVtbl>::new(props_ptr);

    let mut pv = std::mem::zeroed::<PropVariant>();
    (props.vtbl().GetValue)(props.as_ptr(), &PKEY_DEVICE_FRIENDLY_NAME, &mut pv);
    let name = if pv.vt == VT_LPWSTR && !pv.pwsz_val.is_null() {
        u16_ptr_to_string(pv.pwsz_val)
    } else {
        device_id.clone()
    };
    let _ = PropVariantClear(&mut pv);

    Ok((name, device_id))
}

/// Queries the Windows default audio playback endpoint name and endpoint ID.
/// Executes on a dedicated worker thread with isolated COM MTA apartment,
/// leaving the main UI thread clean for tao's STA OleInitialize.
pub fn get_default_endpoint() -> Result<(String, String), String> {
    std::thread::spawn(move || unsafe {
        let hr = CoInitializeEx(std::ptr::null_mut(), COINIT_MULTITHREADED);
        let res = get_default_endpoint_inner();
        if hr >= 0 {
            CoUninitialize();
        }
        res
    })
    .join()
    .map_err(|_| "Audio endpoint query thread panicked".to_string())?
}

/// Internal COM implementation for switching the default playback endpoint.
unsafe fn switch_default_endpoint_inner(target: &str) -> Result<String, String> {
    let mut enumerator_ptr: *mut c_void = std::ptr::null_mut();
    let hr = CoCreateInstance(
        &CLSID_MM_DEVICE_ENUMERATOR,
        std::ptr::null_mut(),
        CLSCTX_ALL,
        &IID_IMM_DEVICE_ENUMERATOR,
        &mut enumerator_ptr,
    );
    if hr < 0 || enumerator_ptr.is_null() {
        return Err(format!("Failed to create MMDeviceEnumerator: 0x{:08X}", hr));
    }
    let enumerator = ComPtr::<IMMDeviceEnumeratorVtbl>::new(enumerator_ptr);

    let mut collection_ptr: *mut c_void = std::ptr::null_mut();
    // eRender = 0, DEVICE_STATE_ACTIVE = 1
    let hr = (enumerator.vtbl().EnumAudioEndpoints)(enumerator.as_ptr(), 0, 1, &mut collection_ptr);
    if hr < 0 || collection_ptr.is_null() {
        return Err(format!("EnumAudioEndpoints failed: 0x{:08X}", hr));
    }
    let collection = ComPtr::<IMMDeviceCollectionVtbl>::new(collection_ptr);

    let mut count = 0u32;
    (collection.vtbl().GetCount)(collection.as_ptr(), &mut count);

    let mut matched_id: Option<String> = None;
    let mut matched_name: Option<String> = None;

    let target_lower = target.to_lowercase();

    for i in 0..count {
        let mut dev_ptr: *mut c_void = std::ptr::null_mut();
        if (collection.vtbl().Item)(collection.as_ptr(), i, &mut dev_ptr) >= 0 && !dev_ptr.is_null() {
            let dev = ComPtr::<IMMDeviceVtbl>::new(dev_ptr);

            let mut id_raw: *mut u16 = std::ptr::null_mut();
            (dev.vtbl().GetId)(dev.as_ptr(), &mut id_raw);
            let id_str = u16_ptr_to_string(id_raw);
            if !id_raw.is_null() {
                CoTaskMemFree(id_raw as *mut c_void);
            }

            let mut props_ptr: *mut c_void = std::ptr::null_mut();
            (dev.vtbl().OpenPropertyStore)(dev.as_ptr(), 0, &mut props_ptr);
            let props = ComPtr::<IPropertyStoreVtbl>::new(props_ptr);

            let mut pv = std::mem::zeroed::<PropVariant>();
            (props.vtbl().GetValue)(props.as_ptr(), &PKEY_DEVICE_FRIENDLY_NAME, &mut pv);
            let friendly_name = if pv.vt == VT_LPWSTR && !pv.pwsz_val.is_null() {
                u16_ptr_to_string(pv.pwsz_val)
            } else {
                id_str.clone()
            };
            let _ = PropVariantClear(&mut pv);

            if id_str.eq_ignore_ascii_case(target) || friendly_name.to_lowercase().contains(&target_lower) {
                matched_id = Some(id_str);
                matched_name = Some(friendly_name);
                break;
            }
        }
    }

    let id = matched_id.ok_or_else(|| format!("Target audio device '{}' not found", target))?;
    let name = matched_name.unwrap_or_else(|| id.clone());

    // Convert target endpoint ID to UTF-16
    let wide_id: Vec<u16> = id.encode_utf16().chain(std::iter::once(0)).collect();

    let mut policy_config_ptr: *mut c_void = std::ptr::null_mut();
    let hr = CoCreateInstance(
        &CLSID_POLICY_CONFIG_CLIENT,
        std::ptr::null_mut(),
        CLSCTX_ALL,
        &IID_POLICY_CONFIG,
        &mut policy_config_ptr,
    );
    if hr < 0 || policy_config_ptr.is_null() {
        return Err(format!("Failed to create CPolicyConfigClient: 0x{:08X}", hr));
    }
    let policy_config = ComPtr::<IPolicyConfigVtbl>::new(policy_config_ptr);

    // eConsole = 0, eMultimedia = 1, eCommunications = 2
    for role in 0..3 {
        let res = (policy_config.vtbl().SetDefaultEndpoint)(policy_config.as_ptr(), wide_id.as_ptr(), role);
        if res < 0 {
            eprintln!("Warning: SetDefaultEndpoint role {} returned 0x{:08X}", role, res);
        }
    }

    Ok(name)
}

/// Switches Windows default playback endpoint to target device matching friendly name substring or exact ID.
/// Automatically applies to all three audio roles (Console, Multimedia, Communications).
/// Executes on a dedicated worker thread with isolated COM MTA apartment,
/// leaving the main UI thread clean for tao's STA OleInitialize.
pub fn switch_default_endpoint(target: &str) -> Result<String, String> {
    let target = target.to_string();
    std::thread::spawn(move || unsafe {
        let hr = CoInitializeEx(std::ptr::null_mut(), COINIT_MULTITHREADED);
        let res = switch_default_endpoint_inner(&target);
        if hr >= 0 {
            CoUninitialize();
        }
        res
    })
    .join()
    .map_err(|_| "Audio endpoint switch thread panicked".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_endpoint() {
        let res = get_default_endpoint();
        if let Ok((name, id)) = res {
            assert!(!name.is_empty());
            assert!(!id.is_empty());
            println!("Current default audio endpoint: '{}' [{}]", name, id);
        } else {
            eprintln!("get_default_endpoint skipped on headless environment: {:?}", res);
        }
    }

    #[test]
    fn test_switch_endpoint_invalid_target() {
        let res = switch_default_endpoint("NonExistentAudioEndpoint_XYZ_12345");
        assert!(res.is_err());
    }
}
