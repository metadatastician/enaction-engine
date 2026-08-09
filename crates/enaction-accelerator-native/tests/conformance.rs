// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use enaction_accelerator::{
    ACCELERATOR_CONTRACT_VERSION, Backend, Determinism, ExecutionLane, FallbackPolicy,
    KernelBuffers, KernelRequest, Layout, Operation, ScalarReferenceBackend, SupportLevel,
};
use enaction_accelerator_native::ZigScalarBackend;
use serde_json::Value;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/accelerator/v1")
}

fn load(path: impl AsRef<Path>) -> Value {
    let path = path.as_ref();
    serde_json::from_str(&fs::read_to_string(path).expect("read fixture")).expect("parse fixture")
}

fn numbers(value: &Value, field: &str) -> Vec<i32> {
    value[field]
        .as_array()
        .expect("integer array")
        .iter()
        .map(|number| i32::try_from(number.as_i64().expect("integer")).expect("i32"))
        .collect()
}

fn expected(value: &Value) -> Vec<i64> {
    value["output"]
        .as_array()
        .expect("output array")
        .iter()
        .map(|number| number.as_i64().expect("i64"))
        .collect()
}

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

fn execute(
    backend: &dyn Backend,
    request: &KernelRequest<'_>,
    left: &[i32],
    right: &[i32],
    output_len: usize,
) -> Vec<i64> {
    let mut output = vec![0_i64; output_len];
    backend
        .execute(
            request,
            KernelBuffers {
                left,
                right,
                output: &mut output,
            },
        )
        .expect("fixture executes");
    output
}

#[test]
fn rust_and_zig_match_v1_canonical_fixtures_byte_for_byte() {
    let root = fixture_root();
    let rust = ScalarReferenceBackend;
    let zig = ZigScalarBackend;

    let dot = load(root.join("cases/fixed-i32-dot.json"));
    let dot_request = request(
        Operation::FixedI32Dot,
        Layout::Dot {
            len: usize::try_from(dot["layout"]["len"].as_u64().expect("len")).expect("usize"),
        },
    );
    let dot_left = numbers(&dot, "left");
    let dot_right = numbers(&dot, "right");
    let rust_dot = execute(&rust, &dot_request, &dot_left, &dot_right, 1);
    let zig_dot = execute(&zig, &dot_request, &dot_left, &dot_right, 1);
    assert_eq!(zig_dot, rust_dot);
    assert_eq!(
        zig_dot,
        expected(&load(root.join("expected/fixed-i32-dot.json")))
    );
    assert_eq!(
        zig_dot[0].to_le_bytes(),
        rust_dot[0].to_le_bytes(),
        "canonical serialized scalar bytes differ"
    );

    let matrix = load(root.join("cases/fixed-i32-matmul.json"));
    let dimension = |name: &str| {
        usize::try_from(matrix["layout"][name].as_u64().expect("dimension")).expect("usize")
    };
    let matrix_request = request(
        Operation::FixedI32MatMul,
        Layout::MatMul {
            m: dimension("m"),
            k: dimension("k"),
            n: dimension("n"),
        },
    );
    let matrix_left = numbers(&matrix, "left");
    let matrix_right = numbers(&matrix, "right");
    let rust_matrix = execute(&rust, &matrix_request, &matrix_left, &matrix_right, 4);
    let zig_matrix = execute(&zig, &matrix_request, &matrix_left, &matrix_right, 4);
    assert_eq!(zig_matrix, rust_matrix);
    assert_eq!(
        zig_matrix,
        expected(&load(root.join("expected/fixed-i32-matmul.json")))
    );
    assert_eq!(
        zig_matrix
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>(),
        rust_matrix
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>(),
        "canonical serialized matrix bytes differ"
    );
}

fn canonical_zig_bytes() -> String {
    let root = fixture_root();
    let zig = ZigScalarBackend;
    let dot = load(root.join("cases/fixed-i32-dot.json"));
    let dot_request = request(
        Operation::FixedI32Dot,
        Layout::Dot {
            len: usize::try_from(dot["layout"]["len"].as_u64().expect("len")).expect("usize"),
        },
    );
    let dot_output = execute(
        &zig,
        &dot_request,
        &numbers(&dot, "left"),
        &numbers(&dot, "right"),
        1,
    );
    let matrix = load(root.join("cases/fixed-i32-matmul.json"));
    let dimension = |name: &str| {
        usize::try_from(matrix["layout"][name].as_u64().expect("dimension")).expect("usize")
    };
    let matrix_output = execute(
        &zig,
        &request(
            Operation::FixedI32MatMul,
            Layout::MatMul {
                m: dimension("m"),
                k: dimension("k"),
                n: dimension("n"),
            },
        ),
        &numbers(&matrix, "left"),
        &numbers(&matrix, "right"),
        4,
    );
    dot_output
        .iter()
        .chain(&matrix_output)
        .flat_map(|value| value.to_le_bytes())
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn fresh_process_worker_emits_canonical_bytes() {
    if std::env::var_os("ENACTION_ACCELERATOR_FRESH_PROCESS_WORKER").is_some() {
        println!("ENACTION_CANONICAL={}", canonical_zig_bytes());
    }
}

#[test]
fn zig_canonical_bytes_repeat_across_fresh_processes() {
    let executable = std::env::current_exe().expect("current test executable");
    let mut observations = Vec::new();
    for _ in 0..3 {
        let child = Command::new(&executable)
            .args([
                "--exact",
                "fresh_process_worker_emits_canonical_bytes",
                "--nocapture",
            ])
            .env("ENACTION_ACCELERATOR_FRESH_PROCESS_WORKER", "1")
            .output()
            .expect("launch fresh conformance process");
        assert!(
            child.status.success(),
            "fresh process failed: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        let stdout = String::from_utf8(child.stdout).expect("UTF-8 test output");
        let observation = stdout
            .lines()
            .find_map(|line| line.strip_prefix("ENACTION_CANONICAL="))
            .expect("canonical byte marker")
            .to_owned();
        observations.push(observation);
    }
    assert!(observations.windows(2).all(|pair| pair[0] == pair[1]));
    assert_eq!(observations[0], canonical_zig_bytes());
}
