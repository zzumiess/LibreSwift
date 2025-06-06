/* -*- Mode: rust; rust-indent-offset: 4 -*- */
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![allow(non_snake_case)]

extern crate pkcs11_bindings;

use pkcs11_bindings::*;
use std::sync::atomic::{AtomicBool, Ordering};

static LOGGED_IN: AtomicBool = AtomicBool::new(false);

/// This gets called to initialize the module. For this implementation, there
/// is nothing to initialize.
extern "C" fn C_Initialize(_pInitArgs: CK_VOID_PTR) -> CK_RV {
    CKR_OK
}

extern "C" fn C_Finalize(_pReserved: CK_VOID_PTR) -> CK_RV {
    CKR_OK
}

// The specification mandates that these strings be padded with spaces to the appropriate length.
// Since the length of fixed-size arrays in rust is part of the type, the compiler enforces that
// these byte strings are of the correct length.
const MANUFACTURER_ID_BYTES: &[u8; 32] = b"Test PKCS11 Manufacturer ID     ";
const LIBRARY_DESCRIPTION_BYTES: &[u8; 32] = b"Test PKCS11 Library             ";

/// This gets called to gather some information about the module. In particular, this implementation
/// supports (portions of) cryptoki (PKCS #11) version 2.2.
extern "C" fn C_GetInfo(pInfo: CK_INFO_PTR) -> CK_RV {
    if pInfo.is_null() {
        return CKR_ARGUMENTS_BAD;
    }
    let info = CK_INFO {
        cryptokiVersion: CK_VERSION { major: 2, minor: 2 },
        manufacturerID: *MANUFACTURER_ID_BYTES,
        flags: 0,
        libraryDescription: *LIBRARY_DESCRIPTION_BYTES,
        libraryVersion: CK_VERSION { major: 0, minor: 0 },
    };
    unsafe {
        *pInfo = info;
    }
    CKR_OK
}

extern "C" fn C_GetSlotList(
    tokenPresent: CK_BBOOL,
    pSlotList: CK_SLOT_ID_PTR,
    pulCount: CK_ULONG_PTR,
) -> CK_RV {
    // There are 3 slots total, but slots 1 and 3 have no token present.
    let slot_count: usize = if tokenPresent == CK_TRUE { 1 } else { 3 };
    if pulCount.is_null() {
        return CKR_ARGUMENTS_BAD;
    }
    if !pSlotList.is_null() {
        if unsafe { *pulCount } < slot_count.try_into().unwrap() {
            return CKR_BUFFER_TOO_SMALL;
        }
        let slot_list = unsafe { std::slice::from_raw_parts_mut(pSlotList, slot_count) };
        if tokenPresent == CK_TRUE {
            // The token with ID 2 is always present.
            slot_list[0] = 2;
        } else {
            slot_list[0] = 1;
            slot_list[1] = 2;
            slot_list[2] = 3;
        }
    }
    unsafe {
        *pulCount = slot_count.try_into().unwrap();
    }
    CKR_OK
}

const SLOT_DESCRIPTIONS_BYTES: [&'static [u8; 64]; 3] = [
    b"Test PKCS11 Slot                                                ",
    // \xE4\xBA\x8C is the utf-8 encoding of '二' (2)
    b"Test PKCS11 Slot \xE4\xBA\x8C                                            ",
    b"Empty PKCS11 Slot                                               ",
];

extern "C" fn C_GetSlotInfo(slotID: CK_SLOT_ID, pInfo: CK_SLOT_INFO_PTR) -> CK_RV {
    let Ok(slot_id): Result<usize, _> = slotID.try_into() else {
        return CKR_ARGUMENTS_BAD;
    };
    if slot_id <= 0 || slot_id > SLOT_DESCRIPTIONS_BYTES.len() || pInfo.is_null() {
        return CKR_ARGUMENTS_BAD;
    }
    // Only slot 2 has a token present.
    let slot_info = CK_SLOT_INFO {
        slotDescription: *SLOT_DESCRIPTIONS_BYTES[slot_id - 1],
        manufacturerID: *MANUFACTURER_ID_BYTES,
        flags: if slot_id == 2 { CKF_TOKEN_PRESENT } else { 0 } | CKF_REMOVABLE_DEVICE,
        hardwareVersion: CK_VERSION::default(),
        firmwareVersion: CK_VERSION::default(),
    };
    unsafe {
        *pInfo = slot_info;
    }
    CKR_OK
}

const TOKEN_LABELS_BYTES: [&'static [u8; 32]; 3] = [
    // \xC3\xB1 is the utf-8 encoding of 'ñ'
    b"Test PKCS11 Toke\xC3\xB1 Label        ",
    b"Test PKCS11 Toke\xC3\xB1 2 Label      ",
    b"Test PKCS11 Toke\xC3\xB1 3 Label      ",
];
const TOKEN_MODEL_BYTES: &[u8; 16] = b"Test Model      ";

extern "C" fn C_GetTokenInfo(slotID: CK_SLOT_ID, pInfo: CK_TOKEN_INFO_PTR) -> CK_RV {
    let Ok(slot_id): Result<usize, _> = slotID.try_into() else {
        return CKR_ARGUMENTS_BAD;
    };
    if slot_id <= 0 || slot_id > TOKEN_LABELS_BYTES.len() || pInfo.is_null() {
        return CKR_ARGUMENTS_BAD;
    }
    let token_info = CK_TOKEN_INFO {
        label: *TOKEN_LABELS_BYTES[slot_id - 1],
        manufacturerID: *MANUFACTURER_ID_BYTES,
        model: *TOKEN_MODEL_BYTES,
        serialNumber: [0; 16],
        // Token 2 has a protected authentication path and requires login.
        flags: if slot_id == 2 {
            CKF_PROTECTED_AUTHENTICATION_PATH | CKF_USER_PIN_INITIALIZED | CKF_LOGIN_REQUIRED
        } else {
            0
        } | CKF_TOKEN_INITIALIZED,
        ulMaxSessionCount: CK_ULONG::MAX,
        ulSessionCount: 0,
        ulMaxRwSessionCount: CK_ULONG::MAX,
        ulRwSessionCount: 0,
        ulMaxPinLen: CK_ULONG::MAX,
        ulMinPinLen: 0,
        ulTotalPublicMemory: CK_ULONG::MAX,
        ulFreePublicMemory: CK_ULONG::MAX,
        ulTotalPrivateMemory: CK_ULONG::MAX,
        ulFreePrivateMemory: CK_ULONG::MAX,
        hardwareVersion: CK_VERSION::default(),
        firmwareVersion: CK_VERSION::default(),
        utcTime: [0; 16],
    };
    unsafe {
        *pInfo = token_info;
    }
    CKR_OK
}

extern "C" fn C_GetMechanismList(
    _slotID: CK_SLOT_ID,
    _pMechanismList: CK_MECHANISM_TYPE_PTR,
    pulCount: CK_ULONG_PTR,
) -> CK_RV {
    if pulCount.is_null() {
        return CKR_ARGUMENTS_BAD;
    }
    unsafe {
        *pulCount = 0;
    }
    CKR_OK
}

extern "C" fn C_GetMechanismInfo(
    _slotID: CK_SLOT_ID,
    _type: CK_MECHANISM_TYPE,
    _pInfo: CK_MECHANISM_INFO_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_InitToken(
    _slotID: CK_SLOT_ID,
    _pPin: CK_UTF8CHAR_PTR,
    _ulPinLen: CK_ULONG,
    _pLabel: CK_UTF8CHAR_PTR,
) -> CK_RV {
    CKR_OK
}

extern "C" fn C_InitPIN(
    _hSession: CK_SESSION_HANDLE,
    _pPin: CK_UTF8CHAR_PTR,
    _ulPinLen: CK_ULONG,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_SetPIN(
    _hSession: CK_SESSION_HANDLE,
    _pOldPin: CK_UTF8CHAR_PTR,
    _ulOldLen: CK_ULONG,
    _pNewPin: CK_UTF8CHAR_PTR,
    _ulNewLen: CK_ULONG,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_OpenSession(
    slotID: CK_SLOT_ID,
    _flags: CK_FLAGS,
    _pApplication: CK_VOID_PTR,
    _Notify: CK_NOTIFY,
    phSession: CK_SESSION_HANDLE_PTR,
) -> CK_RV {
    if phSession.is_null() {
        return CKR_ARGUMENTS_BAD;
    }
    // Be "clever" and differentiate sessions based on the slotID.
    unsafe { *phSession = slotID };
    CKR_OK
}

extern "C" fn C_CloseSession(_hSession: CK_SESSION_HANDLE) -> CK_RV {
    CKR_OK
}

extern "C" fn C_CloseAllSessions(_slotID: CK_SLOT_ID) -> CK_RV {
    CKR_OK
}

// The CKS_* definitions aren't exposed by pkcs11-bindings currently.
const CKS_RO_PUBLIC_FUNCTIONS: CK_STATE = 0;
const CKS_RO_USER_FUNCTIONS: CK_STATE = 1;

extern "C" fn C_GetSessionInfo(hSession: CK_SESSION_HANDLE, pInfo: CK_SESSION_INFO_PTR) -> CK_RV {
    if pInfo.is_null() {
        return CKR_ARGUMENTS_BAD;
    }

    let info = CK_SESSION_INFO {
        // For now, slotID <=> hSession
        slotID: hSession,
        // If slot 2 has been logged in to, session 2 is a "user" session.
        state: if hSession == 2 && LOGGED_IN.load(Ordering::Acquire) {
            CKS_RO_USER_FUNCTIONS
        } else {
            CKS_RO_PUBLIC_FUNCTIONS
        },
        flags: 0,
        ulDeviceError: 0,
    };
    unsafe {
        *pInfo = info;
    }
    CKR_OK
}

extern "C" fn C_GetOperationState(
    _hSession: CK_SESSION_HANDLE,
    _pOperationState: CK_BYTE_PTR,
    _pulOperationStateLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_SetOperationState(
    _hSession: CK_SESSION_HANDLE,
    _pOperationState: CK_BYTE_PTR,
    _ulOperationStateLen: CK_ULONG,
    _hEncryptionKey: CK_OBJECT_HANDLE,
    _hAuthenticationKey: CK_OBJECT_HANDLE,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_Login(
    _hSession: CK_SESSION_HANDLE,
    _userType: CK_USER_TYPE,
    _pPin: CK_UTF8CHAR_PTR,
    _ulPinLen: CK_ULONG,
) -> CK_RV {
    LOGGED_IN.store(true, Ordering::Release);
    CKR_OK
}

extern "C" fn C_Logout(_hSession: CK_SESSION_HANDLE) -> CK_RV {
    LOGGED_IN.store(false, Ordering::Release);
    CKR_OK
}

extern "C" fn C_CreateObject(
    _hSession: CK_SESSION_HANDLE,
    _pTemplate: CK_ATTRIBUTE_PTR,
    _ulCount: CK_ULONG,
    _phObject: CK_OBJECT_HANDLE_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_CopyObject(
    _hSession: CK_SESSION_HANDLE,
    _hObject: CK_OBJECT_HANDLE,
    _pTemplate: CK_ATTRIBUTE_PTR,
    _ulCount: CK_ULONG,
    _phNewObject: CK_OBJECT_HANDLE_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DestroyObject(_hSession: CK_SESSION_HANDLE, _hObject: CK_OBJECT_HANDLE) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_GetObjectSize(
    _hSession: CK_SESSION_HANDLE,
    _hObject: CK_OBJECT_HANDLE,
    _pulSize: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_GetAttributeValue(
    _hSession: CK_SESSION_HANDLE,
    _hObject: CK_OBJECT_HANDLE,
    _pTemplate: CK_ATTRIBUTE_PTR,
    _ulCount: CK_ULONG,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_SetAttributeValue(
    _hSession: CK_SESSION_HANDLE,
    _hObject: CK_OBJECT_HANDLE,
    _pTemplate: CK_ATTRIBUTE_PTR,
    _ulCount: CK_ULONG,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_FindObjectsInit(
    _hSession: CK_SESSION_HANDLE,
    _pTemplate: CK_ATTRIBUTE_PTR,
    _ulCount: CK_ULONG,
) -> CK_RV {
    CKR_OK
}

extern "C" fn C_FindObjects(
    _hSession: CK_SESSION_HANDLE,
    _phObject: CK_OBJECT_HANDLE_PTR,
    _ulMaxObjectCount: CK_ULONG,
    pulObjectCount: CK_ULONG_PTR,
) -> CK_RV {
    if pulObjectCount.is_null() {
        return CKR_ARGUMENTS_BAD;
    }
    unsafe { *pulObjectCount = 0 };
    CKR_OK
}

extern "C" fn C_FindObjectsFinal(_hSession: CK_SESSION_HANDLE) -> CK_RV {
    CKR_OK
}

extern "C" fn C_EncryptInit(
    _hSession: CK_SESSION_HANDLE,
    _pMechanism: CK_MECHANISM_PTR,
    _hKey: CK_OBJECT_HANDLE,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_Encrypt(
    _hSession: CK_SESSION_HANDLE,
    _pData: CK_BYTE_PTR,
    _ulDataLen: CK_ULONG,
    _pEncryptedData: CK_BYTE_PTR,
    _pulEncryptedDataLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_EncryptUpdate(
    _hSession: CK_SESSION_HANDLE,
    _pPart: CK_BYTE_PTR,
    _ulPartLen: CK_ULONG,
    _pEncryptedPart: CK_BYTE_PTR,
    _pulEncryptedPartLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_EncryptFinal(
    _hSession: CK_SESSION_HANDLE,
    _pLastEncryptedPart: CK_BYTE_PTR,
    _pulLastEncryptedPartLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DecryptInit(
    _hSession: CK_SESSION_HANDLE,
    _pMechanism: CK_MECHANISM_PTR,
    _hKey: CK_OBJECT_HANDLE,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_Decrypt(
    _hSession: CK_SESSION_HANDLE,
    _pEncryptedData: CK_BYTE_PTR,
    _ulEncryptedDataLen: CK_ULONG,
    _pData: CK_BYTE_PTR,
    _pulDataLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DecryptUpdate(
    _hSession: CK_SESSION_HANDLE,
    _pEncryptedPart: CK_BYTE_PTR,
    _ulEncryptedPartLen: CK_ULONG,
    _pPart: CK_BYTE_PTR,
    _pulPartLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DecryptFinal(
    _hSession: CK_SESSION_HANDLE,
    _pLastPart: CK_BYTE_PTR,
    _pulLastPartLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DigestInit(_hSession: CK_SESSION_HANDLE, _pMechanism: CK_MECHANISM_PTR) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_Digest(
    _hSession: CK_SESSION_HANDLE,
    _pData: CK_BYTE_PTR,
    _ulDataLen: CK_ULONG,
    _pDigest: CK_BYTE_PTR,
    _pulDigestLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DigestUpdate(
    _hSession: CK_SESSION_HANDLE,
    _pPart: CK_BYTE_PTR,
    _ulPartLen: CK_ULONG,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DigestKey(_hSession: CK_SESSION_HANDLE, _hKey: CK_OBJECT_HANDLE) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DigestFinal(
    _hSession: CK_SESSION_HANDLE,
    _pDigest: CK_BYTE_PTR,
    _pulDigestLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_SignInit(
    _hSession: CK_SESSION_HANDLE,
    _pMechanism: CK_MECHANISM_PTR,
    _hKey: CK_OBJECT_HANDLE,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_Sign(
    _hSession: CK_SESSION_HANDLE,
    _pData: CK_BYTE_PTR,
    _ulDataLen: CK_ULONG,
    _pSignature: CK_BYTE_PTR,
    _pulSignatureLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_SignUpdate(
    _hSession: CK_SESSION_HANDLE,
    _pPart: CK_BYTE_PTR,
    _ulPartLen: CK_ULONG,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_SignFinal(
    _hSession: CK_SESSION_HANDLE,
    _pSignature: CK_BYTE_PTR,
    _pulSignatureLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_SignRecoverInit(
    _hSession: CK_SESSION_HANDLE,
    _pMechanism: CK_MECHANISM_PTR,
    _hKey: CK_OBJECT_HANDLE,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_SignRecover(
    _hSession: CK_SESSION_HANDLE,
    _pData: CK_BYTE_PTR,
    _ulDataLen: CK_ULONG,
    _pSignature: CK_BYTE_PTR,
    _pulSignatureLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_VerifyInit(
    _hSession: CK_SESSION_HANDLE,
    _pMechanism: CK_MECHANISM_PTR,
    _hKey: CK_OBJECT_HANDLE,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_Verify(
    _hSession: CK_SESSION_HANDLE,
    _pData: CK_BYTE_PTR,
    _ulDataLen: CK_ULONG,
    _pSignature: CK_BYTE_PTR,
    _ulSignatureLen: CK_ULONG,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_VerifyUpdate(
    _hSession: CK_SESSION_HANDLE,
    _pPart: CK_BYTE_PTR,
    _ulPartLen: CK_ULONG,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_VerifyFinal(
    _hSession: CK_SESSION_HANDLE,
    _pSignature: CK_BYTE_PTR,
    _ulSignatureLen: CK_ULONG,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_VerifyRecoverInit(
    _hSession: CK_SESSION_HANDLE,
    _pMechanism: CK_MECHANISM_PTR,
    _hKey: CK_OBJECT_HANDLE,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_VerifyRecover(
    _hSession: CK_SESSION_HANDLE,
    _pSignature: CK_BYTE_PTR,
    _ulSignatureLen: CK_ULONG,
    _pData: CK_BYTE_PTR,
    _pulDataLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DigestEncryptUpdate(
    _hSession: CK_SESSION_HANDLE,
    _pPart: CK_BYTE_PTR,
    _ulPartLen: CK_ULONG,
    _pEncryptedPart: CK_BYTE_PTR,
    _pulEncryptedPartLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DecryptDigestUpdate(
    _hSession: CK_SESSION_HANDLE,
    _pEncryptedPart: CK_BYTE_PTR,
    _ulEncryptedPartLen: CK_ULONG,
    _pPart: CK_BYTE_PTR,
    _pulPartLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_SignEncryptUpdate(
    _hSession: CK_SESSION_HANDLE,
    _pPart: CK_BYTE_PTR,
    _ulPartLen: CK_ULONG,
    _pEncryptedPart: CK_BYTE_PTR,
    _pulEncryptedPartLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DecryptVerifyUpdate(
    _hSession: CK_SESSION_HANDLE,
    _pEncryptedPart: CK_BYTE_PTR,
    _ulEncryptedPartLen: CK_ULONG,
    _pPart: CK_BYTE_PTR,
    _pulPartLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_GenerateKey(
    _hSession: CK_SESSION_HANDLE,
    _pMechanism: CK_MECHANISM_PTR,
    _pTemplate: CK_ATTRIBUTE_PTR,
    _ulCount: CK_ULONG,
    _phKey: CK_OBJECT_HANDLE_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_GenerateKeyPair(
    _hSession: CK_SESSION_HANDLE,
    _pMechanism: CK_MECHANISM_PTR,
    _pPublicKeyTemplate: CK_ATTRIBUTE_PTR,
    _ulPublicKeyAttributeCount: CK_ULONG,
    _pPrivateKeyTemplate: CK_ATTRIBUTE_PTR,
    _ulPrivateKeyAttributeCount: CK_ULONG,
    _phPublicKey: CK_OBJECT_HANDLE_PTR,
    _phPrivateKey: CK_OBJECT_HANDLE_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_WrapKey(
    _hSession: CK_SESSION_HANDLE,
    _pMechanism: CK_MECHANISM_PTR,
    _hWrappingKey: CK_OBJECT_HANDLE,
    _hKey: CK_OBJECT_HANDLE,
    _pWrappedKey: CK_BYTE_PTR,
    _pulWrappedKeyLen: CK_ULONG_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_UnwrapKey(
    _hSession: CK_SESSION_HANDLE,
    _pMechanism: CK_MECHANISM_PTR,
    _hUnwrappingKey: CK_OBJECT_HANDLE,
    _pWrappedKey: CK_BYTE_PTR,
    _ulWrappedKeyLen: CK_ULONG,
    _pTemplate: CK_ATTRIBUTE_PTR,
    _ulAttributeCount: CK_ULONG,
    _phKey: CK_OBJECT_HANDLE_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_DeriveKey(
    _hSession: CK_SESSION_HANDLE,
    _pMechanism: CK_MECHANISM_PTR,
    _hBaseKey: CK_OBJECT_HANDLE,
    _pTemplate: CK_ATTRIBUTE_PTR,
    _ulAttributeCount: CK_ULONG,
    _phKey: CK_OBJECT_HANDLE_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_SeedRandom(
    _hSession: CK_SESSION_HANDLE,
    _pSeed: CK_BYTE_PTR,
    _ulSeedLen: CK_ULONG,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_GenerateRandom(
    _hSession: CK_SESSION_HANDLE,
    _RandomData: CK_BYTE_PTR,
    _ulRandomLen: CK_ULONG,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_GetFunctionStatus(_hSession: CK_SESSION_HANDLE) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_CancelFunction(_hSession: CK_SESSION_HANDLE) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

extern "C" fn C_WaitForSlotEvent(
    _flags: CK_FLAGS,
    _pSlot: CK_SLOT_ID_PTR,
    _pRserved: CK_VOID_PTR,
) -> CK_RV {
    CKR_FUNCTION_NOT_SUPPORTED
}

/// To be a valid PKCS #11 module, this list of functions must be supported. At least cryptoki 2.2
/// must be supported for this module to work in NSS.
static FUNCTION_LIST: CK_FUNCTION_LIST = CK_FUNCTION_LIST {
    version: CK_VERSION { major: 2, minor: 2 },
    C_Initialize: Some(C_Initialize),
    C_Finalize: Some(C_Finalize),
    C_GetInfo: Some(C_GetInfo),
    C_GetFunctionList: None,
    C_GetSlotList: Some(C_GetSlotList),
    C_GetSlotInfo: Some(C_GetSlotInfo),
    C_GetTokenInfo: Some(C_GetTokenInfo),
    C_GetMechanismList: Some(C_GetMechanismList),
    C_GetMechanismInfo: Some(C_GetMechanismInfo),
    C_InitToken: Some(C_InitToken),
    C_InitPIN: Some(C_InitPIN),
    C_SetPIN: Some(C_SetPIN),
    C_OpenSession: Some(C_OpenSession),
    C_CloseSession: Some(C_CloseSession),
    C_CloseAllSessions: Some(C_CloseAllSessions),
    C_GetSessionInfo: Some(C_GetSessionInfo),
    C_GetOperationState: Some(C_GetOperationState),
    C_SetOperationState: Some(C_SetOperationState),
    C_Login: Some(C_Login),
    C_Logout: Some(C_Logout),
    C_CreateObject: Some(C_CreateObject),
    C_CopyObject: Some(C_CopyObject),
    C_DestroyObject: Some(C_DestroyObject),
    C_GetObjectSize: Some(C_GetObjectSize),
    C_GetAttributeValue: Some(C_GetAttributeValue),
    C_SetAttributeValue: Some(C_SetAttributeValue),
    C_FindObjectsInit: Some(C_FindObjectsInit),
    C_FindObjects: Some(C_FindObjects),
    C_FindObjectsFinal: Some(C_FindObjectsFinal),
    C_EncryptInit: Some(C_EncryptInit),
    C_Encrypt: Some(C_Encrypt),
    C_EncryptUpdate: Some(C_EncryptUpdate),
    C_EncryptFinal: Some(C_EncryptFinal),
    C_DecryptInit: Some(C_DecryptInit),
    C_Decrypt: Some(C_Decrypt),
    C_DecryptUpdate: Some(C_DecryptUpdate),
    C_DecryptFinal: Some(C_DecryptFinal),
    C_DigestInit: Some(C_DigestInit),
    C_Digest: Some(C_Digest),
    C_DigestUpdate: Some(C_DigestUpdate),
    C_DigestKey: Some(C_DigestKey),
    C_DigestFinal: Some(C_DigestFinal),
    C_SignInit: Some(C_SignInit),
    C_Sign: Some(C_Sign),
    C_SignUpdate: Some(C_SignUpdate),
    C_SignFinal: Some(C_SignFinal),
    C_SignRecoverInit: Some(C_SignRecoverInit),
    C_SignRecover: Some(C_SignRecover),
    C_VerifyInit: Some(C_VerifyInit),
    C_Verify: Some(C_Verify),
    C_VerifyUpdate: Some(C_VerifyUpdate),
    C_VerifyFinal: Some(C_VerifyFinal),
    C_VerifyRecoverInit: Some(C_VerifyRecoverInit),
    C_VerifyRecover: Some(C_VerifyRecover),
    C_DigestEncryptUpdate: Some(C_DigestEncryptUpdate),
    C_DecryptDigestUpdate: Some(C_DecryptDigestUpdate),
    C_SignEncryptUpdate: Some(C_SignEncryptUpdate),
    C_DecryptVerifyUpdate: Some(C_DecryptVerifyUpdate),
    C_GenerateKey: Some(C_GenerateKey),
    C_GenerateKeyPair: Some(C_GenerateKeyPair),
    C_WrapKey: Some(C_WrapKey),
    C_UnwrapKey: Some(C_UnwrapKey),
    C_DeriveKey: Some(C_DeriveKey),
    C_SeedRandom: Some(C_SeedRandom),
    C_GenerateRandom: Some(C_GenerateRandom),
    C_GetFunctionStatus: Some(C_GetFunctionStatus),
    C_CancelFunction: Some(C_CancelFunction),
    C_WaitForSlotEvent: Some(C_WaitForSlotEvent),
};

/// # Safety
///
/// This is the only function this module exposes. NSS calls it to obtain the list of functions
/// comprising this module.
/// ppFunctionList must be a valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn PKCS11TestModule_GetFunctionList(
    ppFunctionList: CK_FUNCTION_LIST_PTR_PTR,
) -> CK_RV {
    if ppFunctionList.is_null() {
        return CKR_ARGUMENTS_BAD;
    }
    // CK_FUNCTION_LIST_PTR is a *mut CK_FUNCTION_LIST, but as per the
    // specification, the caller must treat it as *const CK_FUNCTION_LIST.
    unsafe { *ppFunctionList = std::ptr::addr_of!(FUNCTION_LIST) as CK_FUNCTION_LIST_PTR };
    CKR_OK
}
