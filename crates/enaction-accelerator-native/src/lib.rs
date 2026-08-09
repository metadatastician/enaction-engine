// SPDX-License-Identifier: AGPL-3.0-or-later
//! Safe Rust adapter for the Idris2-defined, pure-Zig accelerator ABI.
//!
//! All `unsafe` is confined to the four generated calls in this crate. The
//! operation contract, planner, and scalar oracle remain in the safe
//! `enaction-accelerator` crate.

#![deny(unsafe_op_in_unsafe_fn)]

use enaction_accelerator::{
    ACCELERATOR_CONTRACT_VERSION, AcceleratorError, Backend, BackendDescriptor, Capability,
    ContractVersion, Determinism, DeviceClass, ExecutionEvidence, ExecutionLane, KernelBuffers,
    KernelRequest, Layout, Operation, SupportLevel,
};

#[allow(dead_code)]
mod ffi {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../src/interface/generated/abi/enaction_accelerator.rs"
    ));
}

const ZIG_CAPABILITIES: [Capability; 2] = [
    Capability {
        operation: Operation::FixedI32Dot,
        version: ACCELERATOR_CONTRACT_VERSION,
        support: SupportLevel::Resilient,
        determinism: Determinism::CanonicalExact,
    },
    Capability {
        operation: Operation::FixedI32MatMul,
        version: ACCELERATOR_CONTRACT_VERSION,
        support: SupportLevel::Resilient,
        determinism: Determinism::CanonicalExact,
    },
];

const ZIG_DESCRIPTOR: BackendDescriptor = BackendDescriptor {
    id: "enaction.cpu.zig.scalar",
    implementation_version: ACCELERATOR_CONTRACT_VERSION,
    device_class: DeviceClass::Cpu,
    priority: 10,
    is_reference: false,
    is_remote: false,
    capabilities: &ZIG_CAPABILITIES,
};

/// Pure-Zig scalar implementation behind one audited native call boundary.
#[derive(Clone, Copy, Debug, Default)]
pub struct ZigScalarBackend;

impl Backend for ZigScalarBackend {
    fn descriptor(&self) -> &'static BackendDescriptor {
        &ZIG_DESCRIPTOR
    }

    fn execute(
        &self,
        request: &KernelRequest<'_>,
        buffers: KernelBuffers<'_>,
    ) -> Result<ExecutionEvidence, AcceleratorError> {
        let raw_request = request_to_ffi(request)?;
        let left = ffi::BufferI32 {
            data: buffers.left.as_ptr(),
            len: u64::try_from(buffers.left.len())
                .map_err(|_| AcceleratorError::DimensionOverflow)?,
        };
        let right = ffi::BufferI32 {
            data: buffers.right.as_ptr(),
            len: u64::try_from(buffers.right.len())
                .map_err(|_| AcceleratorError::DimensionOverflow)?,
        };
        let mut output = ffi::BufferI64 {
            data: buffers.output.as_mut_ptr(),
            len: u64::try_from(buffers.output.len())
                .map_err(|_| AcceleratorError::DimensionOverflow)?,
        };
        let mut evidence = ffi::Evidence::default();

        // SAFETY: all descriptors live for this call; Rust slices provide
        // valid, aligned buffers; the mutable output borrow is exclusive; the
        // Zig ABI retains no pointer. Zig validates lengths and non-aliasing
        // before constructing slices or writing output.
        let status = unsafe {
            ffi::enaction_accel_execute(&raw_request, &left, &right, &mut output, &mut evidence)
        };
        map_status(status)?;
        validate_evidence(request, evidence)?;

        Ok(ExecutionEvidence {
            backend_id: ZIG_DESCRIPTOR.id,
            backend_version: ZIG_DESCRIPTOR.implementation_version,
            operation: request.operation,
            operation_version: request.version,
            determinism: Determinism::CanonicalExact,
            support: SupportLevel::Resilient,
        })
    }
}

/// Query the ABI version exported by the linked Zig implementation.
pub fn native_abi_version() -> ContractVersion {
    // SAFETY: leaf function with no parameters, pointers, or retained state.
    let packed = unsafe { ffi::enaction_accel_abi_version() };
    ContractVersion::new((packed >> 16) as u16, packed as u16)
}

/// Read and validate every capability exported by the Zig implementation.
pub fn native_capabilities() -> Result<Vec<Capability>, AcceleratorError> {
    // SAFETY: leaf function with no parameters, pointers, or retained state.
    let count = unsafe { ffi::enaction_accel_capability_count() };
    let mut capabilities = Vec::with_capacity(count as usize);
    for index in 0..count {
        let mut raw = ffi::Capability::default();
        // SAFETY: `raw` is valid writable storage for the duration of the call.
        let status = unsafe { ffi::enaction_accel_capability_at(index, &mut raw) };
        map_status(status)?;
        if raw.abi_major != ffi::ABI_MAJOR
            || raw.abi_minor != ffi::ABI_MINOR
            || raw.operation_major != ffi::OPERATION_MAJOR
            || raw.operation_minor != ffi::OPERATION_MINOR
            || raw.backend_id != ffi::BACKEND_ZIG_SCALAR
            || raw.device_class != ffi::DEVICE_CPU
            || raw.support != ffi::SUPPORT_RESILIENT
            || raw.determinism != ffi::DETERMINISM_CANONICAL_EXACT
        {
            return Err(AcceleratorError::InvalidExecutionEvidence {
                backend: ZIG_DESCRIPTOR.id,
            });
        }
        capabilities.push(Capability {
            operation: operation_from_ffi(raw.operation)?,
            version: ContractVersion::new(raw.operation_major, raw.operation_minor),
            support: SupportLevel::Resilient,
            determinism: Determinism::CanonicalExact,
        });
    }
    Ok(capabilities)
}

fn request_to_ffi(request: &KernelRequest<'_>) -> Result<ffi::Request, AcceleratorError> {
    let (layout, dim0, dim1, dim2) = match (request.operation, request.layout) {
        (Operation::FixedI32Dot, Layout::Dot { len }) => (
            ffi::LAYOUT_DOT,
            u64::try_from(len).map_err(|_| AcceleratorError::DimensionOverflow)?,
            0,
            0,
        ),
        (Operation::FixedI32MatMul, Layout::MatMul { m, k, n }) => (
            ffi::LAYOUT_MATMUL,
            u64::try_from(m).map_err(|_| AcceleratorError::DimensionOverflow)?,
            u64::try_from(k).map_err(|_| AcceleratorError::DimensionOverflow)?,
            u64::try_from(n).map_err(|_| AcceleratorError::DimensionOverflow)?,
        ),
        _ => return Err(AcceleratorError::LayoutMismatch),
    };
    Ok(ffi::Request {
        abi_major: ffi::ABI_MAJOR,
        abi_minor: ffi::ABI_MINOR,
        operation_major: request.version.major,
        operation_minor: request.version.minor,
        operation: operation_to_ffi(request.operation),
        lane: lane_to_ffi(request.lane),
        minimum_support: support_to_ffi(request.minimum_support),
        minimum_determinism: determinism_to_ffi(request.minimum_determinism),
        layout,
        reserved: 0,
        dim0,
        dim1,
        dim2,
    })
}

fn operation_to_ffi(operation: Operation) -> u32 {
    match operation {
        Operation::FixedI32Dot => ffi::OPERATION_FIXED_I32_DOT,
        Operation::FixedI32MatMul => ffi::OPERATION_FIXED_I32_MATMUL,
    }
}

fn operation_from_ffi(operation: u32) -> Result<Operation, AcceleratorError> {
    match operation {
        ffi::OPERATION_FIXED_I32_DOT => Ok(Operation::FixedI32Dot),
        ffi::OPERATION_FIXED_I32_MATMUL => Ok(Operation::FixedI32MatMul),
        status => Err(AcceleratorError::NativeAbiFailure {
            backend: ZIG_DESCRIPTOR.id,
            status,
        }),
    }
}

fn lane_to_ffi(lane: ExecutionLane) -> u32 {
    match lane {
        ExecutionLane::Authoritative => ffi::LANE_AUTHORITATIVE,
        ExecutionLane::Advisory => ffi::LANE_ADVISORY,
        ExecutionLane::RemoteJob => ffi::LANE_REMOTE_JOB,
    }
}

fn determinism_to_ffi(determinism: Determinism) -> u32 {
    match determinism {
        Determinism::AdvisoryOnly => ffi::DETERMINISM_ADVISORY_ONLY,
        Determinism::ToleranceBounded => ffi::DETERMINISM_TOLERANCE_BOUNDED,
        Determinism::CanonicalExact => ffi::DETERMINISM_CANONICAL_EXACT,
    }
}

fn support_to_ffi(support: SupportLevel) -> u32 {
    support as u32 + ffi::SUPPORT_DECLARED
}

fn map_status(status: u32) -> Result<(), AcceleratorError> {
    match status {
        ffi::STATUS_OK => Ok(()),
        ffi::STATUS_LAYOUT_MISMATCH => Err(AcceleratorError::LayoutMismatch),
        ffi::STATUS_DIMENSION_OVERFLOW => Err(AcceleratorError::DimensionOverflow),
        ffi::STATUS_ARITHMETIC_OVERFLOW => Err(AcceleratorError::ArithmeticOverflow),
        status => Err(AcceleratorError::NativeAbiFailure {
            backend: ZIG_DESCRIPTOR.id,
            status,
        }),
    }
}

fn validate_evidence(
    request: &KernelRequest<'_>,
    evidence: ffi::Evidence,
) -> Result<(), AcceleratorError> {
    if evidence.abi_major == ffi::ABI_MAJOR
        && evidence.abi_minor == ffi::ABI_MINOR
        && evidence.operation_major == request.version.major
        && evidence.operation_minor == request.version.minor
        && evidence.operation == operation_to_ffi(request.operation)
        && evidence.backend_id == ffi::BACKEND_ZIG_SCALAR
        && evidence.support == ffi::SUPPORT_RESILIENT
        && evidence.determinism == ffi::DETERMINISM_CANONICAL_EXACT
    {
        Ok(())
    } else {
        Err(AcceleratorError::InvalidExecutionEvidence {
            backend: ZIG_DESCRIPTOR.id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use enaction_accelerator::{FallbackPolicy, Registry};

    fn request(operation: Operation, layout: Layout) -> KernelRequest<'static> {
        KernelRequest {
            operation,
            version: ACCELERATOR_CONTRACT_VERSION,
            layout,
            lane: ExecutionLane::Authoritative,
            minimum_support: SupportLevel::Conformant,
            minimum_determinism: Determinism::CanonicalExact,
            fallback: FallbackPolicy::PreferAccelerated,
            named_backend: None,
        }
    }

    #[test]
    fn generated_layouts_match_idris2_proofs() {
        assert_eq!(core::mem::size_of::<ffi::Request>(), 56);
        assert_eq!(core::mem::align_of::<ffi::Request>(), 8);
        assert_eq!(core::mem::size_of::<ffi::BufferI32>(), 16);
        assert_eq!(core::mem::size_of::<ffi::BufferI64>(), 16);
        assert_eq!(core::mem::size_of::<ffi::Capability>(), 32);
        assert_eq!(core::mem::size_of::<ffi::Evidence>(), 24);
    }

    #[test]
    fn zig_backend_executes_through_the_safe_registry() {
        let backend = ZigScalarBackend;
        let mut registry = Registry::new();
        registry.register(&backend).unwrap();
        let request = request(Operation::FixedI32Dot, Layout::Dot { len: 3 });
        let planned = registry.plan(&request).unwrap();
        let mut output = [0];
        let evidence = planned
            .execute(
                &request,
                KernelBuffers {
                    left: &[2, -3, 4],
                    right: &[5, 7, -2],
                    output: &mut output,
                },
            )
            .unwrap();
        assert_eq!(output, [-19]);
        assert_eq!(evidence.backend_id, ZIG_DESCRIPTOR.id);
    }

    #[test]
    fn native_capability_evidence_is_self_consistent() {
        assert_eq!(native_abi_version(), ACCELERATOR_CONTRACT_VERSION);
        assert_eq!(native_capabilities().unwrap(), ZIG_CAPABILITIES);
    }

    #[test]
    fn zig_overflow_is_failure_atomic() {
        let backend = ZigScalarBackend;
        let request = request(
            Operation::FixedI32MatMul,
            Layout::MatMul { m: 1, k: 3, n: 2 },
        );
        let mut output = [71, 72];
        assert_eq!(
            backend.execute(
                &request,
                KernelBuffers {
                    left: &[i32::MAX; 3],
                    right: &[1, i32::MAX, 1, i32::MAX, 1, i32::MAX],
                    output: &mut output,
                },
            ),
            Err(AcceleratorError::ArithmeticOverflow)
        );
        assert_eq!(output, [71, 72]);
    }
}
