#![doc = include_str!("../README.md")]
#![no_std]
#![allow(clippy::similar_names)]

extern crate alloc;

use alloc::{ffi::CString, string::ToString};
use core::{
    ffi::{CStr, c_char, c_int, c_void},
    ptr, slice,
};

use sqlite_wasm_rs::{
    SQLITE_BLOB, SQLITE_DETERMINISTIC, SQLITE_INNOCUOUS, SQLITE_OK, SQLITE_TEXT, SQLITE_TRANSIENT,
    SQLITE_UTF8, sqlite3, sqlite3_api_routines, sqlite3_context, sqlite3_create_function_v2,
    sqlite3_result_blob, sqlite3_result_null, sqlite3_result_text, sqlite3_value,
    sqlite3_value_blob, sqlite3_value_bytes, sqlite3_value_text, sqlite3_value_type,
};
use uuid::Uuid;

/// Helper function to parse a UUID from an SQLite argument value.
///
/// Supports two input formats:
/// - **TEXT**: A 32 (hex) or 36 (hyphenated) character string string.
/// - **BLOB**: A raw 16-byte UUID buffer.
///
/// # Arguments
/// * `argv` - Pointer to the array of sqlite3_value pointers.
/// * `index` - Index of the argument to check.
///
/// # Returns
/// * `Option<Uuid>` - The parsed UUID if valid, or `None` if invalid/wrong
///   type.
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers from `argv`.
unsafe fn parse_uuid_arg(argv: *mut *mut sqlite3_value, index: usize) -> Option<Uuid> {
    // SAFETY: Caller must ensure `argv` has at least `index + 1` elements
    let arg = unsafe { *argv.add(index) };
    let ty = unsafe { sqlite3_value_type(arg) };

    match ty {
        SQLITE_TEXT => {
            let text_ptr = unsafe { sqlite3_value_text(arg) };
            if text_ptr.is_null() {
                return None;
            }
            let c_str = unsafe { CStr::from_ptr(text_ptr.cast::<c_char>()) };
            let s = c_str.to_str().ok()?;
            Uuid::parse_str(s).ok()
        }
        SQLITE_BLOB => {
            let blob_ptr = unsafe { sqlite3_value_blob(arg) };
            let bytes = unsafe { sqlite3_value_bytes(arg) };
            if bytes == 16 && !blob_ptr.is_null() {
                let s = unsafe { slice::from_raw_parts(blob_ptr.cast::<u8>(), 16) };
                let array: [u8; 16] = s.try_into().ok()?;
                Some(Uuid::from_bytes(array))
            } else {
                None
            }
        }
        _ => None,
    }
}

// --- SQL Functions (UUIDv7) ---

/// SQL Function: `uuid7()`
///
/// Generates a UUIDv7 (time-ordered) and returns it as a canonical 36-character
/// string.
unsafe extern "C" fn uuid7_func(
    ctx: *mut sqlite3_context,
    _argc: c_int,
    _argv: *mut *mut sqlite3_value,
) {
    let u = Uuid::now_v7();
    let s = u.to_string(); // canonical 36-char string
    let c_str = CString::new(s).unwrap();
    unsafe {
        sqlite3_result_text(ctx, c_str.as_ptr(), -1, SQLITE_TRANSIENT());
    }
}

/// SQL Function: `uuid7_blob()`
unsafe extern "C" fn uuid7_blob_func(
    ctx: *mut sqlite3_context,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) {
    if argc == 0 {
        let u = Uuid::now_v7();
        let bytes = u.as_bytes();
        unsafe {
            sqlite3_result_blob(ctx, bytes.as_ptr().cast::<c_void>(), 16, SQLITE_TRANSIENT());
        }
        return;
    }

    if let Some(u) = unsafe { parse_uuid_arg(argv, 0) } {
        let bytes = u.as_bytes();
        unsafe {
            sqlite3_result_blob(ctx, bytes.as_ptr().cast::<c_void>(), 16, SQLITE_TRANSIENT());
        }
    } else {
        unsafe {
            sqlite3_result_null(ctx);
        }
    }
}

// --- SQL Functions (UUIDv4) ---

/// Implementation of the `uuid()` SQL function.
unsafe extern "C" fn uuid_func(
    ctx: *mut sqlite3_context,
    _argc: c_int,
    _argv: *mut *mut sqlite3_value,
) {
    let u = Uuid::new_v4();
    let s = u.to_string();
    let c_str = CString::new(s).unwrap();
    unsafe {
        sqlite3_result_text(ctx, c_str.as_ptr(), -1, SQLITE_TRANSIENT());
    }
}

/// Implementation of the `uuid_str(X)` SQL function.
unsafe extern "C" fn uuid_str_func(
    ctx: *mut sqlite3_context,
    _argc: c_int,
    argv: *mut *mut sqlite3_value,
) {
    if let Some(u) = unsafe { parse_uuid_arg(argv, 0) } {
        let s = u.to_string();
        let c_str = CString::new(s).unwrap();
        unsafe {
            sqlite3_result_text(ctx, c_str.as_ptr(), -1, SQLITE_TRANSIENT());
        }
    } else {
        unsafe {
            sqlite3_result_null(ctx);
        }
    }
}

/// Implementation of the `uuid_blob(X)` SQL function.
unsafe extern "C" fn uuid_blob_func(
    ctx: *mut sqlite3_context,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) {
    if argc == 0 {
        let u = Uuid::new_v4();
        let bytes = u.as_bytes();
        unsafe {
            sqlite3_result_blob(ctx, bytes.as_ptr().cast::<c_void>(), 16, SQLITE_TRANSIENT());
        }
        return;
    }

    if let Some(u) = unsafe { parse_uuid_arg(argv, 0) } {
        let bytes = u.as_bytes();
        unsafe {
            sqlite3_result_blob(ctx, bytes.as_ptr().cast::<c_void>(), 16, SQLITE_TRANSIENT());
        }
    } else {
        unsafe {
            sqlite3_result_null(ctx);
        }
    }
}

// --- Extension Entry Point ---

/// SQLite Extension Entry Point: `sqlite3_uuid_init`
///
/// Registers the following SQL functions with the SQLite database connection:
/// - `uuid`
/// - `uuid_str`
/// - `uuid_blob`
/// - `uuid7`
/// - `uuid7_blob`
///
/// # Arguments
/// * `db` - The SQLite database connection.
/// * `_pz_err_msg` - Pointer to error message pointer (unused).
/// * `_p_api` - Pointer to SQLite API (unused, assuming linked implementation).
///
/// # Returns
/// * `SQLITE_OK` on success, or an error code.
///
/// # Safety
/// This function is unsafe because it interacts with raw SQLite pointers.
/// It assumes `db` is a valid SQLite database connection.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_lines)]
pub unsafe extern "C" fn sqlite3_uuid_init(
    db: *mut sqlite3,
    _pz_err_msg: *mut *mut c_char,
    _p_api: *const sqlite3_api_routines,
) -> c_int {
    let flags = SQLITE_UTF8 | SQLITE_INNOCUOUS;
    let deterministic = flags | SQLITE_DETERMINISTIC;

    // --- UUIDv7 Registration ---

    let rc = unsafe {
        sqlite3_create_function_v2(
            db,
            c"uuid7".as_ptr(),
            0,
            flags,
            ptr::null_mut(),
            Some(uuid7_func),
            None,
            None,
            None,
        )
    };
    if rc != SQLITE_OK {
        return rc;
    }

    let rc = unsafe {
        sqlite3_create_function_v2(
            db,
            c"uuid7_blob".as_ptr(),
            0,
            flags,
            ptr::null_mut(),
            Some(uuid7_blob_func),
            None,
            None,
            None,
        )
    };
    if rc != SQLITE_OK {
        return rc;
    }

    let rc = unsafe {
        sqlite3_create_function_v2(
            db,
            c"uuid7_blob".as_ptr(),
            1,
            deterministic,
            ptr::null_mut(),
            Some(uuid7_blob_func),
            None,
            None,
            None,
        )
    };
    if rc != SQLITE_OK {
        return rc;
    }

    // --- UUIDv4 Registration ---

    let rc = unsafe {
        sqlite3_create_function_v2(
            db,
            c"uuid".as_ptr(),
            0,
            flags,
            ptr::null_mut(),
            Some(uuid_func),
            None,
            None,
            None,
        )
    };
    if rc != SQLITE_OK {
        return rc;
    }

    let rc = unsafe {
        sqlite3_create_function_v2(
            db,
            c"uuid_str".as_ptr(),
            1,
            deterministic,
            ptr::null_mut(),
            Some(uuid_str_func),
            None,
            None,
            None,
        )
    };
    if rc != SQLITE_OK {
        return rc;
    }

    let rc = unsafe {
        sqlite3_create_function_v2(
            db,
            c"uuid_blob".as_ptr(),
            0,
            flags,
            ptr::null_mut(),
            Some(uuid_blob_func),
            None,
            None,
            None,
        )
    };
    if rc != SQLITE_OK {
        return rc;
    }

    unsafe {
        sqlite3_create_function_v2(
            db,
            c"uuid_blob".as_ptr(),
            1,
            deterministic,
            ptr::null_mut(),
            Some(uuid_blob_func),
            None,
            None,
            None,
        )
    }
}

/// Rust-friendly helper to register the extension.
///
/// # Returns
///
/// * `c_int` - Result code from registering the extension.
///
/// # Safety
///
/// This function is unsafe because it calls the unsafe `sqlite3_uuid_init`
/// function.
///
/// # Errors
///
/// * Returns `Ok(())` if the extension was registered successfully.
/// * Returns `Err(c_int)` with the SQLite error code if registration failed. Learn more about SQLite error codes [here](https://www.sqlite.org/rescode.html).
pub unsafe fn register() -> Result<(), c_int> {
    let status = unsafe { sqlite_wasm_rs::sqlite3_auto_extension(Some(sqlite3_uuid_init)) };
    if status == SQLITE_OK { Ok(()) } else { Err(status) }
}
