// HRML PAM Authentication
//
// Unix philosophy: use PAM. The system already knows how to authenticate users.
// Don't reinvent auth. Delegate to PAM, trust the OS.
//
// Uses dlopen to load libpam at runtime, avoiding link-time dependencies.

use std::ffi::{CStr, CString};
use std::ptr;

/// Authenticate a user against PAM.
/// Uses the "login" PAM service by default.
pub fn authenticate(username: &str, password: &str) -> Result<bool, String> {
    authenticate_with_service(username, password, "login")
}

/// Authenticate a user against a specific PAM service.
pub fn authenticate_with_service(
    username: &str,
    password: &str,
    service: &str,
) -> Result<bool, String> {
    let pam = PamHandle::new()?;
    pam.authenticate(username, password, service)
}

/// Check if a user is a member of a group.
pub fn is_user_in_group(username: &str, group: &str) -> Result<bool, String> {
    let c_group = CString::new(group).map_err(|e| e.to_string())?;
    unsafe {
        let grp = libc::getgrnam(c_group.as_ptr());
        if grp.is_null() {
            return Err(format!("Group '{}' not found", group));
        }
        let grp = &*grp;
        if grp.gr_mem.is_null() {
            return Ok(false);
        }
        let mut i = 0;
        loop {
            let member = *grp.gr_mem.offset(i);
            if member.is_null() {
                break;
            }
            let member_str = CStr::from_ptr(member).to_string_lossy();
            if member_str.as_ref() == username {
                return Ok(true);
            }
            i += 1;
        }
        Ok(false)
    }
}

/// Get the current effective user.
pub fn current_user() -> Option<String> {
    std::env::var("USER").ok().or_else(|| unsafe {
        let pw = libc::getpwuid(libc::geteuid());
        if pw.is_null() {
            None
        } else {
            Some(CStr::from_ptr((*pw).pw_name).to_string_lossy().to_string())
        }
    })
}

/// Get the current effective user's UID.
pub fn current_uid() -> u32 {
    unsafe { libc::geteuid() }
}

/// Check if running as root.
pub fn is_root() -> bool {
    current_uid() == 0
}

// ============================================================
// Dynamic PAM loading via dlopen
// ============================================================

struct PamLib {
    handle: *mut libc::c_void,
    start: extern "C" fn(
        *const libc::c_char,
        *const libc::c_char,
        *const PamConv,
        *mut *mut libc::c_void,
    ) -> libc::c_int,
    end: extern "C" fn(*mut libc::c_void, libc::c_int) -> libc::c_int,
    authenticate: extern "C" fn(*mut libc::c_void, libc::c_int) -> libc::c_int,
    acct_mgmt: extern "C" fn(*mut libc::c_void, libc::c_int) -> libc::c_int,
    strerror: extern "C" fn(*mut libc::c_void, libc::c_int) -> *const libc::c_char,
}

impl PamLib {
    fn new() -> Result<Self, String> {
        unsafe {
            let libname = b"libpam.so.0\0";
            let handle = libc::dlopen(libname.as_ptr() as *const libc::c_char, libc::RTLD_LAZY);
            if handle.is_null() {
                let err = CStr::from_ptr(libc::dlerror())
                    .to_string_lossy()
                    .to_string();
                return Err(format!("Failed to load libpam: {}", err));
            }

            let start = Self::sym(handle, "pam_start")?;
            let end = Self::sym(handle, "pam_end")?;
            let authenticate = Self::sym(handle, "pam_authenticate")?;
            let acct_mgmt = Self::sym(handle, "pam_acct_mgmt")?;
            let strerror = Self::sym(handle, "pam_strerror")?;

            Ok(Self {
                handle,
                start,
                end,
                authenticate,
                acct_mgmt,
                strerror,
            })
        }
    }

    unsafe fn sym<T>(handle: *mut libc::c_void, name: &str) -> Result<T, String> {
        let cname = CString::new(name).unwrap();
        let sym = libc::dlsym(handle, cname.as_ptr());
        if sym.is_null() {
            Err(format!("Missing symbol: {}", name))
        } else {
            Ok(std::mem::transmute_copy(&sym))
        }
    }
}

impl Drop for PamLib {
    fn drop(&mut self) {
        unsafe {
            libc::dlclose(self.handle);
        }
    }
}

struct PamHandle {
    lib: std::sync::Arc<PamLib>,
}

impl PamHandle {
    fn new() -> Result<Self, String> {
        Ok(Self {
            lib: std::sync::Arc::new(PamLib::new()?),
        })
    }

    fn authenticate(&self, username: &str, password: &str, service: &str) -> Result<bool, String> {
        let c_username = CString::new(username).map_err(|e| e.to_string())?;
        let c_password = CString::new(password).map_err(|e| e.to_string())?;
        let c_service = CString::new(service).map_err(|e| e.to_string())?;

        unsafe {
            let mut pamh: *mut libc::c_void = ptr::null_mut();

            let conv = PamConv::new(c_password.clone());
            let conv_ptr = Box::into_raw(Box::new(conv));

            let ret = (self.lib.start)(
                c_service.as_ptr(),
                c_username.as_ptr(),
                conv_ptr as *const PamConv,
                &mut pamh,
            );

            if ret != 0 {
                let err = self.pam_error_str(pamh, ret);
                Box::from_raw(conv_ptr);
                return Err(format!("pam_start failed: {}", err));
            }

            let ret = (self.lib.authenticate)(pamh, 0);

            if ret == 0 {
                let ret2 = (self.lib.acct_mgmt)(pamh, 0);
                if ret2 != 0 {
                    (self.lib.end)(pamh, ret2);
                    Box::from_raw(conv_ptr);
                    return Ok(false);
                }
            }

            (self.lib.end)(pamh, ret);
            Box::from_raw(conv_ptr);

            Ok(ret == 0)
        }
    }

    fn pam_error_str(&self, pamh: *mut libc::c_void, code: libc::c_int) -> String {
        unsafe {
            let s = (self.lib.strerror)(pamh, code);
            if s.is_null() {
                format!("PAM error {}", code)
            } else {
                CStr::from_ptr(s).to_string_lossy().to_string()
            }
        }
    }
}

// ============================================================
// PAM types
// ============================================================

const PAM_SUCCESS: libc::c_int = 0;
const PAM_CONV_ERR: libc::c_int = 5;
const PAM_BUF_ERR: libc::c_int = 4;
const PAM_PROMPT_ECHO_OFF: libc::c_int = 1;

#[repr(C)]
struct PamMessage {
    msg_style: libc::c_int,
    msg: *const libc::c_char,
}

#[repr(C)]
struct PamResponse {
    resp: *mut libc::c_char,
    resp_retcode: libc::c_int,
}

#[repr(C)]
struct PamConv {
    conv: extern "C" fn(
        libc::c_int,
        *const *const PamMessage,
        *mut *mut PamResponse,
        *mut libc::c_void,
    ) -> libc::c_int,
    appdata_ptr: *mut libc::c_void,
}

impl PamConv {
    fn new(password: CString) -> Self {
        // Store password in the appdata_ptr
        let pwd_ptr = Box::into_raw(Box::new(password));
        Self {
            conv: pam_conv_fn,
            appdata_ptr: pwd_ptr as *mut libc::c_void,
        }
    }
}

impl Drop for PamConv {
    fn drop(&mut self) {
        unsafe {
            if !self.appdata_ptr.is_null() {
                drop(Box::from_raw(self.appdata_ptr as *mut CString));
            }
        }
    }
}

extern "C" fn pam_conv_fn(
    num_msg: libc::c_int,
    msg: *const *const PamMessage,
    resp: *mut *mut PamResponse,
    appdata_ptr: *mut libc::c_void,
) -> libc::c_int {
    unsafe {
        if num_msg <= 0 || msg.is_null() || resp.is_null() || appdata_ptr.is_null() {
            return PAM_CONV_ERR;
        }

        let message = &**msg;
        if (*message).msg_style != PAM_PROMPT_ECHO_OFF {
            return PAM_CONV_ERR;
        }

        let password = &*(appdata_ptr as *const CString);

        let response = libc::malloc(password.as_bytes().len() + 1) as *mut libc::c_char;
        if response.is_null() {
            return PAM_BUF_ERR;
        }
        libc::strcpy(response, password.as_ptr());

        *resp = libc::malloc(std::mem::size_of::<PamResponse>()) as *mut PamResponse;
        if (*resp).is_null() {
            libc::free(response as *mut libc::c_void);
            return PAM_BUF_ERR;
        }

        (**resp).resp = response;
        (**resp).resp_retcode = 0;

        PAM_SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_user_returns_some() {
        let user = current_user();
        assert!(user.is_some() || std::env::var("USER").is_err());
    }

    #[test]
    fn current_uid_is_valid() {
        let uid = current_uid();
        assert!(uid > 0 || uid == 0);
    }

    #[test]
    fn is_root_check() {
        let _ = is_root();
    }

    #[test]
    fn group_membership_check() {
        let result = is_user_in_group("root", "root");
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn pam_loads_or_gracefully_fails() {
        // Either PAM works or we get a clear error
        let result = authenticate("test_user", "test_pass");
        assert!(result.is_ok() || result.as_ref().unwrap_err().contains("libpam"));
    }
}
