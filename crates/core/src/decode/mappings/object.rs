use serde::Serialize;

/// Details of a host object error subcode.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ObjectErrorDetail {
    /// The numeric subcode.
    pub code: u32,
    /// The canonical name of the subcode.
    pub name: &'static str,
    /// A developer-friendly summary of the error.
    pub summary: &'static str,
    /// The severity level of the error ("Info", "Warning", "Error", "Fatal").
    pub severity: &'static str,
}

/// The complete mapping of all Object category error subcodes from soroban-env-host.
pub const OBJECT_ERROR_MAPPINGS: &[ObjectErrorDetail] = &[
    ObjectErrorDetail {
        code: 0,
        name: "UnknownError",
        summary: "An unknown error occurred during a host object operation.",
        severity: "Error",
    },
    ObjectErrorDetail {
        code: 1,
        name: "UnknownReference",
        summary: "The referenced host object handle is invalid or does not exist.",
        severity: "Error",
    },
    ObjectErrorDetail {
        code: 2,
        name: "UnexpectedType",
        summary: "The host object type does not match the type expected by the host function.",
        severity: "Error",
    },
    ObjectErrorDetail {
        code: 3,
        name: "ExceededLimit",
        summary: "The number of active host objects has exceeded the maximum capacity of a 32-bit unsigned integer (u32::MAX).",
        severity: "Fatal",
    },
    ObjectErrorDetail {
        code: 4,
        name: "ObjectNotFound",
        summary: "The requested host object could not be found or has already been consumed.",
        severity: "Error",
    },
    ObjectErrorDetail {
        code: 5,
        name: "IndexBounds",
        summary: "Attempted to access a vector or collection element at an out-of-bounds index.",
        severity: "Error",
    },
    ObjectErrorDetail {
        code: 6,
        name: "ContractHashWrongLength",
        summary: "The contract hash or address byte array has an incorrect length (must be 32 bytes).",
        severity: "Error",
    },
];

/// Resolve an Object category subcode to its corresponding details.
pub fn resolve_object_error(code: u32) -> Option<&'static ObjectErrorDetail> {
    OBJECT_ERROR_MAPPINGS.iter().find(|m| m.code == code)
}
