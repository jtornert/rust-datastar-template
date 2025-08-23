#[test]
fn ui() {
    let harness = trybuild::TestCases::new();

    harness.pass("tests/ui/smoke.rs");
    harness.compile_fail("tests/ui/non_str_lit.rs");
    harness.compile_fail("tests/ui/missing_template_attr.rs");
    harness.compile_fail("tests/ui/missing_path.rs");
    harness.compile_fail("tests/ui/no_args.rs");
}