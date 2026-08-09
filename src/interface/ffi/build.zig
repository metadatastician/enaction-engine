// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//! Pure-Zig FFI build: real library and real tests, with the Idris2-rendered
//! ABI declarations injected as a module.

const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const abi_module = b.addModule("enaction_accelerator_abi", .{
        .root_source_file = b.path("../generated/abi/enaction_accelerator.zig"),
        .target = target,
        .optimize = optimize,
    });
    const accelerator_module = b.addModule("enaction_accelerator", .{
        .root_source_file = b.path("src/accelerator.zig"),
        .target = target,
        .optimize = optimize,
    });
    accelerator_module.addImport("abi", abi_module);

    const library = b.addLibrary(.{
        .name = "enaction_accelerator",
        .root_module = accelerator_module,
    });
    b.installArtifact(library);

    const generic_module = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    const generic_tests = b.addTest(.{ .root_module = generic_module });
    const accelerator_tests = b.addTest(.{ .root_module = accelerator_module });
    const integration_module = b.createModule(.{
        .root_source_file = b.path("test/integration_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    const integration_tests = b.addTest(.{ .root_module = integration_module });

    const run_generic = b.addRunArtifact(generic_tests);
    const run_accelerator = b.addRunArtifact(accelerator_tests);
    const run_integration = b.addRunArtifact(integration_tests);
    const test_step = b.step("test", "Run generic, accelerator and integration FFI tests");
    test_step.dependOn(&run_generic.step);
    test_step.dependOn(&run_accelerator.step);
    test_step.dependOn(&run_integration.step);
}
